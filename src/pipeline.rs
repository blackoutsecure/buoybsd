// SPDX-License-Identifier: Apache-2.0

use crate::{
    domain::{Detector, PolicySet},
    error::{Error, Result, join},
    normalize::normalize,
    output::Destination,
    replay::Collector,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct Options {
    pub queue_size: usize,
    pub policies: PolicySet,
    pub detectors: Vec<Box<dyn Detector>>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct Stats {
    pub events_received: u64,
    pub events_normalized: u64,
    pub events_filtered: u64,
    pub detections_generated: u64,
    pub events_delivered: u64,
    pub events_dropped: u64,
    pub parser_errors: u64,
    pub destination_errors: u64,
    pub delivery_attempts: u64,
    pub collector_restarts: u64,
}

pub async fn run(
    parent: &CancellationToken,
    source: Arc<dyn Collector>,
    mut destinations: Vec<Box<dyn Destination>>,
    options: Options,
) -> (Stats, Result<()>) {
    let mut stats = Stats::default();
    let mut result = Ok(());
    if !(1..=4096).contains(&options.queue_size) || destinations.is_empty() {
        result = Err(Error::message(
            "pipeline requires a bounded queue, compiled policies, and destinations",
        ));
    } else {
        let cancel = parent.child_token();
        let (sender, mut events) = mpsc::channel(options.queue_size);
        let collector = Arc::clone(&source);
        let collection_cancel = cancel.clone();
        let completed = tokio::spawn(async move { collector.run(collection_cancel, sender).await });
        'processing: loop {
            let raw = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    result = Err(Error::Cancelled);
                    break;
                }
                raw = events.recv() => {
                    let Some(raw) = raw else { break; };
                    raw
                }
            };
            let mut event = match normalize(&raw) {
                Ok(event) => event,
                Err(error) => {
                    stats.events_dropped += 1;
                    result = Err(error);
                    break;
                }
            };
            stats.events_normalized += 1;
            let (retain, matched) = options.policies.matches(&event);
            if !retain {
                stats.events_filtered += 1;
                continue;
            }
            event.matched_policies = matched;
            for detector in &options.detectors {
                if !detector
                    .definition()
                    .event_selectors
                    .contains(&event.name.as_str())
                {
                    continue;
                }
                match detector.evaluate(&event) {
                    Ok(Some(detection)) => {
                        event.detections.push(detection);
                        stats.detections_generated += 1;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        stats.events_dropped += 1;
                        result = Err(error.context("evaluate detector"));
                        break 'processing;
                    }
                }
            }
            for destination in &mut destinations {
                if let Err(error) = destination.write_event(&cancel, &event).await {
                    stats.events_dropped += 1;
                    result = Err(error.context(format!("deliver to {:?}", destination.name())));
                    break 'processing;
                }
            }
            stats.events_delivered += 1;
        }
        cancel.cancel();
        let collection = completed
            .await
            .unwrap_or_else(|_| Err(Error::message("collector task failed")));
        let downstream_cancellation = result.is_err()
            && !parent.is_cancelled()
            && collection.as_ref().is_err_and(Error::is_cancelled);
        if !downstream_cancellation {
            join(&mut result, collection);
        }
        while events.try_recv().is_ok() {
            stats.events_dropped += 1;
        }
    }
    join(
        &mut result,
        source
            .close()
            .await
            .map_err(|e| e.context("close collector")),
    );
    let observed = source.stats();
    stats.events_received = observed.received;
    stats.events_dropped += observed.dropped;
    stats.parser_errors = observed.parser_errors;
    stats.collector_restarts = observed.restarts;
    for destination in &mut destinations {
        if let Err(error) = destination.close().await {
            stats.destination_errors += 1;
            join(
                &mut result,
                Err(error.context(format!("close destination {:?}", destination.name()))),
            );
        }
        let observed = destination.stats();
        stats.destination_errors += observed.errors;
        stats.delivery_attempts += observed.attempts;
    }
    (stats, result)
}
