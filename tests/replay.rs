// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{
    error::{Error, Result},
    io::{BoxFuture, Input},
    replay::{Collector, CollectorStats, Replay, parse},
};
use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use support::*;
use tokio::{
    io::{AsyncRead, ReadBuf},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn bytes_presence_order_counters_and_single_run() {
    let first = format!("{VALID}\r\n");
    let source = Replay::new(Box::new(BytesInput::new(format!("{first}{VALID}"))), 65536).unwrap();
    let (sender, mut received) = mpsc::channel(2);
    source
        .run(CancellationToken::new(), sender.clone())
        .await
        .unwrap();
    let one = received.recv().await.unwrap();
    let two = received.recv().await.unwrap();
    assert_eq!(one.record, first.as_bytes());
    assert_eq!(two.record, VALID.as_bytes());
    assert_eq!((one.sequence, two.sequence), (1, 2));
    assert_eq!(one.process.as_ref().unwrap().uid, Some(0));
    assert_eq!(one.process.as_ref().unwrap().euid, None);
    assert_eq!(one.timestamp.unwrap().nanos, 123_456_789);
    assert_eq!(
        (one.collector.as_str(), one.evidence.as_str()),
        ("replay", "synthetic")
    );
    assert_eq!(
        source.stats(),
        CollectorStats {
            received: 2,
            ..CollectorStats::default()
        }
    );
    assert!(source.run(CancellationToken::new(), sender).await.is_err());
    source.close().await.unwrap();
    source.close().await.unwrap();
}

#[tokio::test]
async fn all_original_invalid_records_are_rejected_and_counted() {
    let mut cases = vec![
        "\n".into(),
        "{".into(),
        "null".into(),
        "[]".into(),
        format!("{VALID} {{}}"),
        VALID.replacen("\"event\":", "\"event\":\"process_exec\",\"event\":", 1),
        VALID.replace("\"pid\":42", "\"pid\":42,\"pid\":43"),
        VALID.replace("\"event\":", "\"Event\":"),
        VALID.replace("\"event\":", "\"Event\":\"process_exec\",\"event\":"),
        VALID.replace("\"event\":", "\"unrecognized\":true,\"event\":"),
        VALID.replace("\"pid\":42", "\"pid\":42,\"unrecognized\":1"),
        VALID.replace("buoybsd.replay.v1", "buoybsd.replay.v2"),
        VALID.replace("process_exec", "process_exit"),
        VALID.replace("synthetic", "captured"),
        VALID.replace("freebsd", "linux"),
        VALID.replace("2026-01-01T00:00:00.123456789Z", "2026-01-01"),
        VALID.replace("2026-01-01T00:00:00.123456789Z", "2026-01-01T00:00:00"),
        VALID.replace(".123456789Z", ".1234567891Z"),
        VALID.replace(".123456789Z", ",123456789Z"),
        VALID.replace(".123456789Z", ".123456789+24:00"),
        VALID.replace(".123456789Z", ".123456789+00:60"),
        VALID.replace("2026-01-01", "0000-01-01"),
        VALID.replace("\"pid\":42", "\"pid\":-1"),
        VALID.replace("\"pid\":42", "\"pid\":4294967296"),
        VALID.replace("\"pid\":42", "\"pid\":0"),
        VALID.replace(
            "\"pid\":42",
            &format!("\"pid\":42,\"command\":\"{}\"", "x".repeat(4097)),
        ),
        VALID.replace(
            "\"success\":true",
            &format!("\"success\":true,\"argv\":[\"{}\"]", "x".repeat(4097)),
        ),
        VALID.replace(
            "\"success\":true",
            &format!("\"success\":true,\"argv\":[{}\"x\"]", "\"x\",".repeat(128)),
        ),
        format!("{}0{}", "{\"x\":".repeat(18), "}".repeat(18)),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect::<Vec<_>>();
    let mut invalid_utf8 = VALID.as_bytes().to_vec();
    invalid_utf8.insert(150, 0xff);
    cases.push(invalid_utf8);
    for input in cases {
        let source = Replay::new(Box::new(BytesInput::new(&input)), 65536).unwrap();
        let (sender, mut received) = mpsc::channel(1);
        assert!(
            source.run(CancellationToken::new(), sender).await.is_err(),
            "{input:?}"
        );
        assert!(received.try_recv().is_err());
        assert_eq!(
            source.stats(),
            CollectorStats {
                received: 1,
                parser_errors: 1,
                ..CollectorStats::default()
            }
        );
        source.close().await.unwrap();
    }
}

#[tokio::test]
async fn record_limit_includes_line_ending() {
    for extra in [0, 1] {
        let input = format!("{VALID}{}\n", " ".repeat(256 - VALID.len() - 1 + extra));
        let source = Replay::new(Box::new(BytesInput::new(input)), 256).unwrap();
        let (sender, mut events) = mpsc::channel(1);
        assert_eq!(
            source.run(CancellationToken::new(), sender).await.is_ok(),
            extra == 0
        );
        assert_eq!(events.try_recv().is_ok(), extra == 0);
        source.close().await.unwrap();
    }
    for size in [255, 1_048_577] {
        assert!(Replay::new(Box::new(BytesInput::new([])), size).is_err());
    }
}

#[tokio::test]
async fn backpressure_and_cancellation_account_for_unsent_observation() {
    let source = Arc::new(
        Replay::new(
            Box::new(BytesInput::new(format!("{VALID}\n{VALID}\n{VALID}\n"))),
            65536,
        )
        .unwrap(),
    );
    let token = CancellationToken::new();
    let (sender, mut received) = mpsc::channel(1);
    let collector = Arc::clone(&source);
    let child = token.clone();
    let task = tokio::spawn(async move { collector.run(child, sender).await });
    let _ = received.recv().await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!task.is_finished());
    assert_eq!(source.stats().received, 3);
    token.cancel();
    assert_cancelled(
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap(),
    );
    assert_eq!(source.stats().dropped, 1);
    source.close().await.unwrap();
}

struct BrokenInput(Arc<AtomicUsize>);
impl AsyncRead for BrokenInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::other("fixture IO failed")))
    }
}
impl Input for BrokenInput {
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Err(Error::message("fixture close failed")) })
    }
}

