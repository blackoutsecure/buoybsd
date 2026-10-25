// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::DestinationConfig,
    domain::Event,
    error::{Error, Result},
    io::{BoxFuture, Writer},
    wire::marshal,
};
use std::{io, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DestinationStats {
    pub attempts: u64,
    pub errors: u64,
}

pub trait Destination: Send {
    fn name(&self) -> &str;
    fn stats(&self) -> DestinationStats;
    fn write_event<'a>(
        &'a mut self,
        cancel: &'a CancellationToken,
        event: &'a Event,
    ) -> BoxFuture<'a, Result<()>>;
    fn close(&mut self) -> BoxFuture<'_, Result<()>>;
}

pub struct Json {
    name: String,
    writer: Box<dyn Writer>,
    owned: bool,
    stats: DestinationStats,
}

impl Json {
    pub fn new(name: String, writer: Box<dyn Writer>, owned: bool) -> Result<Self> {
        if name.is_empty() {
            return Err(Error::message("JSON output requires a name and writer"));
        }
        Ok(Self {
            name,
            writer,
            owned,
            stats: DestinationStats::default(),
        })
    }
}

impl Destination for Json {
    fn name(&self) -> &str {
        &self.name
    }
    fn stats(&self) -> DestinationStats {
        self.stats
    }
    fn write_event<'a>(
        &'a mut self,
        cancel: &'a CancellationToken,
        event: &'a Event,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.stats.attempts += 1;
            let result = (|| {
                if cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let mut data = marshal(event)?;
                data.push(b'\n');
                write(&mut *self.writer, &data)
            })();
            self.stats.errors += u64::from(result.is_err());
            result
        })
    }
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            if self.owned {
                self.writer
                    .close()
                    .map_err(|e| Error::io("close JSON writer", e))?;
            }
            Ok(())
        })
    }
}

fn write(writer: &mut dyn Writer, bytes: &[u8]) -> Result<()> {
    match writer.write(bytes) {
        Ok(n) if n == bytes.len() => Ok(()),
        Ok(_) => Err(Error::io(
            "write event",
            io::Error::new(io::ErrorKind::WriteZero, "short write"),
        )),
        Err(error) => Err(Error::io("write event", error)),
    }
}

pub struct Table {
    name: String,
    writer: Box<dyn Writer>,
    header: bool,
    stats: DestinationStats,
}

impl Table {
    pub fn new(name: String, writer: Box<dyn Writer>) -> Result<Self> {
        if name.is_empty() {
            return Err(Error::message("table output requires a name and writer"));
        }
        Ok(Self {
            name,
            writer,
            header: false,
            stats: DestinationStats::default(),
        })
    }

    fn emit(&mut self, cancel: &CancellationToken, event: &Event) -> Result<()> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let timestamp = event
            .timestamp
            .ok_or_else(|| Error::message("table output requires an event timestamp"))?
            .rfc3339_nano()
            .map_err(|_| Error::message("invalid table event timestamp"))?;
        if !self.header {
            write(&mut *self.writer, b"TIME\tEVENT\tPID\tEXECUTABLE\n")?;
            self.header = true;
        }
        let pid = event
            .process
            .as_ref()
            .and_then(|p| p.pid)
            .map_or("-".into(), |p| p.to_string());
        let executable = event
            .process
            .as_ref()
            .and_then(|p| p.executable_path.as_deref())
            .map_or("-".into(), quote_to_ascii);
        let line = format!(
            "{timestamp}\t{}\t{pid}\t{executable}\n",
            quote_to_ascii(&event.name)
        );
        write(&mut *self.writer, line.as_bytes())
    }
}

impl Destination for Table {
    fn name(&self) -> &str {
        &self.name
    }
    fn stats(&self) -> DestinationStats {
        self.stats
    }
    fn write_event<'a>(
        &'a mut self,
        cancel: &'a CancellationToken,
        event: &'a Event,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.stats.attempts += 1;
            let result = self.emit(cancel, event);
            self.stats.errors += u64::from(result.is_err());
            result
        })
    }
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

