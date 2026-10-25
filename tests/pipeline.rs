// SPDX-License-Identifier: Apache-2.0

mod support;
use buoybsd::{
    domain::*,
    error::{Error, Result},
    io::BoxFuture,
    output::{Destination, DestinationStats, Json},
    pipeline::{self, Options, Stats},
    replay::{Collector, CollectorStats, Replay},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use support::*;
use tokio::sync::{Notify, mpsc};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct TestCollector {
    events: Vec<RawEvent>,
    run_error: bool,
    close_error: bool,
    received: AtomicU64,
    dropped: AtomicU64,
    closes: AtomicU64,
}
impl Collector for TestCollector {
    fn name(&self) -> &str {
        "test-replay"
    }
    fn stats(&self) -> CollectorStats {
        CollectorStats {
            received: self.received.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            ..CollectorStats::default()
        }
    }
    fn run<'a>(
        &'a self,
        cancel: CancellationToken,
        out: mpsc::Sender<RawEvent>,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            for raw in &self.events {
                if cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                self.received.fetch_add(1, Ordering::Relaxed);
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        self.dropped.fetch_add(1, Ordering::Relaxed); return Err(Error::Cancelled);
                    }
                    result = out.send(raw.clone()) => if result.is_err() {
                        self.dropped.fetch_add(1, Ordering::Relaxed); return Err(Error::Cancelled);
                    }
                }
            }
            if self.run_error {
                Err(Error::message("fixture collection failure"))
            } else {
                Ok(())
            }
        })
    }
    fn close(&self) -> BoxFuture<'_, Result<()>> {
        self.closes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            if self.close_error {
                Err(Error::message("fixture collector close failure"))
            } else {
                Ok(())
            }
        })
    }
}

fn options(policies: Vec<Policy>) -> Options {
    Options {
        queue_size: 1,
        policies: PolicySet::compile(policies).unwrap(),
        detectors: vec![Box::new(ShellExec)],
    }
}
fn json(writer: Box<dyn buoybsd::io::Writer>, owned: bool) -> Box<dyn Destination> {
    Box::new(Json::new("json".into(), writer, owned).unwrap())
}

