// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{cli, config::*, diagnostics::Diagnostics, domain, pipeline::Stats, wire};
use support::*;
use tokio_util::sync::CancellationToken;

fn diagnostics(buffer: &Buffer) -> Stats {
    let entry: serde_json::Value = serde_json::from_slice(&buffer.bytes()).unwrap();
    serde_json::from_value(entry["metrics"].clone()).unwrap()
}

#[tokio::test]
async fn replay_json_separate_diagnostics_and_original_go_golden() {
    let stdout = Buffer::default();
    let stderr = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--replay", fixture().to_str().unwrap()]),
        Box::new(stdout.clone()),
        &Diagnostics::new(Box::new(stderr.clone())),
    )
    .await
    .unwrap();
    let expected = include_str!("fixtures/process_exec.expected.ndjson");
    let output = stdout.text();
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    for (line, golden) in lines.into_iter().zip(expected.lines()) {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(line).unwrap(),
            serde_json::from_str::<serde_json::Value>(golden).unwrap()
        );
        let event: wire::api::Event = serde_json::from_str(line).unwrap();
        assert_eq!(event.source.as_ref().unwrap().collector, "replay");
        assert_eq!(event.source.unwrap().evidence, "synthetic");
        assert!(event.raw_record.is_some());
    }
    let stats = diagnostics(&stderr);
    assert_eq!((stats.events_delivered, stats.events_dropped), (2, 0));
    assert!(!stderr.text().contains("fixture-host") && !stderr.text().contains("/bin/sh"));
}

#[tokio::test]
async fn explicit_overrides_preserve_configured_outputs_and_file_append() {
    let dir = TestDir::new();
    let path = dir.join("events.ndjson");
    std::fs::write(&path, b"{}\n").unwrap();
    let mut c = valid_config();
    c.outputs[0].format = "table".into();
    c.outputs.push(DestinationConfig {
        name: "file".into(),
        kind: "file".into(),
        format: "json".into(),
        path: path.to_str().unwrap().into(),
        ..DestinationConfig::default()
    });
    let config_path = dir.config(&c);
    let first = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--config", &config_path]),
        Box::new(first.clone()),
        &Diagnostics::new(Box::new(std::io::sink())),
    )
    .await
    .unwrap();
    assert!(first.text().starts_with("TIME\tEVENT\tPID\tEXECUTABLE\n"));
    let second = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--config", &config_path, "--output", "json"]),
        Box::new(second.clone()),
        &Diagnostics::new(Box::new(std::io::sink())),
    )
    .await
    .unwrap();
    assert!(second.text().starts_with('{'));
    let file = std::fs::read_to_string(&path).unwrap();
    assert!(file.starts_with("{}\n"));
    assert_eq!(file.lines().count(), 5);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let new = dir.join("new.ndjson");
        c.outputs = vec![DestinationConfig {
            name: "new".into(),
            kind: "file".into(),
            format: "json".into(),
            path: new.to_str().unwrap().into(),
            ..DestinationConfig::default()
        }];
        let config_path = dir.config(&c);
        cli::run_sensor(
            &CancellationToken::new(),
            &args(&["--config", &config_path]),
            Box::new(std::io::sink()),
            &Diagnostics::new(Box::new(std::io::sink())),
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::metadata(new).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn override_adds_stdout_without_discarding_file_and_replay_override_is_explicit() {
    let dir = TestDir::new();
    let file = dir.join("events.ndjson");
    let mut c = valid_config();
    c.collector.path = dir.join("missing").to_str().unwrap().into();
    c.outputs = vec![DestinationConfig {
        name: "file".into(),
        kind: "file".into(),
        format: "json".into(),
        path: file.to_str().unwrap().into(),
        ..DestinationConfig::default()
    }];
    let config = dir.config(&c);
    let stdout = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&[
            "--config",
            &config,
            "--replay",
            fixture().to_str().unwrap(),
            "--output",
            "table",
        ]),
        Box::new(stdout.clone()),
        &Diagnostics::new(Box::new(std::io::sink())),
    )
    .await
    .unwrap();
    assert!(stdout.text().starts_with("TIME\tEVENT"));
    assert_eq!(std::fs::read_to_string(file).unwrap().lines().count(), 2);
}

