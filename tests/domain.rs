// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{
    domain::*,
    glob::Glob,
    normalize::normalize,
    time::{MAX_SECONDS, MIN_SECONDS, Timestamp},
    wire,
};
use prost::Message;
use support::*;

#[test]
fn catalogue_namespace_and_ownership() {
    let mut d = definitions();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].name, PROCESS_EXEC);
    assert!((1_000_000..=1_999_999).contains(&d[0].id));
    assert!(known(PROCESS_EXEC));
    assert!(!known("sched_process_exec") && !known("process_exit"));
    d[0].name = "changed".into();
    assert_eq!(definitions()[0].name, PROCESS_EXEC);
}

#[test]
fn normalization_preserves_presence_source_bytes_and_ownership() {
    let mut raw = observation();
    let event = normalize(&raw).unwrap();
    assert_eq!(event.raw_record.as_ref().unwrap().data, raw.record);
    use sha2::{Digest, Sha256};
    assert_eq!(
        event.record_id,
        format!("sha256:{:x}", Sha256::digest(&raw.record))
    );
    assert_eq!(event.timestamp.unwrap().nanos, 123_456_789);
    assert_eq!(event.schema_version, SCHEMA_VERSION);
    assert_eq!(event.jail.as_ref().unwrap().id, Some(0));
    let json: serde_json::Value = serde_json::from_slice(&wire::marshal(&event).unwrap()).unwrap();
    assert_eq!(json["process"]["uid"], 0);
    assert!(json["process"].get("euid").is_none());
    assert_eq!(json["source"]["replay_sequence"], "1");
    assert_eq!(json["id"], "EVENT_ID_PROCESS_EXEC");
    raw.record[0] = b'X';
    raw.exec.as_mut().unwrap().argv[0] = "changed".into();
    raw.process.as_mut().unwrap().uid = Some(99);
    raw.source_id = Some("changed".into());
    assert_ne!(event.raw_record.as_ref().unwrap().data[0], b'X');
    assert_eq!(event.process_exec.as_ref().unwrap().argv, ["sh"]);
    assert_eq!(event.process.as_ref().unwrap().uid, Some(0));
    assert_eq!(
        event.source.as_ref().unwrap().source_id.as_deref(),
        Some("synthetic")
    );
}

#[test]
fn normalization_warns_without_fabricating() {
    let mut raw = observation();
    raw.process = None;
    raw.exec = None;
    raw.jail = None;
    raw.host = None;
    let event = normalize(&raw).unwrap();
    assert!(event.process.is_none() && event.jail.is_none() && event.host.is_none());
    assert_eq!(event.warnings.len(), 4);
    assert_eq!(event.process_exec.unwrap(), ProcessExec::default());
    let mut raw = observation();
    raw.timestamp = Some(Timestamp {
        seconds: MIN_SECONDS,
        nanos: 0,
    });
    assert_eq!(
        normalize(&raw)
            .unwrap()
            .timestamp
            .unwrap()
            .rfc3339_nano()
            .unwrap(),
        "0001-01-01T00:00:00Z"
    );
}

#[test]
fn normalization_rejects_invalid_observations() {
    let changes: Vec<fn(&mut RawEvent)> = vec![
        |r| r.name = "unknown".into(),
        |r| r.timestamp = None,
        |r| {
            r.timestamp = Some(Timestamp {
                seconds: MAX_SECONDS + 1,
                nanos: 0,
            })
        },
        |r| r.record.clear(),
        |r| r.record = vec![0; MAX_RECORD_BYTES + 1],
        |r| r.collector.clear(),
        |r| r.operating_system.clear(),
        |r| r.evidence.clear(),
        |r| r.media_type.clear(),
    ];
    for change in changes {
        let mut raw = observation();
        change(&mut raw);
        assert!(normalize(&raw).is_err());
    }
}

