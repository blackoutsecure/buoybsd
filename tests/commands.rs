// SPDX-License-Identifier: Apache-2.0

mod support;
use std::{
    io::Read,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use support::*;

fn execute(binary: &str, args: &[&str]) -> Output {
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    fn read(reader: impl Read) -> Vec<u8> {
        let mut bytes = vec![];
        reader.take(2 << 20).read_to_end(&mut bytes).unwrap();
        bytes
    }
    let stdout = std::thread::spawn(move || read(stdout));
    let stderr = std::thread::spawn(move || read(stderr));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() >= Duration::from_secs(15) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("owned CLI process exceeded its 15-second deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

#[test]
fn installed_commands_keep_names_versions_events_and_failure_exit_statuses() {
    let sensor = env!("CARGO_BIN_EXE_buoybsd");
    let control = env!("CARGO_BIN_EXE_buoyctl");
    let version = execute(sensor, &["--version"]);
    assert!(version.status.success());
    assert!(version.stderr.is_empty());
    assert!(
        String::from_utf8(version.stdout)
            .unwrap()
            .starts_with("buoybsd 0.1.0-dev (")
    );
    let version = execute(control, &["version"]);
    assert!(version.status.success());
    assert_eq!(version.stdout, b"buoyctl 0.1.0-dev\n");
    let catalogue = execute(control, &["events", "list"]);
    assert!(catalogue.status.success());
    let catalogue: serde_json::Value = serde_json::from_slice(&catalogue.stdout).unwrap();
    assert_eq!(catalogue[0]["collector"], "replay");
    assert_eq!(catalogue[0]["id"], 1_000_001);
    let failed = execute(control, &["status"]);
    assert!(!failed.status.success() && failed.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&failed.stderr).unwrap();
    assert_eq!(error["component"], "cli");
    assert_eq!(error["level"], "ERROR");
    assert_eq!(
        error["error"],
        "status unavailable: no live collector or control service is implemented"
    );
    let failed = execute(sensor, &[]);
    assert!(!failed.status.success() && failed.stdout.is_empty());
    let help = execute(sensor, &["--help"]);
    assert!(help.status.success() && help.stdout.is_empty());
    assert!(String::from_utf8(help.stderr).unwrap().contains("replay"));
}

#[test]
fn executable_replay_json_table_legacy_flags_and_last_explicit_override() {
    let sensor = env!("CARGO_BIN_EXE_buoybsd");
    let output = execute(sensor, &["-replay", fixture().to_str().unwrap()]);
    assert!(output.status.success());
    let actual = String::from_utf8(output.stdout).unwrap();
    for (line, expected) in actual
        .lines()
        .zip(include_str!("fixtures/process_exec.expected.ndjson").lines())
    {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(line).unwrap(),
            serde_json::from_str::<serde_json::Value>(expected).unwrap()
        );
    }
    assert_eq!(actual.lines().count(), 2);
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["metrics"]["events_delivered"], 2);
    let table = execute(
        sensor,
        &[
            "--config",
            "configs/buoybsd.example.yaml",
            "--output",
            "json",
            "--output",
            "table",
        ],
    );
    assert!(table.status.success());
    assert_eq!(
        String::from_utf8(table.stdout).unwrap(),
        concat!(
            "TIME\tEVENT\tPID\tEXECUTABLE\n",
            "2026-01-01T00:00:00.123456789Z\t\"process_exec\"\t4242\t\"/bin/sh\"\n",
            "2026-01-01T00:00:01Z\t\"process_exec\"\t4243\t\"/usr/bin/true\"\n",
        )
    );
}

#[test]
fn executable_parser_failures_and_secret_safe_configuration_errors() {
    let sensor = env!("CARGO_BIN_EXE_buoybsd");
    let directory = TestDir::new();
    let replay = directory.join("bad.ndjson");
    std::fs::write(&replay, b"{}\n").unwrap();
    let result = execute(sensor, &["--replay", replay.to_str().unwrap()]);
    assert!(!result.status.success() && result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).unwrap();
    let diagnostic: serde_json::Value =
        serde_json::from_str(stderr.lines().next().unwrap()).unwrap();
    assert_eq!(diagnostic["metrics"]["parser_errors"], 1);
    assert_eq!(diagnostic["metrics"]["events_received"], 1);
    let config = directory.join("bad.yaml");
    std::fs::write(
        &config,
        "outputs:\n- headers:\n    Authorization: [fixture-secret]\n",
    )
    .unwrap();
    let result = execute(sensor, &["--config", config.to_str().unwrap()]);
    assert!(!result.status.success() && result.stdout.is_empty());
    assert!(
        !String::from_utf8(result.stderr)
            .unwrap()
            .contains("fixture-secret")
    );
}
