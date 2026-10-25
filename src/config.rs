// SPDX-License-Identifier: Apache-2.0

use crate::{
    domain::{MAX_RECORD_BYTES, MIN_RECORD_BYTES, Policy, validate_policies},
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};

pub const MAX_CONFIG_BYTES: usize = 1 << 20;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(deserialize_with = "version")]
    pub version: i32,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub collector: CollectorConfig,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub pipeline: PipelineConfig,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub policies: Vec<Policy>,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub detectors: Vec<String>,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub outputs: Vec<DestinationConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CollectorConfig {
    #[serde(rename = "type", deserialize_with = "collector_kind")]
    pub kind: String,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub path: String,
    #[serde(deserialize_with = "record_bytes")]
    pub max_record_bytes: usize,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            kind: "replay".into(),
            path: String::new(),
            max_record_bytes: 65536,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PipelineConfig {
    #[serde(deserialize_with = "queue_size")]
    pub queue_size: usize,
}

fn version<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<i32, D::Error> {
    Ok(Option::<i32>::deserialize(d)?.unwrap_or(1))
}

fn collector_kind<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_else(|| "replay".into()))
}

fn record_bytes<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<usize, D::Error> {
    Ok(Option::<usize>::deserialize(d)?.unwrap_or(65536))
}

fn queue_size<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<usize, D::Error> {
    Ok(Option::<usize>::deserialize(d)?.unwrap_or(64))
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self { queue_size: 64 }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConfigDuration(pub i64);

impl ConfigDuration {
    pub fn duration(self) -> Duration {
        Duration::from_nanos(self.0.max(0) as u64)
    }

    pub fn parse(text: &str) -> std::result::Result<Self, &'static str> {
        let (negative, mut remaining) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        if remaining == "0" {
            return Ok(Self(0));
        }
        if remaining.is_empty() {
            return Err("invalid duration");
        }
        let mut total = 0_u128;
        while !remaining.is_empty() {
            let end = remaining
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .ok_or("invalid duration")?;
            let number = &remaining[..end];
            if number.is_empty() || number == "." {
                return Err("invalid duration");
            }
            remaining = &remaining[end..];
            let unit_end = remaining
                .find(|c: char| c.is_ascii_digit() || c == '.')
                .unwrap_or(remaining.len());
            let unit = &remaining[..unit_end];
            remaining = &remaining[unit_end..];
            let multiplier: u128 = match unit {
                "ns" => 1,
                "us" | "µs" | "μs" => 1_000,
                "ms" => 1_000_000,
                "s" => 1_000_000_000,
                "m" => 60_000_000_000,
                "h" => 3_600_000_000_000,
                _ => return Err("invalid duration"),
            };
            let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
            let integer = if integer.is_empty() {
                0
            } else {
                integer.parse::<u128>().map_err(|_| "invalid duration")?
            };
            if !fraction.bytes().all(|b| b.is_ascii_digit()) {
                return Err("invalid duration");
            }
            let fraction = &fraction[..fraction.len().min(18)];
            let fraction_value = if fraction.is_empty() {
                0
            } else {
                fraction.parse::<u128>().map_err(|_| "invalid duration")?
            };
            let value = integer
                .checked_mul(multiplier)
                .and_then(|n| {
                    n.checked_add(fraction_value * multiplier / 10_u128.pow(fraction.len() as u32))
                })
                .ok_or("invalid duration")?;
            total = total.checked_add(value).ok_or("invalid duration")?;
            if total > i64::MAX as u128 + u128::from(negative) {
                return Err("invalid duration");
            }
        }
        Ok(Self(if negative {
            -(total as i128) as i64
        } else {
            total as i64
        }))
    }
}

impl Serialize for ConfigDuration {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}ns", self.0))
    }
}

impl<'de> Deserialize<'de> for ConfigDuration {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(text) => Self::parse(&text).map_err(serde::de::Error::custom),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DestinationConfig {
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub name: String,
    #[serde(rename = "type", deserialize_with = "crate::domain::null_default")]
    pub kind: String,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub format: String,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub path: String,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub url: String,
    pub timeout: ConfigDuration,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub retries: i32,
    #[serde(deserialize_with = "unique_headers")]
    pub headers: BTreeMap<String, String>,
    #[serde(deserialize_with = "crate::domain::null_default")]
    pub allow_http: bool,
}

fn unique_headers<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, String>, D::Error> {
    struct Headers;
    impl<'de> serde::de::Visitor<'de> for Headers {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a header map")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((name, value)) = map.next_entry::<String, Option<String>>()? {
                if result.insert(name, value.unwrap_or_default()).is_some() {
                    return Err(serde::de::Error::custom("duplicate header"));
                }
            }
            Ok(result)
        }
        fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
            Ok(BTreeMap::new())
        }
    }
    d.deserialize_any(Headers)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            collector: CollectorConfig::default(),
            pipeline: PipelineConfig::default(),
            policies: vec![],
            detectors: vec!["shell-exec".into()],
            outputs: vec![DestinationConfig {
                name: "stdout".into(),
                kind: "stdout".into(),
                format: "json".into(),
                ..DestinationConfig::default()
            }],
        }
    }
}

