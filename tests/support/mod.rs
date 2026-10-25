// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use buoybsd::{
    config::{Config, ConfigDuration, DestinationConfig},
    domain::*,
    error::Result,
    io::{Input, Writer},
    normalize::normalize,
    time::Timestamp,
};
use std::{
    collections::BTreeMap,
    io::{self, Cursor, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};

pub const VALID: &str = r#"{"version":"buoybsd.replay.v1","event":"process_exec","timestamp":"2026-01-01T00:00:00.123456789Z","source_os":"freebsd","evidence":"synthetic","process":{"pid":42,"uid":0},"exec":{"success":true}}"#;

pub fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).into()).collect()
}
pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("process_exec.synthetic.ndjson")
}

pub fn observation() -> RawEvent {
    RawEvent {
        name: PROCESS_EXEC.into(),
        timestamp: Some(Timestamp {
            seconds: 1_700_000_000,
            nanos: 123_456_789,
        }),
        collector: "replay".into(),
        operating_system: "freebsd".into(),
        evidence: "synthetic".into(),
        source_id: Some("synthetic".into()),
        sequence: 1,
        host: Some(Host {
            name: Some("fixture-host".into()),
        }),
        process: Some(Process {
            pid: Some(42),
            uid: Some(0),
            executable_path: Some("/bin/sh".into()),
            ..Process::default()
        }),
        jail: Some(Jail {
            id: Some(0),
            name: None,
        }),
        exec: Some(ProcessExec {
            argv: vec!["sh".into()],
            argv_complete: Some(true),
            success: Some(true),
        }),
        record: b"original synthetic record\r\n".to_vec(),
        media_type: "application/x-buoybsd-replay+json".into(),
    }
}

pub fn event() -> Event {
    normalize(&observation()).unwrap()
}

pub struct BytesInput(pub Cursor<Vec<u8>>);
impl BytesInput {
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        Self(Cursor::new(bytes.as_ref().to_vec()))
    }
}
impl AsyncRead for BytesInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        b: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, b)
    }
}
impl Input for BytesInput {}

#[derive(Clone, Default)]
pub struct Buffer(pub Arc<Mutex<Vec<u8>>>);
impl Buffer {
    pub fn bytes(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
    pub fn text(&self) -> String {
        String::from_utf8(self.bytes()).unwrap()
    }
}
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Writer for Buffer {}

pub struct FailedWriter;
impl Write for FailedWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fixture write failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Writer for FailedWriter {}

pub struct ShortWriter;
impl Write for ShortWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        Ok(b.len().saturating_sub(1))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Writer for ShortWriter {}

pub struct FailedCloser;
impl Write for FailedCloser {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Writer for FailedCloser {
    fn close(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture close failure"))
    }
}

pub struct TestDir(PathBuf);
impl TestDir {
    pub fn new() -> Self {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-data")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    pub fn config(&self, config: &Config) -> String {
        let path = self.join("config.yaml");
        std::fs::write(&path, serde_yaml::to_string(config).unwrap()).unwrap();
        path.to_str().unwrap().into()
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

pub fn valid_config() -> Config {
    let mut c = Config::default();
    c.collector.path = fixture().to_str().unwrap().into();
    c
}

pub fn webhook(url: impl Into<String>) -> DestinationConfig {
    DestinationConfig {
        name: "webhook".into(),
        kind: "webhook".into(),
        format: "json".into(),
        url: url.into(),
        timeout: ConfigDuration(1_000_000_000),
        allow_http: true,
        ..DestinationConfig::default()
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub delay: std::time::Duration,
}
impl Response {
    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: vec![],
            body: vec![],
            delay: std::time::Duration::ZERO,
        }
    }
}

pub struct HttpServer {
    pub url: String,
    pub requests: Arc<Mutex<Vec<Request>>>,
    task: tokio::task::JoinHandle<()>,
}
impl HttpServer {
    pub async fn start(
        handler: impl Fn(&Request, usize) -> Response + Send + Sync + 'static,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(vec![]));
        let received = Arc::clone(&requests);
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            let mut children = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((mut stream, _)) = accepted else { break; };
                        let handler = Arc::clone(&handler);
                        let received = Arc::clone(&received);
                        children.spawn(async move {
                            let Ok(request) = read_request(&mut stream).await else { return; };
                            let count = {
                                let mut requests = received.lock().unwrap();
                                requests.push(request.clone());
                                requests.len()
                            };
                            let response = handler(&request, count);
                            tokio::time::sleep(response.delay).await;
                            let mut head = format!("HTTP/1.1 {} Result\r\nContent-Length: {}\r\nConnection: close\r\n", response.status, response.body.len());
                            for (name, value) in response.headers { head.push_str(&format!("{name}: {value}\r\n")); }
                            head.push_str("\r\n");
                            let _ = stream.write_all(head.as_bytes()).await;
                            let _ = stream.write_all(&response.body).await;
                        });
                    }
                    _ = children.join_next(), if !children.is_empty() => {}
                }
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }
}
impl Drop for HttpServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn read_request(stream: &mut (impl AsyncRead + Unpin)) -> io::Result<Request> {
    let mut head = vec![];
    loop {
        if head.len() > 65536 {
            return Err(io::Error::other("test request headers exceed bound"));
        }
        head.push(stream.read_u8().await?);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8(head).map_err(io::Error::other)?;
    let mut lines = text.split("\r\n");
    let method = lines
        .next()
        .unwrap_or("")
        .split(' ')
        .next()
        .unwrap_or("")
        .into();
    let mut headers: BTreeMap<String, String> = BTreeMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::other("invalid test header"))?;
        headers.insert(name.to_ascii_lowercase(), value.trim().into());
    }
    let size = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    if size > 2 << 20 {
        return Err(io::Error::other("test request body exceeds bound"));
    }
    let mut body = vec![0; size];
    stream.read_exact(&mut body).await?;
    Ok(Request {
        method,
        headers,
        body,
    })
}

pub fn assert_cancelled<T: std::fmt::Debug>(result: Result<T>) {
    assert!(result.unwrap_err().is_cancelled());
}
