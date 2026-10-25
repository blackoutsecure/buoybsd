// SPDX-License-Identifier: Apache-2.0

#[path = "mod.rs"]
mod support;

use super::{Destination, Webhook};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use support::*;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

fn openssl() -> PathBuf {
    if let Some(path) = std::env::var_os("BUOYBSD_TEST_OPENSSL") {
        return path.into();
    }
    #[cfg(windows)]
    for path in [
        r"C:\Program Files\Git\mingw64\bin\openssl.exe",
        r"C:\Program Files\Git\usr\bin\openssl.exe",
    ] {
        if std::path::Path::new(path).is_file() {
            return path.into();
        }
    }
    "openssl".into()
}

fn run_openssl(directory: &std::path::Path, args: &[&str]) {
    let mut process = Command::new(openssl())
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("TLS regression tests require the OpenSSL CLI; see CONTRIBUTING.md");
    let start = Instant::now();
    loop {
        if let Some(status) = process.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if start.elapsed() >= Duration::from_secs(15) {
            process.kill().unwrap();
            let _ = process.wait();
            panic!("owned OpenSSL fixture process exceeded its 15-second deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[tokio::test]
async fn untrusted_tls_is_rejected_and_an_explicit_test_ca_succeeds() {
    let directory = TestDir::new();
    run_openssl(
        directory.path(),
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost,IP:127.0.0.1",
        ],
    );
    run_openssl(
        directory.path(),
        &[
            "pkcs12",
            "-export",
            "-inkey",
            "key.pem",
            "-in",
            "cert.pem",
            "-out",
            "identity.pfx",
            "-password",
            "pass:fixture-only",
        ],
    );
    let identity = native_tls::Identity::from_pkcs12(
        &std::fs::read(directory.join("identity.pfx")).unwrap(),
        "fixture-only",
    )
    .unwrap();
    let mut builder = native_tls::TlsAcceptor::builder(identity);
    builder.min_protocol_version(Some(native_tls::Protocol::Tlsv12));
    let acceptor = tokio_native_tls::TlsAcceptor::from(builder.build().unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break; };
                    let acceptor = acceptor.clone();
                    requests.spawn(async move {
                        let Ok(mut stream) = acceptor.accept(stream).await else { return; };
                        if read_request(&mut stream).await.is_ok() {
                            let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                            let _ = stream.shutdown().await;
                        }
                    });
                }
                _ = requests.join_next(), if !requests.is_empty() => {}
            }
        }
    });
    let mut destination = Webhook::new(webhook(format!("{url}/fixture-secret"))).unwrap();
    let error = destination
        .write_event(&CancellationToken::new(), &event())
        .await
        .unwrap_err()
        .to_string();
    assert!(!error.contains(&url) && !error.contains("fixture-secret"));
    let root = reqwest::Certificate::from_pem(&std::fs::read(directory.join("cert.pem")).unwrap())
        .unwrap();
    destination.trust_test_certificate(root);
    let result = destination
        .write_event(&CancellationToken::new(), &event())
        .await;
    destination.close().await.unwrap();
    task.abort();
    let _ = task.await;
    result.expect("explicitly trusted ephemeral test CA must succeed");
}