impl Config {
    pub fn load(reader: impl Read) -> Result<Self> {
        let mut data = Vec::new();
        reader
            .take((MAX_CONFIG_BYTES + 1) as u64)
            .read_to_end(&mut data)
            .map_err(|e| Error::io("read configuration", e))?;
        if data.len() > MAX_CONFIG_BYTES {
            return Err(Error::message("configuration exceeds 1048576 bytes"));
        }
        let invalid =
            || Error::message("invalid YAML configuration: check syntax, field names, and types");
        if std::str::from_utf8(&data).is_ok_and(|text| {
            text.trim_start_matches('\u{feff}')
                .lines()
                .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        }) {
            return Err(invalid());
        }
        let mut documents = serde_yaml::Deserializer::from_slice(&data);
        let document = documents.next().ok_or_else(invalid)?;
        let result = match Self::deserialize(document) {
            Ok(config) => config,
            Err(_) => {
                let mut value: serde_yaml::Value =
                    serde_yaml::from_slice(&data).map_err(|_| invalid())?;
                value.apply_merge().map_err(|_| invalid())?;
                Self::deserialize(value).map_err(|_| invalid())?
            }
        };
        if documents.next().is_some() {
            return Err(Error::message(
                "configuration must contain exactly one YAML document",
            ));
        }
        Ok(result)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(Error::message("configuration version must be 1"));
        }
        if self.collector.kind != "replay" {
            return Err(Error::message(
                "only the replay collector is implemented; no live collector is available",
            ));
        }
        if self.collector.path.trim().is_empty() || self.collector.path.len() > 4096 {
            return Err(Error::message(
                "collector.path is required and must be at most 4096 bytes",
            ));
        }
        if !(MIN_RECORD_BYTES..=MAX_RECORD_BYTES).contains(&self.collector.max_record_bytes) {
            return Err(Error::message(
                "collector.max_record_bytes must be between 256 and 1048576",
            ));
        }
        if !(1..=4096).contains(&self.pipeline.queue_size) {
            return Err(Error::message(
                "pipeline.queue_size must be between 1 and 4096",
            ));
        }
        if self.pipeline.queue_size as u64 * self.collector.max_record_bytes as u64 > 64 << 20 {
            return Err(Error::message(
                "queue_size times max_record_bytes must not exceed 64 MiB",
            ));
        }
        validate_policies(&self.policies)?;
        if self.detectors.len() > 1 || self.detectors.first().is_some_and(|s| s != "shell-exec") {
            return Err(Error::message(
                "only the shell-exec detector is implemented; use [] to disable it",
            ));
        }
        if !(1..=8).contains(&self.outputs.len()) {
            return Err(Error::message("between 1 and 8 outputs are required"));
        }
        let mut names = HashSet::new();
        let mut stdout = 0;
        for (i, output) in self.outputs.iter().enumerate() {
            output
                .validate()
                .map_err(|e| e.context(format!("output {}", i + 1)))?;
            if !names.insert(&output.name) {
                return Err(Error::message("output names must be unique"));
            }
            stdout += usize::from(output.kind == "stdout");
        }
        if stdout > 1 {
            return Err(Error::message(
                "only one stdout output is allowed; do not mix JSON and table streams",
            ));
        }
        Ok(())
    }

    pub fn validate_paths(&self, config_path: &str) -> Result<()> {
        let mut inputs = vec![self.collector.path.as_str()];
        if !config_path.is_empty() {
            inputs.push(config_path);
        }
        for input in inputs {
            let metadata = fs::metadata(input).map_err(|e| Error::io("stat input file", e))?;
            if !metadata.is_file() {
                return Err(Error::message("input must be a regular file"));
            }
            for output in self.outputs.iter().filter(|o| o.kind == "file") {
                let out = match fs::metadata(&output.path) {
                    Ok(metadata) => Some(metadata),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(Error::io("stat output file", e)),
                };
                let same_path = absolute(input)? == absolute(&output.path)?;
                let same_file = if out.is_some() {
                    same_file::is_same_file(input, &output.path)
                        .map_err(|e| Error::io("compare input and output files", e))?
                } else {
                    false
                };
                if same_path || same_file {
                    return Err(Error::message(
                        "file output cannot also be a configuration or replay input",
                    ));
                }
                if out.is_some_and(|m| !m.is_file()) {
                    return Err(Error::message("file output must be a regular file"));
                }
            }
        }
        Ok(())
    }
}

