//! Enrichment workers: station position, remote position, then publish (plan "Summary").

pub mod resolve;
pub mod station;

use std::sync::Arc;

use tokio::sync::watch;

use crate::config::Config;
use crate::history::{EnrichedQso, qso_timestamp};
use crate::ingest::QsoEvent;
use crate::pipeline::Pipeline;

/// Builds the enriched QSO for one event.
pub fn enrich(event: QsoEvent, config: &Config) -> EnrichedQso {
    let mut record = event.record;
    let station = station::apply_station(&mut record, &config.station);
    let remote = resolve::logger_position(&record);
    match &remote {
        Some(position) => resolve::write_position_fields(&mut record, position),
        None => tracing::warn!(
            call = record.get("CALL").unwrap_or_default(),
            "no position found; the QSO is kept but not drawn"
        ),
    }
    let sort_ts = record
        .get("QSO_DATE")
        .zip(record.get("TIME_ON"))
        .and_then(|(d, t)| qso_timestamp(d, t))
        .unwrap_or_default();
    EnrichedQso {
        id: event.identity.canonical(),
        rev: event.rev,
        sort_ts,
        record,
        remote,
        station,
    }
}

/// One worker: takes events from channel 1 until it closes or shutdown is signalled.
pub async fn run_worker(
    queue: async_channel::Receiver<QsoEvent>,
    pipeline: Pipeline,
    config: Arc<Config>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        let event = tokio::select! {
            _ = shutdown.wait_for(|stop| *stop) => return,
            event = queue.recv() => match event {
                Ok(event) => event,
                Err(_) => return,
            },
        };
        let qso = enrich(event, &config);
        if !pipeline.publish(qso) {
            tracing::debug!("dropping a stale QSO version");
        }
    }
}
