// SPDX-License-Identifier: Apache-2.0

use crate::{
    domain,
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, prost::Message)]
pub struct Timestamp {
    #[prost(int64, tag = "1")]
    pub seconds: i64,
    #[prost(int32, tag = "2")]
    pub nanos: i32,
}

impl Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let timestamp = crate::time::Timestamp {
            seconds: self.seconds,
            nanos: self.nanos,
        };
        let text = timestamp
            .protobuf_json()
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&text)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let timestamp = crate::time::Timestamp::parse(&text).map_err(serde::de::Error::custom)?;
        Ok(Self {
            seconds: timestamp.seconds,
            nanos: timestamp.nanos,
        })
    }
}

pub mod api {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/api/v1beta1/buoybsd.v1beta1.rs"
    ));
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/api/v1beta1/buoybsd.v1beta1.serde.rs"
    ));
}

pub fn to_wire(event: &domain::Event) -> api::Event {
    api::Event {
        timestamp: event.timestamp.map(|t| Timestamp {
            seconds: t.seconds,
            nanos: t.nanos,
        }),
        id: event.id,
        name: event.name.clone(),
        schema_version: event.schema_version.clone(),
        record_id: event.record_id.clone(),
        source: event.source.as_ref().map(|s| api::Source {
            collector: s.collector.clone(),
            operating_system: s.operating_system.clone(),
            evidence: s.evidence.clone(),
            source_id: s.source_id.clone(),
            replay_sequence: s.replay_sequence,
        }),
        host: event.host.as_ref().map(|h| api::Host {
            name: h.name.clone(),
        }),
        process: event.process.as_ref().map(|p| api::Process {
            pid: p.pid,
            tid: p.tid,
            parent_pid: p.parent_pid,
            uid: p.uid,
            euid: p.euid,
            gid: p.gid,
            egid: p.egid,
            executable_path: p.executable_path.clone(),
            command: p.command.clone(),
            ancestors: vec![],
            ancestry_complete: None,
        }),
        matched_policies: event.matched_policies.clone(),
        detections: event
            .detections
            .iter()
            .map(|d| api::Detection {
                id: d.id.clone(),
                name: d.name.clone(),
                description: d.description.clone(),
                severity: d.severity.clone(),
                tags: d.tags.clone(),
                mitre_techniques: d.mitre_techniques.clone(),
                detected_from: d.detected_from.as_ref().map(|s| api::EventReference {
                    id: s.id,
                    name: s.name.clone(),
                    record_id: s.record_id.clone(),
                }),
            })
            .collect(),
        warnings: event
            .warnings
            .iter()
            .map(|w| api::DataWarning {
                field: w.field.clone(),
                reason: w.reason.clone(),
            })
            .collect(),
        raw_record: event.raw_record.as_ref().map(|r| api::RawRecord {
            media_type: r.media_type.clone(),
            data: r.data.clone(),
        }),
        jail: event.jail.as_ref().map(|j| api::Jail {
            id: j.id,
            name: j.name.clone(),
        }),
        data: event.process_exec.as_ref().map(|e| {
            api::event::Data::ProcessExec(api::ProcessExec {
                argv: e.argv.clone(),
                argv_complete: e.argv_complete,
                success: e.success,
            })
        }),
    }
}

pub fn marshal(event: &domain::Event) -> Result<Vec<u8>> {
    if let Some(timestamp) = event.timestamp {
        timestamp.validate()?;
    }
    serde_json::to_vec(&to_wire(event))
        .map_err(|_| Error::message("cannot serialize event as protobuf JSON"))
}
