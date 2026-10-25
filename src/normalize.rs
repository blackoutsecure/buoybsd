// SPDX-License-Identifier: Apache-2.0

use crate::{
    domain::*,
    error::{Error, Result},
};
use sha2::{Digest, Sha256};

pub fn normalize(raw: &RawEvent) -> Result<Event> {
    if raw.name != PROCESS_EXEC {
        return Err(Error::message("normalization: unsupported event"));
    }
    let timestamp = raw
        .timestamp
        .ok_or_else(|| Error::message("normalization: source timestamp is required"))?;
    timestamp
        .validate()
        .map_err(|_| Error::message("normalization: timestamp is outside protobuf range"))?;
    if raw.record.is_empty() || raw.record.len() > MAX_RECORD_BYTES {
        return Err(Error::message(
            "normalization: raw record is missing or exceeds its limit",
        ));
    }
    if raw.collector.is_empty()
        || raw.operating_system.is_empty()
        || raw.evidence.is_empty()
        || raw.media_type.is_empty()
    {
        return Err(Error::message(
            "normalization: source attribution is required",
        ));
    }
    let mut event = Event {
        timestamp: Some(timestamp),
        id: PROCESS_EXEC_ID,
        name: PROCESS_EXEC.into(),
        schema_version: SCHEMA_VERSION.into(),
        record_id: format!("sha256:{:x}", Sha256::digest(&raw.record)),
        source: Some(Source {
            collector: raw.collector.clone(),
            operating_system: raw.operating_system.clone(),
            evidence: raw.evidence.clone(),
            source_id: raw.source_id.clone(),
            replay_sequence: raw.sequence,
        }),
        raw_record: Some(RawRecord {
            media_type: raw.media_type.clone(),
            data: raw.record.clone(),
        }),
        host: raw.host.clone(),
        process: raw.process.clone(),
        jail: raw.jail.clone(),
        process_exec: Some(raw.exec.clone().unwrap_or_default()),
        ..Event::default()
    };
    let mut warn = |field: &str, reason: &str| {
        event.warnings.push(DataWarning {
            field: field.into(),
            reason: reason.into(),
        })
    };
    if event.process.as_ref().and_then(|p| p.pid).is_none() {
        warn("process.pid", "not observed");
    }
    if event
        .process
        .as_ref()
        .and_then(|p| p.executable_path.as_ref())
        .is_none()
    {
        warn("process.executable_path", "not observed");
    }
    let exec = event
        .process_exec
        .as_ref()
        .expect("normalizer always supplies exec");
    if exec.argv_complete != Some(true) {
        warn("process_exec.argv", "completeness not established");
    }
    if exec.success.is_none() {
        warn("process_exec.success", "outcome not observed");
    }
    Ok(event)
}
