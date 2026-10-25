// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{
    config::ConfigDuration,
    output::{Destination, DestinationStats, Webhook},
    wire,
};
use std::time::{Duration, Instant};
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn json_headers_retries_and_failure_counters() {
    let server = HttpServer::start(|_, n| Response::status(if n == 1 { 503 } else { 204 })).await;
    let mut spec = webhook(&server.url);
    spec.retries = 1;
    spec.headers
        .insert("Authorization".into(), "fixture-token".into());
    let mut d = Webhook::new(spec).unwrap();
    d.write_event(&CancellationToken::new(), &event())
        .await
        .unwrap();
    assert_eq!(
        d.stats(),
        DestinationStats {
            attempts: 2,
            errors: 1
        }
    );
    for request in server.requests.lock().unwrap().iter() {
        assert_eq!(request.method, "POST");
        assert_eq!(request.headers["content-type"], "application/json");
        assert_eq!(request.headers["authorization"], "fixture-token");
        assert_eq!(request.headers["x-buoybsd-record-id"], event().record_id);
        assert_eq!(
            serde_json::from_slice::<wire::api::Event>(&request.body).unwrap(),
            wire::to_wire(&event())
        );
    }
    d.close().await.unwrap();
}

#[tokio::test]
async fn retry_budget_status_classification_and_response_bound() {
    for status in [400, 429, 500] {
        let server = HttpServer::start(move |_, _| Response::status(status)).await;
        let mut spec = webhook(&server.url);
        spec.retries = 2;
        let mut d = Webhook::new(spec).unwrap();
        assert!(
            d.write_event(&CancellationToken::new(), &event())
                .await
                .is_err()
        );
        let expected = if status == 400 { 1 } else { 3 };
        assert_eq!(server.requests.lock().unwrap().len(), expected as usize);
        assert_eq!(
            d.stats(),
            DestinationStats {
                attempts: expected,
                errors: expected
            }
        );
        d.close().await.unwrap();
    }
    let server = HttpServer::start(|_, _| {
        let mut r = Response::status(204);
        r.body = vec![b'x'; 128 << 10];
        r
    })
    .await;
    let mut d = Webhook::new(webhook(&server.url)).unwrap();
    d.write_event(&CancellationToken::new(), &event())
        .await
        .unwrap();
    d.close().await.unwrap();
}

#[tokio::test]
async fn redirects_are_not_followed_and_endpoint_header_secrets_are_not_echoed() {
    let target = HttpServer::start(|_, _| Response::status(204)).await;
    let location = format!("{}/fixture-secret", target.url);
    let redirect = HttpServer::start(move |_, _| {
        let mut r = Response::status(307);
        r.headers.push(("Location".into(), location.clone()));
        r
    })
    .await;
    let mut spec = webhook(format!(
        "{}/fixture-secret?token=fixture-token",
        redirect.url
    ));
    spec.headers
        .insert("Authorization".into(), "fixture-authorization".into());
    let mut d = Webhook::new(spec).unwrap();
    let error = d
        .write_event(&CancellationToken::new(), &event())
        .await
        .unwrap_err()
        .to_string();
    assert!(target.requests.lock().unwrap().is_empty());
    assert!(!error.contains("fixture-") && !error.contains(&redirect.url));
    assert!(error.contains("HTTP 307"));
    d.close().await.unwrap();
}

#[tokio::test]
async fn timeout_and_cancelled_backoff_are_bounded() {
    let slow = HttpServer::start(|_, _| {
        let mut r = Response::status(204);
        r.delay = Duration::from_secs(1);
        r
    })
    .await;
    let mut spec = webhook(&slow.url);
    spec.timeout = ConfigDuration(5_000_000);
    let mut d = Webhook::new(spec).unwrap();
    let start = Instant::now();
    assert!(
        d.write_event(&CancellationToken::new(), &event())
            .await
            .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    d.close().await.unwrap();

    let server = HttpServer::start(|_, _| Response::status(503)).await;
    let mut spec = webhook(&server.url);
    spec.retries = 5;
    let mut d = Webhook::new(spec).unwrap();
    let token = CancellationToken::new();
    let child = token.clone();
    let task = tokio::spawn(async move {
        let result = d.write_event(&child, &event()).await;
        (d, result)
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while server.requests.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    token.cancel();
    let (mut d, result) = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap();
    assert_cancelled(result);
    assert_eq!(d.stats().attempts, 1);
    d.close().await.unwrap();
}

#[tokio::test]
async fn pre_cancelled_webhook_sends_nothing_and_transport_errors_are_redacted() {
    let server = HttpServer::start(|_, _| Response::status(204)).await;
    let mut d = Webhook::new(webhook(&server.url)).unwrap();
    let token = CancellationToken::new();
    token.cancel();
    assert_cancelled(d.write_event(&token, &event()).await);
    assert_eq!(d.stats().attempts, 0);
    assert!(server.requests.lock().unwrap().is_empty());
    d.close().await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut d = Webhook::new(webhook(format!(
        "http://{address}/fixture-secret?token=fixture-token"
    )))
    .unwrap();
    let error = d
        .write_event(&CancellationToken::new(), &event())
        .await
        .unwrap_err()
        .to_string();
    assert!(!error.contains("fixture-") && !error.contains(&address.to_string()));
    assert_eq!(
        d.stats(),
        DestinationStats {
            attempts: 1,
            errors: 1
        }
    );
    d.close().await.unwrap();
}