pub fn quote_to_ascii(text: &str) -> String {
    use std::fmt::Write;
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            ' '..='~' => out.push(c),
            c if (c as u32) < 0x80 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c if (c as u32) < 0x10000 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => {
                let _ = write!(out, "\\U{:08x}", c as u32);
            }
        }
    }
    out.push('"');
    out
}

pub struct Webhook {
    spec: DestinationConfig,
    client: Option<reqwest::Client>,
    stats: DestinationStats,
}

impl Webhook {
    pub fn new(spec: DestinationConfig) -> Result<Self> {
        if spec.kind != "webhook" {
            return Err(Error::message("webhook output requires type webhook"));
        }
        spec.validate()?;
        let timeout = spec.timeout.duration();
        let client = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .connect_timeout(timeout)
            .timeout(timeout)
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(1)
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .map_err(|_| Error::message("cannot initialize webhook TLS transport"))?;
        Ok(Self {
            spec,
            client: Some(client),
            stats: DestinationStats::default(),
        })
    }

    #[cfg(test)]
    pub(crate) fn trust_test_certificate(&mut self, certificate: reqwest::Certificate) {
        self.client = Some(
            reqwest::Client::builder()
                .retry(reqwest::retry::never())
                .redirect(reqwest::redirect::Policy::none())
                .min_tls_version(reqwest::tls::Version::TLS_1_2)
                .add_root_certificate(certificate)
                .timeout(self.spec.timeout.duration())
                .build()
                .expect("test TLS client"),
        );
    }

    async fn deliver(&self, record_id: &str, bytes: &[u8]) -> (bool, Result<()>) {
        let Some(client) = &self.client else {
            return (false, Err(Error::message("webhook destination is closed")));
        };
        let mut request = client
            .post(&self.spec.url)
            .header("Content-Type", "application/json")
            .header("X-BuoyBSD-Record-ID", record_id)
            .body(bytes.to_vec());
        for (name, value) in &self.spec.headers {
            request = request.header(name, value);
        }
        let request = match request.build() {
            Ok(request) => request,
            Err(_) => {
                return (
                    false,
                    Err(Error::message("cannot construct webhook request")),
                );
            }
        };
        let mut response = match client.execute(request).await {
            Ok(response) => response,
            Err(_) => {
                return (
                    true,
                    Err(Error::message(
                        "webhook transport failed: check connectivity, TLS trust, and timeout",
                    )),
                );
            }
        };
        let status = response.status().as_u16();
        let mut read = 0;
        while read < 64 << 10 {
            match response.chunk().await {
                Ok(Some(chunk)) => read += chunk.len(),
                Ok(None) => break,
                Err(_) => {
                    return (
                        true,
                        Err(Error::message("webhook response read or close failed")),
                    );
                }
            }
        }
        if (200..300).contains(&status) {
            (false, Ok(()))
        } else {
            (
                status == 429 || status >= 500,
                Err(Error::message(format!("webhook returned HTTP {status}"))),
            )
        }
    }
}

impl Destination for Webhook {
    fn name(&self) -> &str {
        &self.spec.name
    }
    fn stats(&self) -> DestinationStats {
        self.stats
    }
    fn write_event<'a>(
        &'a mut self,
        cancel: &'a CancellationToken,
        event: &'a Event,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let body = match marshal(event) {
                Ok(body) => body,
                Err(error) => {
                    self.stats.attempts += 1;
                    self.stats.errors += 1;
                    return Err(error);
                }
            };
            for attempt in 0..=self.spec.retries {
                if cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                self.stats.attempts += 1;
                let (retry, result) = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => (false, Err(Error::Cancelled)),
                    result = self.deliver(&event.record_id, &body) => result,
                };
                match result {
                    Ok(()) => return Ok(()),
                    Err(error) => {
                        self.stats.errors += 1;
                        if !retry || attempt == self.spec.retries {
                            return Err(error);
                        }
                    }
                }
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(Error::Cancelled),
                    _ = tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))) => {}
                }
            }
            Err(Error::message(
                "webhook delivery exhausted its retry budget",
            ))
        })
    }
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.client.take();
            Ok(())
        })
    }
}

#[cfg(test)]
#[path = "../tests/support/webhook_tls.rs"]
mod tls_tests;