#[tokio::test]
async fn filtering_detection_and_exact_metrics() {
    let mut second = observation();
    second.process.as_mut().unwrap().uid = Some(1000);
    let source = Arc::new(TestCollector {
        events: vec![observation(), second],
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source.clone(),
        vec![json(Box::new(std::io::sink()), false)],
        options(vec![Policy {
            name: "root".into(),
            uids: vec![0],
            ..Policy::default()
        }]),
    )
    .await;
    result.unwrap();
    assert_eq!(
        stats,
        Stats {
            events_received: 2,
            events_normalized: 2,
            events_filtered: 1,
            detections_generated: 1,
            events_delivered: 1,
            delivery_attempts: 1,
            ..Stats::default()
        }
    );
    assert_eq!(source.closes.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn output_failure_stops_and_accounts_for_every_abandoned_event() {
    let source = Arc::new(TestCollector {
        events: vec![observation(); 100],
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source.clone(),
        vec![json(Box::new(FailedWriter), false)],
        options(vec![]),
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("fixture write failure")
    );
    assert!((1..=3).contains(&stats.events_received));
    assert_eq!(stats.events_delivered, 0);
    assert_eq!(stats.events_dropped, stats.events_received);
    assert_eq!(stats.destination_errors, 1);
    assert_eq!(source.closes.load(Ordering::Relaxed), 1);
}

struct BlockingDestination {
    entered: Arc<Notify>,
    closes: Arc<AtomicU64>,
    stats: DestinationStats,
}
impl Destination for BlockingDestination {
    fn name(&self) -> &str {
        "blocking-fixture"
    }
    fn stats(&self) -> DestinationStats {
        self.stats
    }
    fn write_event<'a>(
        &'a mut self,
        cancel: &'a CancellationToken,
        _: &'a Event,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async {
            self.stats.attempts += 1;
            self.entered.notify_one();
            cancel.cancelled().await;
            self.stats.errors += 1;
            Err(Error::Cancelled)
        })
    }
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        self.closes.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn cancellation_unblocks_saturation_and_closes_owned_resources() {
    let source = Arc::new(TestCollector {
        events: vec![observation(); 1000],
        ..TestCollector::default()
    });
    let entered = Arc::new(Notify::new());
    let closes = Arc::new(AtomicU64::new(0));
    let destination = BlockingDestination {
        entered: entered.clone(),
        closes: closes.clone(),
        stats: DestinationStats::default(),
    };
    let token = CancellationToken::new();
    let child = token.clone();
    let collector = source.clone();
    let task = tokio::spawn(async move {
        pipeline::run(
            &child,
            collector,
            vec![Box::new(destination)],
            options(vec![]),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert!(source.stats().received <= 3);
    token.cancel();
    let (stats, result) = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap();
    assert_cancelled(result);
    assert_eq!(stats.events_delivered, 0);
    assert_eq!(stats.events_dropped, stats.events_received);
    assert_eq!(source.closes.load(Ordering::Relaxed), 1);
    assert_eq!(closes.load(Ordering::Relaxed), 1);
}

struct FailedDetector;
impl Detector for FailedDetector {
    fn definition(&self) -> DetectorDefinition {
        ShellExec.definition()
    }
    fn evaluate(&self, _: &Event) -> Result<Option<Detection>> {
        Err(Error::message("fixture detector failure"))
    }
}

#[tokio::test]
async fn collection_normalization_detector_close_and_parser_errors_propagate() {
    let source = Arc::new(TestCollector {
        events: vec![observation()],
        run_error: true,
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![json(Box::new(std::io::sink()), false)],
        options(vec![]),
    )
    .await;
    assert_eq!(stats.events_delivered, 1);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("fixture collection failure")
    );
    let mut raw = observation();
    raw.name = "unknown".into();
    let source = Arc::new(TestCollector {
        events: vec![raw],
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![json(Box::new(std::io::sink()), false)],
        options(vec![]),
    )
    .await;
    assert!(result.is_err());
    assert_eq!((stats.events_dropped, stats.events_normalized), (1, 0));
    let source = Arc::new(TestCollector {
        events: vec![observation()],
        ..TestCollector::default()
    });
    let mut opts = options(vec![]);
    opts.detectors = vec![Box::new(FailedDetector)];
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![json(Box::new(std::io::sink()), false)],
        opts,
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("fixture detector failure")
    );
    assert_eq!(stats.events_dropped, 1);
    let source = Arc::new(TestCollector {
        close_error: true,
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![json(Box::new(FailedCloser), true)],
        options(vec![]),
    )
    .await;
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("fixture collector close failure")
            && error.contains("fixture close failure")
    );
    assert_eq!(stats.destination_errors, 1);
    let source = Arc::new(Replay::new(Box::new(BytesInput::new("{}\n")), 65536).unwrap());
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![json(Box::new(std::io::sink()), false)],
        options(vec![]),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        (
            stats.parser_errors,
            stats.events_received,
            stats.events_normalized
        ),
        (1, 1, 0)
    );
}

#[tokio::test]
async fn partial_broadcast_is_not_delivered_and_invalid_queue_still_closes() {
    let source = Arc::new(TestCollector {
        events: vec![observation()],
        ..TestCollector::default()
    });
    let (stats, result) = pipeline::run(
        &CancellationToken::new(),
        source,
        vec![
            json(Box::new(std::io::sink()), false),
            json(Box::new(FailedWriter), false),
        ],
        options(vec![]),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        (
            stats.delivery_attempts,
            stats.events_delivered,
            stats.events_dropped,
            stats.destination_errors
        ),
        (2, 0, 1, 1)
    );
    let source = Arc::new(TestCollector::default());
    let mut opts = options(vec![]);
    opts.queue_size = 0;
    let (_, result) = pipeline::run(
        &CancellationToken::new(),
        source.clone(),
        vec![json(Box::new(std::io::sink()), false)],
        opts,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(source.closes.load(Ordering::Relaxed), 1);
}
