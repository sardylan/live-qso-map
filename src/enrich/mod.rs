//! Enrichment workers: station position, remote position, then publish (plan "Summary").

pub mod acceptance;
pub mod cty;
pub mod provider;
pub mod resolve;
pub mod station;

use std::sync::Arc;

use tokio::sync::watch;

use crate::config::Config;
use crate::enrich::cty::CountryData;
use crate::entity_extents::ExtentTable;
use crate::history::{EnrichedQso, QsoEntity, qso_timestamp};
use crate::ingest::QsoEvent;
use crate::pipeline::{LookupSource, Pipeline};
use crate::position::{Position, PositionSource};

/// Lookup sources plus their tie-break order (the configured providers, then `cty`), and the
/// country data that names each QSO's entity (feature 005).
#[derive(Clone)]
pub struct Lookups {
    pub sources: Arc<Vec<LookupSource>>,
    pub order: Arc<Vec<PositionSource>>,
    pub country: Option<Arc<CountryData>>,
    /// Entity rectangles, to judge remote positions (feature 010, research R5).
    pub areas: Arc<ExtentTable>,
}

impl Lookups {
    pub fn new(
        sources: Vec<LookupSource>,
        country: Option<Arc<CountryData>>,
        areas: Arc<ExtentTable>,
        config: &Config,
    ) -> Self {
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
            country,
            areas,
        }
    }
}

