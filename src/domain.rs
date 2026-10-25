// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::{Error, Result},
    glob::Glob,
    time::Timestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA_VERSION: &str = "buoybsd.v1beta1";
pub const REPLAY_VERSION: &str = "buoybsd.replay.v1";
pub const PROCESS_EXEC: &str = "process_exec";
pub const PROCESS_EXEC_ID: i32 = 1_000_001;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_RECORD_BYTES: usize = 256;
pub const MAX_RECORD_BYTES: usize = 1 << 20;
pub const MAX_FIELD_BYTES: usize = 4096;
pub const MAX_ARGUMENTS: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Definition {
    pub id: i32,
    pub name: String,
    pub collector: String,
    pub status: String,
}

pub fn definitions() -> Vec<Definition> {
    vec![Definition {
        id: PROCESS_EXEC_ID,
        name: PROCESS_EXEC.into(),
        collector: "replay".into(),
        status: "synthetic replay only; native telemetry unverified".into(),
    }]
}

pub fn known(name: &str) -> bool {
    name == PROCESS_EXEC
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Host {
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Process {
    pub pid: Option<u32>,
    pub tid: Option<u32>,
    #[serde(rename = "ppid")]
    pub parent_pid: Option<u32>,
    pub uid: Option<u32>,
    pub euid: Option<u32>,
    pub gid: Option<u32>,
    pub egid: Option<u32>,
    pub executable_path: Option<String>,
    pub command: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Jail {
    pub id: Option<u32>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProcessExec {
    #[serde(deserialize_with = "argv")]
    pub argv: Vec<String>,
    pub argv_complete: Option<bool>,
    pub success: Option<bool>,
}

fn argv<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<String>, D::Error> {
    let values = Option::<Vec<Option<String>>>::deserialize(d)?.unwrap_or_default();
    Ok(values.into_iter().map(Option::unwrap_or_default).collect())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RawEvent {
    pub name: String,
    pub timestamp: Option<Timestamp>,
    pub collector: String,
    pub operating_system: String,
    pub evidence: String,
    pub source_id: Option<String>,
    pub sequence: u64,
    pub host: Option<Host>,
    pub process: Option<Process>,
    pub jail: Option<Jail>,
    pub exec: Option<ProcessExec>,
    pub record: Vec<u8>,
    pub media_type: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Source {
    pub collector: String,
    pub operating_system: String,
    pub evidence: String,
    pub source_id: Option<String>,
    pub replay_sequence: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RawRecord {
    pub media_type: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DataWarning {
    pub field: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventReference {
    pub id: i32,
    pub name: String,
    pub record_id: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Detection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: String,
    pub tags: Vec<String>,
    pub mitre_techniques: Vec<String>,
    pub detected_from: Option<EventReference>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Event {
    pub timestamp: Option<Timestamp>,
    pub id: i32,
    pub name: String,
    pub schema_version: String,
    pub record_id: String,
    pub source: Option<Source>,
    pub host: Option<Host>,
    pub process: Option<Process>,
    pub matched_policies: Vec<String>,
    pub detections: Vec<Detection>,
    pub warnings: Vec<DataWarning>,
    pub raw_record: Option<RawRecord>,
    pub jail: Option<Jail>,
    pub process_exec: Option<ProcessExec>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    #[serde(deserialize_with = "null_default")]
    pub name: String,
    #[serde(deserialize_with = "null_default")]
    pub event_names: Vec<String>,
    #[serde(deserialize_with = "null_default")]
    pub collectors: Vec<String>,
    #[serde(deserialize_with = "null_default")]
    pub executables: Vec<String>,
    #[serde(deserialize_with = "null_default")]
    pub uids: Vec<u32>,
    #[serde(deserialize_with = "null_default")]
    pub gids: Vec<u32>,
    #[serde(deserialize_with = "null_default")]
    pub jail_ids: Vec<u32>,
    #[serde(deserialize_with = "null_default")]
    pub process_names: Vec<String>,
}

pub(crate) fn null_default<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

impl Policy {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(Error::message("name must contain 1-64 bytes"));
        }
        for values in [
            &self.event_names,
            &self.collectors,
            &self.executables,
            &self.process_names,
        ] {
            if values.len() > 128 {
                return Err(Error::message(
                    "at most 128 values per selector are allowed",
                ));
            }
            if values
                .iter()
                .any(|s| s.is_empty() || s.len() > MAX_FIELD_BYTES)
            {
                return Err(Error::message("selector values must contain 1-4096 bytes"));
            }
        }
        for values in [&self.uids, &self.gids, &self.jail_ids] {
            if values.len() > 128 {
                return Err(Error::message(
                    "at most 128 numeric values per selector are allowed",
                ));
            }
        }
        if self.event_names.iter().any(|name| !known(name)) {
            return Err(Error::message("unsupported event selector"));
        }
        if self.collectors.iter().any(|name| name != "replay") {
            return Err(Error::message("unsupported collector selector"));
        }
        for pattern in self.executables.iter().chain(&self.process_names) {
            Glob::compile(pattern)?;
        }
        Ok(())
    }
}

pub fn validate_policies(policies: &[Policy]) -> Result<()> {
    if policies.len() > 64 {
        return Err(Error::message("at most 64 policies are allowed"));
    }
    let mut names = HashSet::new();
    for (i, policy) in policies.iter().enumerate() {
        policy
            .validate()
            .map_err(|e| e.context(format!("policy {}", i + 1)))?;
        if !names.insert(&policy.name) {
            return Err(Error::message("policy names must be unique"));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CompiledPolicy {
    policy: Policy,
    executables: Vec<Glob>,
    process_names: Vec<Glob>,
}

/// Validated policies are ORed; an absent credential never matches observed UID zero.
///
/// ```
/// use buoybsd::domain::{Event, Policy, PolicySet, Process};
/// let policies = PolicySet::compile(vec![Policy {
///     name: "root".into(), uids: vec![0], ..Policy::default()
/// }]).unwrap();
/// assert!(!policies.matches(&Event::default()).0);
/// let observed = Event {
///     process: Some(Process { uid: Some(0), ..Process::default() }),
///     ..Event::default()
/// };
/// assert!(policies.matches(&observed).0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct PolicySet(Vec<CompiledPolicy>);

impl PolicySet {
    pub fn compile(policies: Vec<Policy>) -> Result<Self> {
        validate_policies(&policies)?;
        let policies = policies
            .into_iter()
            .map(|policy| {
                Ok(CompiledPolicy {
                    executables: policy
                        .executables
                        .iter()
                        .map(|s| Glob::compile(s))
                        .collect::<Result<_>>()?,
                    process_names: policy
                        .process_names
                        .iter()
                        .map(|s| Glob::compile(s))
                        .collect::<Result<_>>()?,
                    policy,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self(policies))
    }

    pub fn matches(&self, event: &Event) -> (bool, Vec<String>) {
        let names: Vec<String> = self
            .0
            .iter()
            .filter(|p| p.matches(event))
            .map(|p| p.policy.name.clone())
            .collect();
        (self.0.is_empty() || !names.is_empty(), names)
    }
}

impl CompiledPolicy {
    fn matches(&self, event: &Event) -> bool {
        let p = &self.policy;
        if !p.event_names.is_empty() && !p.event_names.contains(&event.name) {
            return false;
        }
        let collector = event
            .source
            .as_ref()
            .map(|s| s.collector.as_str())
            .unwrap_or("");
        if !p.collectors.is_empty() && !p.collectors.iter().any(|s| s == collector) {
            return false;
        }
        let process = event.process.as_ref();
        numeric(&p.uids, process.and_then(|p| p.uid))
            && numeric(&p.gids, process.and_then(|p| p.gid))
            && glob(
                &self.executables,
                process.and_then(|p| p.executable_path.as_deref()),
            )
            && glob(
                &self.process_names,
                process.and_then(|p| p.command.as_deref()),
            )
            && numeric(&p.jail_ids, event.jail.as_ref().and_then(|j| j.id))
    }
}

fn numeric(allowed: &[u32], observed: Option<u32>) -> bool {
    allowed.is_empty() || observed.is_some_and(|n| allowed.contains(&n))
}

fn glob(patterns: &[Glob], observed: Option<&str>) -> bool {
    patterns.is_empty() || observed.is_some_and(|s| patterns.iter().any(|p| p.matches(s)))
}

#[derive(Clone, Debug)]
pub struct DetectorDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub event_selectors: &'static [&'static str],
    pub severity: &'static str,
    pub tags: &'static [&'static str],
    pub required_fields: &'static [&'static str],
}

pub trait Detector: Send + Sync {
    fn definition(&self) -> DetectorDefinition;
    fn evaluate(&self, source: &Event) -> Result<Option<Detection>>;
}

pub struct ShellExec;

impl Detector for ShellExec {
    fn definition(&self) -> DetectorDefinition {
        DetectorDefinition {
            id: "BUOY-0001",
            name: "Shell execution",
            description: "A known shell executable was observed executing successfully; not proof of malicious activity.",
            event_selectors: &[PROCESS_EXEC],
            severity: "informational",
            tags: &["execution", "shell"],
            required_fields: &["process.executable_path", "process_exec.success"],
        }
    }

    fn evaluate(&self, source: &Event) -> Result<Option<Detection>> {
        if source.name != PROCESS_EXEC
            || source.process_exec.as_ref().and_then(|e| e.success) != Some(true)
        {
            return Ok(None);
        }
        let path = source
            .process
            .as_ref()
            .and_then(|p| p.executable_path.as_deref());
        if !path.is_some_and(|p| {
            [
                "/bin/sh",
                "/bin/csh",
                "/bin/tcsh",
                "/usr/local/bin/bash",
                "/usr/local/bin/zsh",
            ]
            .contains(&p)
        }) {
            return Ok(None);
        }
        let definition = self.definition();
        Ok(Some(Detection {
            id: definition.id.into(),
            name: definition.name.into(),
            description: definition.description.into(),
            severity: definition.severity.into(),
            tags: definition.tags.iter().map(|s| (*s).into()).collect(),
            mitre_techniques: vec![],
            detected_from: Some(EventReference {
                id: source.id,
                name: source.name.clone(),
                record_id: source.record_id.clone(),
            }),
        }))
    }
}