#[tokio::test]
async fn read_errors_close_errors_and_idempotent_ownership() {
    let closes = Arc::new(AtomicUsize::new(0));
    let source = Replay::new(Box::new(BrokenInput(Arc::clone(&closes))), 65536).unwrap();
    let (sender, _) = mpsc::channel(1);
    assert!(
        source
            .run(CancellationToken::new(), sender.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("fixture IO failed")
    );
    for _ in 0..2 {
        assert!(
            source
                .close()
                .await
                .unwrap_err()
                .to_string()
                .contains("fixture close failed")
        );
    }
    assert_eq!(closes.load(Ordering::Relaxed), 1);
    assert!(source.run(CancellationToken::new(), sender).await.is_err());
}

#[test]
fn exact_fields_arguments_and_year_one_are_valid() {
    for input in [
        VALID.replace(
            "\"pid\":42",
            &format!("\"pid\":42,\"command\":\"{}\"", "x".repeat(4096)),
        ),
        VALID.replace(
            "\"success\":true",
            &format!("\"success\":true,\"argv\":[{}\"x\"]", "\"x\",".repeat(127)),
        ),
        VALID.replace(
            "\"success\":true",
            &format!("\"success\":true,\"argv\":[\"{}\"]", "x".repeat(4096)),
        ),
        VALID.replace("2026-01-01T00:00:00.123456789Z", "0001-01-01T00:00:00Z"),
    ] {
        parse(input.as_bytes(), 1).unwrap();
    }

    let input = VALID.replace("\"success\":true", "\"success\":true,\"argv\":[null,\"x\"]");
    assert_eq!(
        parse(input.as_bytes(), 1).unwrap().exec.unwrap().argv,
        ["", "x"]
    );
}

#[test]
fn escaped_surrogates_keep_original_json_reader_semantics_and_raw_provenance() {
    for (escape, expected) in [
        (r"\ud800", "�"),
        (r"\udc00", "�"),
        (r"\ud83d\ude00", "😀"),
        (r"\\ud800", r"\ud800"),
    ] {
        let line = VALID.replace(
            "\"pid\":42",
            &format!("\"pid\":42,\"command\":\"{escape}\""),
        );
        let raw = parse(line.as_bytes(), 1).unwrap();
        assert_eq!(raw.process.unwrap().command.as_deref(), Some(expected));
        assert_eq!(raw.record, line.as_bytes());
    }
}