#[test]
fn policies_preserve_and_or_real_identity_and_missing_fields() {
    let mut event = event();
    event.process = Some(Process {
        uid: Some(0),
        euid: Some(1000),
        gid: Some(0),
        executable_path: Some("/usr/local/bin/bash".into()),
        command: Some("bash".into()),
        ..Process::default()
    });
    let cases = vec![
        (Policy::default(), true),
        (
            Policy {
                event_names: vec![PROCESS_EXEC.into()],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                collectors: vec!["replay".into()],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                uids: vec![0],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                uids: vec![1000],
                ..Policy::default()
            },
            false,
        ),
        (
            Policy {
                gids: vec![0],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                jail_ids: vec![0],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                executables: vec!["/usr/local/bin/bash".into()],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                executables: vec!["/usr/local/bin/*".into()],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                executables: vec!["/usr/*".into()],
                ..Policy::default()
            },
            false,
        ),
        (
            Policy {
                process_names: vec!["ba*".into()],
                ..Policy::default()
            },
            true,
        ),
        (
            Policy {
                uids: vec![0],
                gids: vec![1000],
                ..Policy::default()
            },
            false,
        ),
        (
            Policy {
                uids: vec![1000, 0],
                ..Policy::default()
            },
            true,
        ),
    ];
    for (mut policy, expected) in cases {
        policy.name = "test".into();
        let (matched, names) = PolicySet::compile(vec![policy]).unwrap().matches(&event);
        assert_eq!(matched, expected);
        assert_eq!(names.len(), usize::from(expected));
    }
    for policy in [
        Policy {
            uids: vec![0],
            ..Policy::default()
        },
        Policy {
            gids: vec![0],
            ..Policy::default()
        },
        Policy {
            jail_ids: vec![0],
            ..Policy::default()
        },
        Policy {
            executables: vec!["*".into()],
            ..Policy::default()
        },
        Policy {
            process_names: vec!["*".into()],
            ..Policy::default()
        },
    ] {
        let set = PolicySet::compile(vec![Policy {
            name: "missing".into(),
            ..policy
        }])
        .unwrap();
        for missing in [
            Event::default(),
            Event {
                process: Some(Process::default()),
                jail: Some(Jail::default()),
                ..Event::default()
            },
        ] {
            assert!(!set.matches(&missing).0);
        }
    }
    let set = PolicySet::compile(vec![
        Policy {
            name: "one".into(),
            ..Policy::default()
        },
        Policy {
            name: "two".into(),
            ..Policy::default()
        },
    ])
    .unwrap();
    assert_eq!(
        set.matches(&event),
        (true, vec!["one".into(), "two".into()])
    );
    assert_eq!(
        PolicySet::default().matches(&Event::default()),
        (true, vec![])
    );
    for policy in [
        Policy {
            executables: vec!["[".into()],
            ..Policy::default()
        },
        Policy {
            event_names: vec!["process_exit".into()],
            ..Policy::default()
        },
        Policy {
            collectors: vec!["audit".into()],
            ..Policy::default()
        },
    ] {
        assert!(
            PolicySet::compile(vec![Policy {
                name: "bad".into(),
                ..policy
            }])
            .is_err()
        );
    }
    assert!(
        PolicySet::compile(vec![
            Policy {
                name: "same".into(),
                ..Policy::default()
            };
            2
        ])
        .is_err()
    );
}

#[test]
fn full_path_globs_match_the_existing_go_contract() {
    for (pattern, name, expected) in [
        ("*", "", true),
        ("?", "", false),
        ("*", "a/b", false),
        ("**", "a/b", false),
        ("/usr/**", "/usr/local/bin/sh", false),
        ("ba?", "bash", false),
        ("ba?", "baé", true),
        ("[a-z]", "b", true),
        ("[^a-z]", "B", true),
        ("[z-a]", "a", false),
        (r"\*", "*", true),
        (r"[a\-c]", "-", true),
        ("[/]", "/", true),
        ("?", "/", false),
        (r"[\\]", "\\", true),
        ("[!a]", "!", true),
    ] {
        assert_eq!(
            Glob::compile(pattern).unwrap().matches(name),
            expected,
            "{pattern} {name}"
        );
    }
    for pattern in ["[", "[]", "[^]", "[a-]", "[-a]", "\\", "[a\\]"] {
        assert!(Glob::compile(pattern).is_err(), "{pattern}");
    }
}