fn absolute(path: &str) -> Result<PathBuf> {
    std::path::absolute(Path::new(path)).map_err(|e| Error::io("resolve path", e))
}

impl DestinationConfig {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(Error::message("name must contain 1-64 bytes"));
        }
        match self.kind.as_str() {
            "stdout" | "file" => {
                if self.format != "json" && !(self.kind == "stdout" && self.format == "table") {
                    return Err(Error::message(
                        "stdout supports json/table; files support json only",
                    ));
                }
                if self.kind == "file" && (self.path.is_empty() || self.path.len() > 4096) {
                    return Err(Error::message(
                        "file output requires a path of 1-4096 bytes",
                    ));
                }
                if self.kind == "stdout" && !self.path.is_empty() {
                    return Err(Error::message("stdout output cannot set path"));
                }
                if !self.url.is_empty()
                    || self.timeout.0 != 0
                    || self.retries != 0
                    || !self.headers.is_empty()
                    || self.allow_http
                {
                    return Err(Error::message(
                        "non-webhook output cannot set webhook options",
                    ));
                }
            }
            "webhook" => {
                if self.format != "json" || !self.path.is_empty() {
                    return Err(Error::message(
                        "webhook requires format json and cannot set path",
                    ));
                }
                self.validate_url()?;
                if !(1_000_000..=30_000_000_000).contains(&self.timeout.0) {
                    return Err(Error::message(
                        "webhook timeout must be between 1ms and 30s",
                    ));
                }
                if !(0..=5).contains(&self.retries) {
                    return Err(Error::message("webhook retries must be between 0 and 5"));
                }
                if self.headers.len() > 32 {
                    return Err(Error::message("at most 32 webhook headers are allowed"));
                }
                let mut names = HashSet::new();
                for (name, value) in &self.headers {
                    if !valid_header(name, value) {
                        return Err(Error::message("invalid webhook header name or value"));
                    }
                    let lower = name.to_ascii_lowercase();
                    if !names.insert(lower.clone()) {
                        return Err(Error::message("duplicate case-insensitive webhook header"));
                    }
                    if [
                        "host",
                        "content-type",
                        "content-length",
                        "transfer-encoding",
                        "connection",
                    ]
                    .contains(&lower.as_str())
                    {
                        return Err(Error::message(
                            "webhook headers cannot override transport or JSON content headers",
                        ));
                    }
                }
            }
            _ => return Err(Error::message("unsupported destination type")),
        }
        Ok(())
    }

    fn validate_url(&self) -> Result<()> {
        if self.url.len() > 4096 {
            return Err(Error::message("webhook URL exceeds 4096 bytes"));
        }
        let invalid =
            || Error::message("webhook requires an absolute URL without userinfo or fragment");
        let parsed = reqwest::Url::parse(&self.url).map_err(|_| invalid())?;
        let (_, rest) = self.url.split_once("://").ok_or_else(invalid)?;
        let scheme = parsed.scheme();
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if parsed.host_str().is_none()
            || authority.contains('@')
            || parsed.fragment().is_some_and(|s| !s.is_empty())
            || self.url.chars().any(|c| c.is_ascii_control())
        {
            return Err(invalid());
        }
        let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
            let (host, suffix) = rest.split_once(']').ok_or_else(invalid)?;
            (host, suffix.strip_prefix(':').unwrap_or(""))
        } else {
            authority.split_once(':').unwrap_or((authority, ""))
        };
        if !port.is_empty() && port.parse::<u16>().ok().filter(|p| *p != 0).is_none() {
            return Err(Error::message("webhook port must be between 1 and 65535"));
        }
        let loopback = host.eq_ignore_ascii_case("localhost")
            || host.parse::<IpAddr>().is_ok_and(|ip| match ip {
                IpAddr::V4(ip) => ip.is_loopback(),
                IpAddr::V6(ip) => ip
                    .to_ipv4_mapped()
                    .map_or_else(|| ip.is_loopback(), |v4| v4.is_loopback()),
            });
        if scheme != "https" && !(scheme == "http" && self.allow_http && loopback) {
            return Err(Error::message(
                "webhook requires HTTPS; explicitly enabled HTTP is limited to loopback testing",
            ));
        }
        Ok(())
    }
}

fn valid_header(name: &str, value: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && value.len() <= 4096
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        && value.chars().all(|c| c >= ' ' && c != '\u{7f}')
}
