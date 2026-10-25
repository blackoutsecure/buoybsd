// SPDX-License-Identifier: Apache-2.0

use crate::{
    domain::*,
    error::{Error, Result},
    io::{BoxFuture, Input},
    time::Timestamp,
};
use serde::{
    Deserialize,
    de::{DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Value};
use std::{
    borrow::Cow,
    fmt,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Mutex, mpsc},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollectorStats {
    pub received: u64,
    pub dropped: u64,
    pub parser_errors: u64,
    pub restarts: u64,
}

pub trait Collector: Send + Sync {
    fn name(&self) -> &str;
    fn stats(&self) -> CollectorStats;
    fn run<'a>(
        &'a self,
        cancel: CancellationToken,
        out: mpsc::Sender<RawEvent>,
    ) -> BoxFuture<'a, Result<()>>;
    fn close(&self) -> BoxFuture<'_, Result<()>>;
}

pub struct Replay {
    input: Mutex<Option<BufReader<Box<dyn Input>>>>,
    max_bytes: usize,
    started: AtomicBool,
    closed: AtomicBool,
    close_result: Mutex<Option<Result<()>>>,
    received: AtomicU64,
    dropped: AtomicU64,
    parser_errors: AtomicU64,
}

impl Replay {
    pub fn new(input: Box<dyn Input>, max_bytes: usize) -> Result<Self> {
        if !(MIN_RECORD_BYTES..=MAX_RECORD_BYTES).contains(&max_bytes) {
            return Err(Error::message(
                "replay max_record_bytes must be between 256 and 1048576",
            ));
        }
        Ok(Self {
            input: Mutex::new(Some(BufReader::with_capacity(max_bytes + 1, input))),
            max_bytes,
            started: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            close_result: Mutex::new(None),
            received: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            parser_errors: AtomicU64::new(0),
        })
    }

    async fn collect(&self, cancel: CancellationToken, out: mpsc::Sender<RawEvent>) -> Result<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::message("replay collector is closed"));
        }
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(Error::message("replay collector can only run once"));
        }
        let mut input = self.input.lock().await;
        let reader = input
            .as_mut()
            .ok_or_else(|| Error::message("replay collector is closed"))?;
        for sequence in 1_u64.. {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let line = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                result = read_record(reader, self.max_bytes) => result,
            };
            let line = match line {
                Ok(Some(line)) => line,
                Ok(None) => return Ok(()),
                Err(ReadFailure::Oversize) => {
                    self.received.fetch_add(1, Ordering::Relaxed);
                    self.parser_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(Error::message(format!(
                        "replay record {sequence} exceeds max_record_bytes"
                    )));
                }
                Err(ReadFailure::Io(error)) => {
                    return Err(Error::io(format!("read replay record {sequence}"), error));
                }
            };
            self.received.fetch_add(1, Ordering::Relaxed);
            let raw = parse(&line, sequence)
                .inspect_err(|_| {
                    self.parser_errors.fetch_add(1, Ordering::Relaxed);
                })
                .map_err(|e| e.context(format!("replay record {sequence}")))?;
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    return Err(Error::Cancelled);
                }
                result = out.send(raw) => {
                    if result.is_err() {
                        self.dropped.fetch_add(1, Ordering::Relaxed);
                        return Err(Error::Cancelled);
                    }
                }
            }
        }
        Err(Error::message("replay sequence exhausted"))
    }
}

impl Collector for Replay {
    fn name(&self) -> &str {
        "replay"
    }

    fn stats(&self) -> CollectorStats {
        CollectorStats {
            received: self.received.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            parser_errors: self.parser_errors.load(Ordering::Relaxed),
            restarts: 0,
        }
    }

    fn run<'a>(
        &'a self,
        cancel: CancellationToken,
        out: mpsc::Sender<RawEvent>,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.collect(cancel, out))
    }

    fn close(&self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            let mut result = self.close_result.lock().await;
            if let Some(result) = result.as_ref() {
                return result.clone();
            }
            self.closed.store(true, Ordering::Release);
            let reader = self.input.lock().await.take();
            let closed = match reader {
                Some(reader) => reader.into_inner().close().await,
                None => Ok(()),
            };
            *result = Some(closed.clone());
            closed
        })
    }
}

enum ReadFailure {
    Oversize,
    Io(std::io::Error),
}

