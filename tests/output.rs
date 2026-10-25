// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{
    domain::{Event, Process},
    output::{Destination, DestinationStats, Json, Table, quote_to_ascii},
    time::{MAX_SECONDS, Timestamp},
    wire::{self, api},
};
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn json_roundtrip_one_object_per_line_and_counters() {
    let buffer = Buffer::default();
    let mut destination = Json::new("json".into(), Box::new(buffer.clone()), false).unwrap();
    let token = CancellationToken::new();
    for _ in 0..2 {
        destination.write_event(&token, &event()).await.unwrap();
    }
    let text = buffer.text();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert_eq!(
            serde_json::from_str::<api::Event>(line).unwrap(),
            wire::to_wire(&event())
        );
    }
    assert_eq!(
        destination.stats(),
        DestinationStats {
            attempts: 2,
            errors: 0
        }
    );
    destination.close().await.unwrap();
}

#[tokio::test]
async fn json_write_short_write_close_and_cancellation_failures() {
    let token = CancellationToken::new();
    let writers: Vec<Box<dyn buoybsd::io::Writer>> =
        vec![Box::new(FailedWriter), Box::new(ShortWriter)];
    for writer in writers {
        let mut d = Json::new("json".into(), writer, false).unwrap();
        assert!(d.write_event(&token, &event()).await.is_err());
        assert_eq!(
            d.stats(),
            DestinationStats {
                attempts: 1,
                errors: 1
            }
        );
    }
    let mut d = Json::new("json".into(), Box::new(FailedCloser), true).unwrap();
    assert!(
        d.close()
            .await
            .unwrap_err()
            .to_string()
            .contains("fixture close failure")
    );
    token.cancel();
    assert_cancelled(d.write_event(&token, &event()).await);
    assert!(Json::new("".into(), Box::new(std::io::sink()), false).is_err());
}

#[tokio::test]
async fn tables_quote_all_terminal_data_and_emit_header_once() {
    let mut source = event();
    source.name = "exec\u{1b}[31m\n".into();
    source.process.as_mut().unwrap().executable_path =
        Some("/bin/hostile\u{1b}[2J\n\t\u{202e}".into());
    let buffer = Buffer::default();
    let mut d = Table::new("table".into(), Box::new(buffer.clone())).unwrap();
    for _ in 0..2 {
        d.write_event(&CancellationToken::new(), &source)
            .await
            .unwrap();
    }
    let text = buffer.text();
    assert!(!text.contains('\u{1b}') && !text.contains('\u{202e}'));
    assert_eq!(text.matches('\n').count(), 3);
    assert_eq!(text.matches("TIME\tEVENT").count(), 1);
    assert!(text.contains("\\x1b") && text.contains("\\n"));
    assert_eq!(
        quote_to_ascii("\0\u{7}\u{8}\u{b}\u{c}é\u{7f}🛟"),
        "\"\\x00\\a\\b\\v\\f\\u00e9\\x7f\\U0001f6df\""
    );
    d.close().await.unwrap();
}

#[tokio::test]
async fn tables_reject_bad_timestamps_write_errors_and_cancelled_calls() {
    let mut d = Table::new("table".into(), Box::new(FailedWriter)).unwrap();
    assert!(
        d.write_event(&CancellationToken::new(), &event())
            .await
            .is_err()
    );
    for e in [
        Event::default(),
        Event {
            timestamp: Some(Timestamp {
                seconds: MAX_SECONDS + 1,
                nanos: 0,
            }),
            ..Event::default()
        },
    ] {
        let mut d = Table::new("table".into(), Box::new(std::io::sink())).unwrap();
        assert!(d.write_event(&CancellationToken::new(), &e).await.is_err());
    }
    let mut d = Table::new("table".into(), Box::new(std::io::sink())).unwrap();
    let token = CancellationToken::new();
    token.cancel();
    assert_cancelled(d.write_event(&token, &event()).await);
    assert!(Table::new("".into(), Box::new(std::io::sink())).is_err());
}

#[tokio::test]
async fn table_missing_values_and_observed_empty_fields_are_distinct() {
    let buffer = Buffer::default();
    let mut d = Table::new("table".into(), Box::new(buffer.clone())).unwrap();
    let mut e = event();
    e.process = None;
    d.write_event(&CancellationToken::new(), &e).await.unwrap();
    e.process = Some(Process {
        executable_path: Some("".into()),
        ..Process::default()
    });
    d.write_event(&CancellationToken::new(), &e).await.unwrap();
    assert!(buffer.text().contains("\t-\t-\n") && buffer.text().contains("\t-\t\"\"\n"));
}
