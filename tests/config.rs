// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{config::*, domain::Policy};
use support::*;

#[test]
fn strict_yaml_roundtrip_and_secret_safe_failures() {
    let mut c = valid_config();
    c.outputs.push(webhook("https://example.invalid/events"));
    let yaml = serde_yaml::to_string(&c).unwrap();
    let loaded = Config::load(yaml.as_bytes()).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded.outputs[1].timeout, ConfigDuration(1_000_000_000));
    for yaml in [
        "unknown: true\n",
        "collector:\n  unknown: true\n",
        "version: 1\nversion: 2\n",
        "version: 1\n---\nversion: 1\n",
        "outputs:\n  - headers:\n      Authorization: [fixture-secret]\n",
        "outputs:\n  - timeout: fixture-secret\n",
        "outputs: [fixture-secret\n",
        "",
        " \n# empty document\n",
        "outputs:\n  - headers:\n      Authorization: a\n      Authorization: b\n",
    ] {
        let error = Config::load(yaml.as_bytes()).unwrap_err().to_string();
        assert!(!error.contains("fixture-secret"), "{error}");
    }
    assert!(Config::load(" ".repeat(MAX_CONFIG_BYTES + 1).as_bytes()).is_err());
    let c = Config::load(b"collector:\n  path: synthetic.ndjson\n" as &[u8]).unwrap();
    assert_eq!(c.collector.max_record_bytes, 65536);
    assert_eq!(c.collector.kind, "replay");
}

#[test]
fn original_configuration_bounds_and_contradictions() {
    let modifications: Vec<fn(&mut Config)> = vec![
        |c| c.version = 2,
        |c| c.collector.kind = "audit".into(),
        |c| c.collector.path.clear(),
        |c| c.collector.max_record_bytes = 255,
        |c| c.collector.max_record_bytes = 1_048_577,
        |c| c.pipeline.queue_size = 0,
        |c| c.pipeline.queue_size = 4097,
        |c| {
            c.pipeline.queue_size = 65;
            c.collector.max_record_bytes = 1 << 20;
        },
        |c| c.detectors = vec!["rego".into()],
        |c| c.detectors = vec!["shell-exec".into(); 2],
        |c| {
            c.policies = vec![
                Policy {
                    name: "same".into(),
                    ..Policy::default()
                };
                2
            ]
        },
        |c| {
            c.policies = vec![Policy {
                name: "bad".into(),
                event_names: vec!["process_exit".into()],
                ..Policy::default()
            }]
        },
        |c| {
            c.policies = vec![Policy {
                name: "bad".into(),
                executables: vec!["[".into()],
                ..Policy::default()
            }]
        },
        |c| c.outputs.clear(),
        |c| c.outputs[0].kind = "forward".into(),
        |c| c.outputs.push(c.outputs[0].clone()),
        |c| {
            c.outputs.push(DestinationConfig {
                name: "other".into(),
                kind: "stdout".into(),
                format: "table".into(),
                ..DestinationConfig::default()
            })
        },
    ];
    for modify in modifications {
        let mut c = valid_config();
        modify(&mut c);
        assert!(c.validate().is_err());
    }
    let mut c = valid_config();
    c.collector.max_record_bytes = 1 << 20;
    c.pipeline.queue_size = 64;
    c.validate().unwrap();
    c.collector.max_record_bytes = 256;
    c.pipeline.queue_size = 4096;
    c.validate().unwrap();
}