/// Builds the enriched QSO for one event: station position, then the logger's remote position,
/// else the best acceptable lookup result (FR-011 – FR-019; feature 010).
pub async fn enrich(event: QsoEvent, config: &Config, lookups: &Lookups) -> EnrichedQso {
    let mut record = event.record;
    let station = station::apply_station(&mut record, &config.station);
    let call = record.get("CALL").unwrap_or_default().trim().to_string();
    // The entity always comes from cty.dat for the call as logged, whatever gave the position
    // (feature 005, FR-002); feature 010 judges remote positions against it.
    let entity = lookups.country.as_ref().and_then(|country| {
        let info = country.entity_info(&call)?;
        Some(QsoEntity {
            prefix: info.prefix.to_string(),
            name: info.name.to_string(),
            cq_zone: info.cq_zone,
        })
    });
    let prefix = entity.as_ref().map(|e| e.prefix.as_str());
    // (source, found through the base callsign, reason), in the order they happened.
    let mut rejections: Vec<(&'static str, bool, acceptance::RejectReason)> = Vec::new();

    let mut remote = resolve::logger_position(&record);
    if remote.is_none() && !lookups.sources.is_empty() {
        let accept = |position: &Position| {
            let online = position.source != PositionSource::Cty;
            acceptance::check(position, prefix, online, &lookups.areas)
        };
        let lookup = resolve::resolve_remote(
            &call,
            &lookups.sources,
            &lookups.order,
            config.lookup.timeout,
            &accept,
        )
        .await;
        for (source, reason) in &lookup.rejections {
            rejections.push((source.as_str(), false, *reason));
        }
        let (position, fields) = resolve::select(&lookup.candidates, &lookups.order);
        for (name, value) in fields.iter() {
            record.set_if_absent(name, value);
        }
        remote = position;
    }
    for (source, base, reason) in &rejections {
        tracing::warn!(
            call,
            source,
            reason = reason.as_str(),
            round = if *base { "base" } else { "logged" },
            "position rejected"
        );
    }
    if let Some(note) = acceptance::rejection_note(&rejections) {
        record.set("APP_LIVEQSOMAP_POS_REJECTED", note);
    }
    match &remote {
        Some(position) => resolve::write_position_fields(&mut record, position),
        None => tracing::warn!(call, "no position found; the QSO is kept but not drawn"),
    }
    let sort_ts = record
        .get("QSO_DATE")
        .zip(record.get("TIME_ON"))
        .and_then(|(d, t)| qso_timestamp(d, t))
        .unwrap_or_default();
    EnrichedQso {
        id: event.id,
        rev: event.rev,
        sort_ts,
        record,
        remote,
        station,
        entity,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adif::AdifRecord;
    use crate::enrich::cty::CountryData;
    use crate::history::QsoEntity;
    use crate::ingest::{EventKind, QsoEvent};

    const CTY: &str = include_str!("../../tests/fixtures/cty-excerpt.dat");

    fn lookups(country: Option<Arc<CountryData>>, config: &Config) -> Lookups {
        let areas = Arc::new(ExtentTable::embedded().expect("embedded entity extents"));
        Lookups::new(Vec::new(), country, areas, config)
    }

    fn event(call: &str, extra: &[(&str, &str)]) -> QsoEvent {
        let mut record = AdifRecord::new();
        for (k, v) in [
            ("CALL", call),
            ("QSO_DATE", "20261009"),
            ("TIME_ON", "1504"),
            ("BAND", "20m"),
            ("MODE", "CW"),
        ]
        .iter()
        .chain(extra)
        {
            record.set(k, *v);
        }
        QsoEvent::new(EventKind::New, record, None, &Default::default())
    }

    async fn entity_of(
        call: &str,
        extra: &[(&str, &str)],
        with_country: bool,
    ) -> Option<(String, String)> {
        let config = Config::default();
        let country = with_country.then(|| Arc::new(CountryData::parse(CTY)));
        enrich(event(call, extra), &config, &lookups(country, &config))
            .await
            .entity
            .map(|e| (e.prefix, e.name))
    }

    /// Prefix and name only; the CQ zone has its own tests.
    fn entity(prefix: &str, name: &str) -> Option<(String, String)> {
        Some((prefix.into(), name.into()))
    }

    fn named(e: Option<QsoEntity>) -> Option<(String, String)> {
        e.map(|e| (e.prefix, e.name))
    }

    #[tokio::test]
    async fn enrich_fills_the_cty_entity() {
        assert_eq!(
            entity_of("K1ABC", &[], true).await,
            entity("K", "United States")
        );
        assert_eq!(
            entity_of("is0mzk", &[], true).await,
            entity("IS", "Sardinia")
        );
        // WAE: the `*` of `*IT9` is not part of the prefix.
        assert_eq!(
            entity_of("IT9ABC", &[], true).await,
            entity("IT9", "Sicily")
        );
        assert_eq!(
            entity_of("IS0MZK/P", &[], true).await,
            entity("IS", "Sardinia")
        );
        // Exact-call entries win over the prefix.
        assert_eq!(entity_of("IT9QZW/4", &[], true).await, entity("I", "Italy"));
        assert_eq!(
            entity_of("KH6QZ", &[], true).await,
            entity("K", "United States")
        );
        // Prefixed call.
        assert_eq!(
            entity_of("KH6/W1XX", &[], true).await,
            entity("KH6", "Hawaii")
        );
        assert_eq!(entity_of("QQ9QQ", &[], true).await, None);
    }

    #[tokio::test]
    async fn enrich_fills_the_cq_zone_and_updates_it_on_corrections() {
        let config = Config::default();
        let l = lookups(Some(Arc::new(CountryData::parse(CTY))), &config);
        let first = enrich(event("IS0MZK", &[]), &config, &l).await;
        assert_eq!(first.entity.as_ref().map(|e| e.cq_zone), Some(15));
        let mut correction = event("K1ABC", &[]);
        correction.kind = EventKind::Correction;
        correction.id = first.id.clone();
        correction.rev = first.rev + 1;
        let second = enrich(correction, &config, &l).await;
        assert_eq!(second.entity.as_ref().map(|e| e.cq_zone), Some(5));
    }

    #[tokio::test]
    async fn no_country_data_means_no_entity() {
        assert_eq!(entity_of("K1ABC", &[], false).await, None);
    }

    #[tokio::test]
    async fn the_entity_does_not_depend_on_the_position_source() {
        // The logger gives the position; the entity still comes from cty.dat (FR-002).
        let config = Config::default();
        let l = lookups(Some(Arc::new(CountryData::parse(CTY))), &config);
        let q = enrich(event("K1ABC", &[("GRIDSQUARE", "FN42")]), &config, &l).await;
        assert_eq!(
            q.remote.unwrap().source,
            crate::position::PositionSource::Logger
        );
        assert_eq!(named(q.entity), entity("K", "United States"));
    }

    #[tokio::test]
    async fn a_corrected_callsign_gets_its_own_entity() {
        let config = Config::default();
        let l = lookups(Some(Arc::new(CountryData::parse(CTY))), &config);
        let first = enrich(event("IS0MZK", &[]), &config, &l).await;
        let mut correction = event("K1ABC", &[]);
        correction.kind = EventKind::Correction;
        correction.id = first.id.clone();
        correction.rev = first.rev + 1;
        let second = enrich(correction, &config, &l).await;
        assert_eq!(named(first.entity), entity("IS", "Sardinia"));
        assert_eq!(named(second.entity), entity("K", "United States"));
    }

    #[test]
    fn entity_matches_the_cty_lookup_for_30_callsigns() {
        // SC-001: ordinary, portable, prefixed, exact-call and WAE callsigns.
        let data = CountryData::parse(CTY);
        let calls = [
            "BY1AA",
            "BA4XX",
            "DL1ABC",
            "DK2XY",
            "HB9ABC",
            "I2XYZ",
            "IK0ABC",
            "IZ8XX",
            "IS0MZK",
            "IM0ABC",
            "IT9ABC",
            "K1ABC",
            "W1XX",
            "N2XX",
            "AA1A",
            "KH6XX",
            "DL1ABC/P",
            "K1ABC/M",
            "IS0MZK/P",
            "I2XYZ/QRP",
            "KH6/W1XX",
            "DL/K1ABC",
            "HB9/DL1ABC",
            "IS0/I2XYZ",
            "I/DL1ABC",
            "IT9QZW/4",
            "KH6QZ",
            "I2QXJ/RM",
            "IA5QZX/4",
            "IT9XYZ/P",
        ];
        assert_eq!(calls.len(), 30);
        for call in calls {
            let by_entity = data.entity_info(call).map(|e| e.name.to_string());
            let by_lookup = data
                .lookup(call)
                .and_then(|c| c.fields.get("COUNTRY").map(str::to_string));
            assert!(by_entity.is_some(), "{call} resolves");
            assert_eq!(by_entity, by_lookup, "{call}");
        }
    }
}