#[test]
fn shell_detector_metadata_source_and_no_invented_matches() {
    let source = event();
    let d = ShellExec.evaluate(&source).unwrap().unwrap();
    assert_eq!(d.id, "BUOY-0001");
    assert_eq!(d.severity, "informational");
    assert_eq!(d.detected_from.unwrap().record_id, source.record_id);
    assert_eq!(ShellExec.definition().required_fields.len(), 2);
    assert_eq!(ShellExec.definition().event_selectors, [PROCESS_EXEC]);
    assert!(!ShellExec.definition().description.is_empty());
    for mutate in [
        |e: &mut Event| e.process = None,
        |e: &mut Event| e.process.as_mut().unwrap().executable_path = None,
        |e: &mut Event| e.process_exec = None,
        |e: &mut Event| e.process_exec.as_mut().unwrap().success = None,
        |e: &mut Event| e.process_exec.as_mut().unwrap().success = Some(false),
        |e: &mut Event| e.process.as_mut().unwrap().executable_path = Some("/usr/bin/true".into()),
        |e: &mut Event| e.name = "process_exit".into(),
    ] {
        let mut e = event();
        mutate(&mut e);
        assert!(ShellExec.evaluate(&e).unwrap().is_none());
    }
}

#[test]
fn protobuf_numbers_presence_and_json_roundtrip() {
    let wire = wire::to_wire(&event());
    let binary = wire.encode_to_vec();
    assert_eq!(wire::api::Event::decode(binary.as_slice()).unwrap(), wire);
    let json = serde_json::to_vec(&wire).unwrap();
    assert_eq!(
        serde_json::from_slice::<wire::api::Event>(&json).unwrap(),
        wire
    );
    let uid = wire::api::Process {
        uid: Some(0),
        ..Default::default()
    };
    assert_eq!(uid.encode_to_vec(), [0x20, 0]);
    let sequence = wire::api::Source {
        replay_sequence: 1,
        ..Default::default()
    };
    assert_eq!(sequence.encode_to_vec(), [0x28, 1]);
    let payload = wire::api::Event {
        data: Some(wire::api::event::Data::ProcessExec(
            wire::api::ProcessExec::default(),
        )),
        ..Default::default()
    };
    assert_eq!(payload.encode_to_vec(), [0xa2, 0x01, 0x00]);
}

#[test]
fn timestamps_keep_offsets_range_and_precision() {
    for (value, utc) in [
        ("0001-01-01T00:00:00Z", "0001-01-01T00:00:00Z"),
        (
            "9999-12-31T23:59:59.999999999Z",
            "9999-12-31T23:59:59.999999999Z",
        ),
        (
            "2026-01-01T01:30:00.000001+01:30",
            "2026-01-01T00:00:00.000001Z",
        ),
        ("2000-02-29T00:00:00-01:00", "2000-02-29T01:00:00Z"),
    ] {
        assert_eq!(
            Timestamp::parse(value).unwrap().rfc3339_nano().unwrap(),
            utc
        );
    }
    for value in [
        "1900-02-29T00:00:00Z",
        "0001-01-01T00:00:00+00:01",
        "9999-12-31T23:59:59-00:01",
    ] {
        assert!(Timestamp::parse(value).is_err(), "{value}");
    }
    for (nanos, fraction) in [
        (1, "000000001"),
        (1000, "000001"),
        (1_000_000, "001"),
        (100_000_000, "100"),
    ] {
        let t = Timestamp { seconds: 0, nanos };
        assert_eq!(
            t.protobuf_json().unwrap(),
            format!("1970-01-01T00:00:00.{fraction}Z")
        );
    }
}
