//! Enrichment workers: station position, remote position, then publish (plan "Summary").

pub mod cty;
pub mod provider;
pub mod resolve;
pub mod station;

use std::sync::Arc;

use tokio::sync::watch;

use crate::config::Config;
use crate::history::{EnrichedQso, qso_timestamp};
use crate::ingest::QsoEvent;
use crate::pipeline::{LookupSource, Pipeline};
use crate::position::PositionSource;

/// Lookup sources plus their tie-break order (the configured providers, then `cty`).
#[derive(Clone)]
pub struct Lookups {
    pub sources: Arc<Vec<LookupSource>>,
    pub order: Arc<Vec<PositionSource>>,
}

impl Lookups {
    pub fn new(sources: Vec<LookupSource>, config: &Config) -> Self {
        let mut order: Vec<PositionSource> = config
            .lookup
            .provider_order
            .iter()
            .map(|kind| match kind {
                crate::config::ProviderKind::Qrz => PositionSource::Qrz,
                crate::config::ProviderKind::HamQth => PositionSource::HamQth,
                crate::config::ProviderKind::QrzCq => PositionSource::QrzCq,
            })
            .collect();
        order.push(PositionSource::Cty);
        Self {
            sources: Arc::new(sources),
            order: Arc::new(order),
        }
    }
}

/// Builds the enriched QSO for one event: station position, then the logger's remote position,
/// else the best lookup result (FR-011 – FR-019).
pub async fn enrich(event: QsoEvent, config: &Config, lookups: &Lookups) -> EnrichedQso {
    let mut record = event.record;
    let station = station::apply_station(&mut record, &config.station);
    let mut remote = resolve::logger_position(&record);
    if remote.is_none() && !lookups.sources.is_empty() {
        let call = record.get("CALL").unwrap_or_default().trim().to_string();
        let (position, fields) = resolve::resolve_remote(
            &call,
            &lookups.sources,
            &lookups.order,
            config.lookup.timeout,
        )
        .await;
        for (name, value) in fields.iter() {
            record.set_if_absent(name, value);
        }
        remote = position;
    }
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
    lookups: Lookups,
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
        let qso = enrich(event, &config, &lookups).await;
        if !pipeline.publish(qso) {
            tracing::debug!("dropping a stale QSO version");
        }
    }
}