#[test]
fn original_destination_validation_bounds_and_contradictions() {
    let modifications: Vec<fn(&mut DestinationConfig)> = vec![
        |d| d.url = "https://user:fixture-secret@example.invalid".into(),
        |d| d.url.push_str("#fixture-secret"),
        |d| d.url = "/events".into(),
        |d| d.url = "https://%".into(),
        |d| d.url = "https://example.invalid:65536/events".into(),
        |d| {
            d.url = "http://127.0.0.1/events".into();
            d.allow_http = false;
        },
        |d| d.url = "http://example.invalid/events".into(),
        |d| d.timeout = ConfigDuration(0),
        |d| d.timeout = ConfigDuration(31_000_000_000),
        |d| d.timeout = ConfigDuration(1_000),
        |d| d.retries = -1,
        |d| d.retries = 6,
        |d| {
            d.headers
                .insert("Authorization".into(), "fixture-secret\r\nx: y".into());
        },
        |d| {
            d.headers.insert("bad name".into(), "fixture-secret".into());
        },
        |d| {
            d.headers
                .insert("Content-Type".into(), "fixture-secret".into());
        },
        |d| {
            d.headers
                .insert("Authorization".into(), "fixture-secret".into());
            d.headers.insert("authorization".into(), "other".into());
        },
        |d| d.format = "table".into(),
        |d| d.path = "events.ndjson".into(),
    ];
    for modify in modifications {
        let mut d = webhook("https://example.invalid/events");
        modify(&mut d);
        let error = d.validate().unwrap_err().to_string();
        assert!(!error.contains("fixture-secret"));
    }
    for url in [
        "http://localhost:8080/events",
        "http://127.0.0.1:8080/events",
        "http://[::1]:8080/events",
        "http://[::ffff:127.0.0.1]:8080/events",
        "HTTP://LOCALHOST:8080/events",
    ] {
        webhook(url).validate().unwrap();
    }
    webhook("HTTPS://example.invalid/events")
        .validate()
        .unwrap();
    for url in [
        "http://2130706433/events",
        "http://0x7f000001/events",
        "http://localhost./events",
    ] {
        assert!(webhook(url).validate().is_err(), "{url}");
    }
    let mut d = webhook("https://example.invalid/events");
    d.retries = 5;
    d.timeout = ConfigDuration(1_000_000);
    d.validate().unwrap();
    for d in [
        DestinationConfig {
            name: "file".into(),
            kind: "file".into(),
            format: "table".into(),
            path: "events".into(),
            ..DestinationConfig::default()
        },
        DestinationConfig {
            name: "stdout".into(),
            kind: "stdout".into(),
            format: "json".into(),
            path: "events".into(),
            ..DestinationConfig::default()
        },
        DestinationConfig {
            name: "file".into(),
            kind: "file".into(),
            format: "json".into(),
            path: "events".into(),
            url: "https://example.invalid".into(),
            ..DestinationConfig::default()
        },
    ] {
        assert!(d.validate().is_err());
    }
}

#[test]
fn path_validation_rejects_regular_symlink_hardlink_and_configuration_aliases() {
    let dir = TestDir::new();
    let input = dir.join("input.ndjson");
    std::fs::write(&input, b"synthetic").unwrap();
    let mut c = valid_config();
    c.collector.path = input.to_str().unwrap().into();
    c.outputs = vec![DestinationConfig {
        name: "file".into(),
        kind: "file".into(),
        format: "json".into(),
        path: c.collector.path.clone(),
        ..DestinationConfig::default()
    }];
    assert!(c.validate_paths("").is_err());
    c.outputs[0].path = dir.join("new-output.ndjson").to_str().unwrap().into();
    c.validate_paths("").unwrap();
    let alias = dir.join("alias.ndjson");
    std::fs::hard_link(&input, &alias).unwrap();
    c.outputs[0].path = alias.to_str().unwrap().into();
    assert!(c.validate_paths("").is_err());
    c.outputs[0].path = dir.path().to_str().unwrap().into();
    assert!(c.validate_paths("").is_err());
    c.outputs[0].path = dir.join("new-output.ndjson").to_str().unwrap().into();
    assert!(c.validate_paths(dir.path().to_str().unwrap()).is_err());
    std::fs::write(&c.outputs[0].path, b"configuration").unwrap();
    assert!(c.validate_paths(&c.outputs[0].path).is_err());
    c.collector.path = dir.path().to_str().unwrap().into();
    assert!(c.validate_paths("").is_err());
    #[cfg(unix)]
    {
        c.collector.path = input.to_str().unwrap().into();
        let link = dir.join("symlink.ndjson");
        std::os::unix::fs::symlink(&input, &link).unwrap();
        c.outputs[0].path = link.to_str().unwrap().into();
        assert!(c.validate_paths("").is_err());
    }
}

#[test]
fn go_duration_grammar_and_typed_units_remain_supported() {
    for (text, nanos) in [
        ("1s", 1_000_000_000),
        ("1m0.005s", 60_005_000_000),
        (".5ms", 500_000),
        ("1µs", 1000),
        ("-1h2m", -3_720_000_000_000),
        ("0", 0),
    ] {
        assert_eq!(ConfigDuration::parse(text).unwrap(), ConfigDuration(nanos));
    }

    for text in [
        "",
        "100",
        "1d",
        "1..0s",
        "1s fixture-secret",
        "999999999999999999999h",
    ] {
        assert!(ConfigDuration::parse(text).is_err());
    }
}

#[test]
fn yaml_null_defaults_and_merge_keys_retain_existing_semantics() {
    let yaml = b"version: null\ncollector:\n  type: null\n  path: synthetic.ndjson\n  max_record_bytes: null\npipeline:\n  queue_size: null\ndetectors: null\npolicies: null\noutputs:\n- name: stdout\n  type: stdout\n  format: json\n  headers: null\n  timeout: null\n";
    let config = Config::load(yaml.as_slice()).unwrap();
    config.validate().unwrap();
    assert_eq!(config.collector.max_record_bytes, 65536);
    assert_eq!(config.pipeline.queue_size, 64);
    assert!(config.detectors.is_empty());
    let config =
        Config::load(b"collector:\n  <<: {type: replay, path: synthetic.ndjson}\n" as &[u8])
            .unwrap();
    config.validate().unwrap();
    assert_eq!(config.collector.path, "synthetic.ndjson");
}
