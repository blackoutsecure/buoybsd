// SPDX-License-Identifier: Apache-2.0

use crate::{io::Writer, pipeline::Stats, time::Timestamp};
use serde_json::{Map, Value};
use std::{
    fmt,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::SubscriberExt};

#[derive(Clone)]
pub struct Diagnostics(Arc<Mutex<Box<dyn Writer>>>);

impl Diagnostics {
    pub fn new(writer: Box<dyn Writer>) -> Self {
        Self(Arc::new(Mutex::new(writer)))
    }

    pub fn plain(&self, text: &str) {
        if let Ok(mut writer) = self.0.lock() {
            let _ = writer.write_all(text.as_bytes());
        }
    }

    pub fn stopped(&self, collector: &str, stats: Stats, failed: bool) {
        let subscriber = tracing_subscriber::registry().with(DiagnosticLayer(self.clone()));
        let metrics = serde_json::to_string(&stats).expect("integer counters serialize");
        tracing::subscriber::with_default(subscriber, || {
            if failed {
                tracing::error!(component = "pipeline", collector, metrics = %metrics, "pipeline stopped");
            } else {
                tracing::info!(component = "pipeline", collector, metrics = %metrics, "pipeline stopped");
            }
        });
    }

    pub fn failed(&self, operation: &str, message: &str, error: &dyn fmt::Display) {
        let subscriber = tracing_subscriber::registry().with(DiagnosticLayer(self.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(component = "cli", operation, error = %error, "{message}");
        });
    }
}

struct DiagnosticLayer(Diagnostics);

impl<S: Subscriber> Layer<S> for DiagnosticLayer {
    fn on_event(&self, event: &Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = Timestamp {
            seconds: now.as_secs() as i64,
            nanos: now.subsec_nanos() as i32,
        }
        .rfc3339_nano()
        .unwrap_or_default();
        let mut fields = Fields(Map::new());
        event.record(&mut fields);
        fields.0.insert("time".into(), timestamp.into());
        fields
            .0
            .insert("level".into(), event.metadata().level().as_str().into());
        if let Ok(mut data) = serde_json::to_vec(&fields.0) {
            data.push(b'\n');
            if let Ok(mut writer) = self.0.0.lock() {
                let _ = writer.write_all(&data);
            }
        }
    }
}

struct Fields(Map<String, Value>);
impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        let name = if field.name() == "message" {
            "msg"
        } else {
            field.name()
        };
        let value = if name == "metrics" {
            serde_json::from_str(value).unwrap_or_else(|_| value.into())
        } else {
            value.into()
        };
        self.0.insert(name.into(), value);
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_str(field, &format!("{value:?}"));
    }
}