#[tokio::test]
async fn unsafe_and_unsupported_invocations_emit_no_events_or_input_mutations() {
    let dir = TestDir::new();
    let missing = dir.join("missing");
    for args in [
        vec![],
        args(&[
            "--replay",
            fixture().to_str().unwrap(),
            "--output",
            "forward",
        ]),
        args(&["--replay", missing.to_str().unwrap()]),
        args(&["--unknown"]),
        args(&["anything"]),
        args(&["--config", dir.path().to_str().unwrap()]),
    ] {
        let stdout = Buffer::default();
        assert!(
            cli::run_sensor(
                &CancellationToken::new(),
                &args,
                Box::new(stdout.clone()),
                &Diagnostics::new(Box::new(std::io::sink()))
            )
            .await
            .is_err()
        );
        assert!(stdout.bytes().is_empty());
    }
    let mut c = valid_config();
    c.outputs = vec![DestinationConfig {
        name: "alias".into(),
        kind: "file".into(),
        format: "json".into(),
        path: c.collector.path.clone(),
        ..DestinationConfig::default()
    }];
    let before = std::fs::read(fixture()).unwrap();
    assert!(
        cli::run_sensor(
            &CancellationToken::new(),
            &args(&["--config", &dir.config(&c)]),
            Box::new(std::io::sink()),
            &Diagnostics::new(Box::new(std::io::sink()))
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(fixture()).unwrap(), before);
}

#[tokio::test]
async fn output_failure_and_cancelled_run_are_visible() {
    let stderr = Buffer::default();
    let error = cli::run_sensor(
        &CancellationToken::new(),
        &args(&["--replay", fixture().to_str().unwrap()]),
        Box::new(FailedWriter),
        &Diagnostics::new(Box::new(stderr.clone())),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("fixture write failure"));
    assert_eq!(diagnostics(&stderr).destination_errors, 1);
    let token = CancellationToken::new();
    token.cancel();
    assert_cancelled(
        cli::run_sensor(
            &token,
            &args(&["--replay", fixture().to_str().unwrap()]),
            Box::new(std::io::sink()),
            &Diagnostics::new(Box::new(std::io::sink())),
        )
        .await,
    );
}

#[tokio::test]
async fn versions_help_legacy_flags_and_truthful_control_catalogue() {
    for flag in [
        "--version",
        "-version",
        "--version=true",
        "--version=1",
        "--version=T",
    ] {
        let stdout = Buffer::default();
        cli::run_sensor(
            &CancellationToken::new(),
            &args(&[flag]),
            Box::new(stdout.clone()),
            &Diagnostics::new(Box::new(std::io::sink())),
        )
        .await
        .unwrap();
        assert!(stdout.text().contains(domain::VERSION));
        assert!(stdout.text().contains("replay prototype"));
    }
    let stdout = Buffer::default();
    let stderr = Buffer::default();
    cli::run_sensor(
        &CancellationToken::new(),
        &args(&["-h"]),
        Box::new(stdout.clone()),
        &Diagnostics::new(Box::new(stderr.clone())),
    )
    .await
    .unwrap();
    assert!(stdout.bytes().is_empty());
    assert!(stderr.text().contains("replay"));
    for args in [args(&["version"]), args(&["--version"])] {
        let mut buffer = Buffer::default();
        cli::run_control(&args, &mut buffer).unwrap();
        assert_eq!(buffer.text(), format!("buoyctl {}\n", domain::VERSION));
    }
    let mut buffer = Buffer::default();
    cli::run_control(&args(&["events", "list"]), &mut buffer).unwrap();
    let list: serde_json::Value = serde_json::from_slice(&buffer.bytes()).unwrap();
    assert_eq!(list[0]["id"], 1_000_001);
    assert_eq!(list[0]["name"], "process_exec");
    assert_eq!(list[0]["collector"], "replay");
    assert_eq!(list.as_array().unwrap().len(), 1);
    for args in [
        args(&["status"]),
        vec![],
        args(&["events"]),
        args(&["unknown"]),
    ] {
        assert!(cli::run_control(&args, &mut std::io::sink()).is_err());
    }
}