async fn read_record(
    reader: &mut BufReader<Box<dyn Input>>,
    max: usize,
) -> std::result::Result<Option<Vec<u8>>, ReadFailure> {
    let mut record = Vec::new();
    loop {
        let bytes = reader.fill_buf().await.map_err(ReadFailure::Io)?;
        if bytes.is_empty() {
            return Ok((!record.is_empty()).then_some(record));
        }
        let consumed = bytes
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |n| n + 1);
        if record.len() + consumed > max {
            return Err(ReadFailure::Oversize);
        }
        let newline = bytes[consumed - 1] == b'\n';
        record.extend_from_slice(&bytes[..consumed]);
        reader.consume(consumed);
        if newline {
            return Ok(Some(record));
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Record {
    #[serde(deserialize_with = "text")]
    version: String,
    #[serde(deserialize_with = "text")]
    event: String,
    #[serde(deserialize_with = "text")]
    timestamp: String,
    #[serde(deserialize_with = "text")]
    source_os: String,
    #[serde(deserialize_with = "text")]
    evidence: String,
    source_id: Option<String>,
    host: Option<Host>,
    process: Option<Process>,
    jail: Option<Jail>,
    exec: Option<ProcessExec>,
}

fn text<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

pub fn parse(line: &[u8], sequence: u64) -> Result<RawEvent> {
    if std::str::from_utf8(line).is_err() {
        return Err(Error::message("record is not valid UTF-8"));
    }
    let compatible = unicode_escapes(line);
    let mut decoder = serde_json::Deserializer::from_slice(&compatible);
    let value = StrictValue(0).deserialize(&mut decoder).map_err(|_| {
        Error::message("invalid JSON record: check duplicate/canonical fields and nesting")
    })?;
    if !value.is_object() {
        return Err(Error::message("record must be a JSON object"));
    }
    decoder
        .end()
        .map_err(|_| Error::message("record must contain exactly one JSON object"))?;
    let input: Record = serde_json::from_value(value)
        .map_err(|_| Error::message("invalid replay fields or values"))?;
    if input.version != REPLAY_VERSION {
        return Err(Error::message("unsupported replay version"));
    }
    if input.event != PROCESS_EXEC {
        return Err(Error::message("unsupported replay event"));
    }
    if input.source_os != "freebsd" || input.evidence != "synthetic" {
        return Err(Error::message(
            "replay requires source_os freebsd and evidence synthetic",
        ));
    }
    let timestamp = Timestamp::parse(&input.timestamp)?;
    let mut fields = vec![input.source_id.as_deref()];
    if let Some(host) = &input.host {
        fields.push(host.name.as_deref());
    }
    if let Some(process) = &input.process {
        if process.pid == Some(0) {
            return Err(Error::message(
                "process-exec PID must be positive when present",
            ));
        }
        fields.extend([
            process.executable_path.as_deref(),
            process.command.as_deref(),
        ]);
    }
    if let Some(jail) = &input.jail {
        fields.push(jail.name.as_deref());
    }
    if fields
        .into_iter()
        .flatten()
        .any(|s| s.len() > MAX_FIELD_BYTES)
    {
        return Err(Error::message("record field exceeds 4096 bytes"));
    }
    if let Some(exec) = &input.exec {
        if exec.argv.len() > MAX_ARGUMENTS {
            return Err(Error::message("argv exceeds 128 entries"));
        }
        if exec.argv.iter().any(|s| s.len() > MAX_FIELD_BYTES) {
            return Err(Error::message("argument exceeds 4096 bytes"));
        }
    }
    Ok(RawEvent {
        name: input.event,
        timestamp: Some(timestamp),
        collector: "replay".into(),
        operating_system: input.source_os,
        evidence: input.evidence,
        source_id: input.source_id,
        sequence,
        host: input.host,
        process: input.process,
        jail: input.jail,
        exec: input.exec,
        record: line.to_vec(),
        media_type: "application/x-buoybsd-replay+json".into(),
    })
}

struct StrictValue(u8);

fn unicode_escapes(line: &[u8]) -> Cow<'_, [u8]> {
    fn code(bytes: &[u8]) -> Option<u16> {
        if bytes.len() < 6 || &bytes[..2] != b"\\u" {
            return None;
        }
        std::str::from_utf8(&bytes[2..6])
            .ok()
            .and_then(|s| u16::from_str_radix(s, 16).ok())
    }
    let mut data = Cow::Borrowed(line);
    let mut inside = false;
    let mut at = 0;
    while at < line.len() {
        if line[at] == b'"' {
            inside = !inside;
        }
        if inside && line[at] == b'\\' {
            if let Some(value) = code(&line[at..]) {
                if (0xd800..=0xdbff).contains(&value)
                    && code(&line[at + 6..]).is_some_and(|low| (0xdc00..=0xdfff).contains(&low))
                {
                    at += 12;
                    continue;
                }
                if (0xd800..=0xdfff).contains(&value) {
                    // Retain the original reader's U+FFFD substitution without altering raw evidence.
                    data.to_mut()[at..at + 6].copy_from_slice(b"\\ufffd");
                }
                at += 6;
                continue;
            }
            at += 2;
        } else {
            at += 1;
        }
    }
    data
}

impl<'de> DeserializeSeed<'de> for StrictValue {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        d: D,
    ) -> std::result::Result<Value, D::Error> {
        if self.0 > 16 {
            return Err(serde::de::Error::custom("JSON nesting exceeds 16 levels"));
        }
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for StrictValue {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("strict JSON")
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Value, E> {
        Ok(v.into())
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = a.next_element_seed(StrictValue(self.0 + 1))? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = a.next_key::<String>()? {
            if key != key.to_lowercase() || values.contains_key(&key) {
                return Err(serde::de::Error::custom(
                    "duplicate or noncanonical JSON field",
                ));
            }
            values.insert(key, a.next_value_seed(StrictValue(self.0 + 1))?);
        }
        Ok(Value::Object(values))
    }
}
