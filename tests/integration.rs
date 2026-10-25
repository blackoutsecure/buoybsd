// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{cli, config::*, diagnostics::Diagnostics, domain::Policy, pipeline::Stats, wire};
use support::*;
use tokio_util::sync::CancellationToken;

fn metrics(buffer: &Buffer) -> Stats {
    let entry: serde_json::Value = serde_json::from_slice(&buffer.bytes()).unwrap();
    serde_json::from_value(entry["metrics"].clone()).unwrap()
}

#[tokio::test]
async fn replay_to_json_file_webhook_and_original_record_evidence() {
    let directory = TestDir::new();
    let file = directory.join("events.ndjson");
    let server = HttpServer::start(|_, _| Response::status(204)).await;
    let mut c = valid_config();
    c.pipeline.queue_size = 1;
    c.policies = vec![Policy {
        name: "exec".into(),
        event_names: vec!["process_exec".into()],
        ..Policy::default()
    }];
    c.outputs.extend([
        DestinationConfig {
            name: "file".into(),
            kind: "file".into(),
            format: "json".into(),
            path: file.to_str().unwrap().into(),
            ..DestinationConfig::default()
        },
        webhook(&server.url),
    ]);
    let stdout = Buffer::default();
    let stderr = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--config", &directory.config(&c)]),
        Box::new(stdout.clone()),
        &Diagnostics::new(Box::new(stderr.clone())),
    )
    .await
    .unwrap();
    let raw = std::fs::read(fixture()).unwrap();
    let expected: Vec<_> = raw.split_inclusive(|b| *b == b'\n').collect();
    let file = std::fs::read_to_string(file).unwrap();
    let output = stdout.text();
    assert_eq!(output.lines().count(), 2);
    assert_eq!(file.lines().count(), 2);
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for (i, (line, file_line)) in output.lines().zip(file.lines()).enumerate() {
        let event: wire::api::Event = serde_json::from_str(line).unwrap();
        assert_eq!(event.schema_version, "buoybsd.v1beta1");
        assert_eq!(event.name, "process_exec");
        assert_eq!(event.source.as_ref().unwrap().evidence, "synthetic");
        assert_eq!(event.source.as_ref().unwrap().replay_sequence, i as u64 + 1);
        assert_eq!(event.raw_record.as_ref().unwrap().data, expected[i]);
        assert_eq!(event.matched_policies, ["exec"]);
        if i == 0 {
            assert_eq!(event.process.as_ref().unwrap().uid, Some(0));
            assert_eq!(event.detections.len(), 1);
            assert_eq!(
                event.detections[0]
                    .detected_from
                    .as_ref()
                    .unwrap()
                    .record_id,
                event.record_id
            );
        }
        assert_eq!(
            serde_json::from_str::<wire::api::Event>(file_line).unwrap(),
            event
        );
        assert_eq!(
            serde_json::from_slice::<wire::api::Event>(&requests[i].body).unwrap(),
            event
        );
    }
    assert_eq!(
        metrics(&stderr),
        Stats {
            events_received: 2,
            events_normalized: 2,
            events_delivered: 2,
            detections_generated: 1,
            delivery_attempts: 6,
            ..Stats::default()
        }
    );
}

#[tokio::test]
async fn policy_table_and_permanent_delivery_failure_vertical_slices() {
    let directory = TestDir::new();
    let mut c = valid_config();
    c.policies = vec![Policy {
        name: "user".into(),
        uids: vec![1001],
        ..Policy::default()
    }];
    c.outputs[0].format = "table".into();
    let stdout = Buffer::default();
    let stderr = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--config", &directory.config(&c)]),
        Box::new(stdout.clone()),
        &Diagnostics::new(Box::new(stderr.clone())),
    )
    .await
    .unwrap();
    assert_eq!(stdout.text().lines().count(), 2);
    assert!(stdout.text().contains("/usr/bin/true") && !stdout.text().contains("/bin/sh"));
    assert_eq!(
        (
            metrics(&stderr).events_filtered,
            metrics(&stderr).events_delivered
        ),
        (1, 1)
    );
    let server = HttpServer::start(|_, _| Response::status(400)).await;
    let mut spec = webhook(&server.url);
    spec.retries = 5;
    c.outputs = vec![spec];
    c.policies.clear();
    let stderr = Buffer::default();
    assert!(
        cli::run_sensor(
            &CancellationToken::new(),
            &args(&["--config", &directory.config(&c)]),
            Box::new(std::io::sink()),
            &Diagnostics::new(Box::new(stderr.clone()))
        )
        .await
        .is_err()
    );
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    let stats = metrics(&stderr);
    assert_eq!((stats.events_delivered, stats.destination_errors), (0, 1));
    assert!(stats.events_dropped > 0);
}
