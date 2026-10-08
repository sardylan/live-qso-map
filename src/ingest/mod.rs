//! UDP ingestion: detect the format, parse, normalise and queue QSO events (FR-001 – FR-006).

pub mod detect;
pub mod gate;
pub mod identity;
pub mod qartest;

use std::net::SocketAddr;
use std::time::Instant;

use tokio::net::UdpSocket;
use tokio::sync::watch;

use crate::adif::AdifRecord;
use crate::adif::parse::parse_datagram;
use crate::band_filter::BandSet;
use crate::config::StationConfig;
use crate::history::qso_timestamp;
use crate::ingest::detect::{SourceFormat, detect};
use crate::ingest::identity::Identity;
use crate::pipeline::Pipeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    New,
    Correction,
}

/// What a parsed message means (data-model "QsoEvent").
#[derive(Debug, Clone)]
pub struct QsoEvent {
    pub kind: EventKind,
    /// Normalised, not yet enriched.
    pub record: AdifRecord,
    pub logger_id: Option<String>,
    /// QARTest `oldcall`/`oldtimestamp`; only used when there is no logger id.
    pub previous: Option<(String, String)>,
    pub identity: Identity,
    /// History id: the identity's canonical form, kept stable across corrections by the gate.
    pub id: String,
    pub rev: u32,
    pub received_at: Instant,
}

impl QsoEvent {
    pub fn new(
        kind: EventKind,
        record: AdifRecord,
        logger_id: Option<String>,
        station: &StationConfig,
    ) -> Self {
        let identity =
            Identity::from_record(&record, logger_id.as_deref(), station.callsign.as_deref());
        Self {
            kind,
            record,
            logger_id,
            previous: None,
            id: identity.canonical(),
            identity,
            rev: 1,
            received_at: Instant::now(),
        }
    }
}

/// Turns one datagram into QSO events. Malformed input is logged and skipped (FR-005).
pub fn events_from_datagram(
    data: &[u8],
    from: SocketAddr,
    station: &StationConfig,
) -> Vec<QsoEvent> {
    match detect(data) {
        SourceFormat::Adif => adif_events(data, from, station),
        SourceFormat::QarTest => qartest_events(data, from, station),
        SourceFormat::Unrecognised => {
            tracing::warn!(%from, len = data.len(), "dropping datagram in an unrecognised format");
            Vec::new()
        }
    }
}

fn adif_events(data: &[u8], from: SocketAddr, station: &StationConfig) -> Vec<QsoEvent> {
    let (records, errors) = parse_datagram(data);
    for error in errors {
        tracing::warn!(%from, "skipping malformed ADIF: {error}");
    }
    records
        .into_iter()
        .filter_map(|record| normalise(record, from, station))
        .map(|record| QsoEvent::new(EventKind::New, record, None, station))
        .collect()
}

fn qartest_events(data: &[u8], from: SocketAddr, station: &StationConfig) -> Vec<QsoEvent> {
    match qartest::parse(data) {
        Ok(Some(message)) => normalise(message.record, from, station)
            .map(|record| {
                let mut event = QsoEvent::new(message.kind, record, message.logger_id, station);
                event.previous = message.previous;
                event
            })
            .into_iter()
            .collect(),
        Ok(None) => Vec::new(),
        Err(e) => {
            tracing::warn!(%from, "skipping malformed QARTest message: {e}");
            Vec::new()
        }
    }
}

/// Checks the fields the pipeline needs and fills `STATION_CALLSIGN` from the configuration.
/// This is the only place that does it, because the identity needs it.
pub(crate) fn normalise(
    mut record: AdifRecord,
    from: SocketAddr,
    station: &StationConfig,
) -> Option<AdifRecord> {
    let date = record.get_non_empty("QSO_DATE");
    let time = record.get_non_empty("TIME_ON");
    if record.get_non_empty("CALL").is_none()
        || date.and_then(|d| qso_timestamp(d, time?)).is_none()
    {
        tracing::warn!(%from, "skipping QSO without a valid CALL, QSO_DATE and TIME_ON");
        return None;
    }
    if let Some(call) = &station.callsign {
        record.set_if_absent("STATION_CALLSIGN", call.clone());
    }
    // HR-3: a record with FREQ but no BAND gets the band from the frequency.
    if record.get_non_empty("BAND").is_none()
        && let Some(band) = record
            .get_non_empty("FREQ")
            .and_then(|f| f.parse::<f64>().ok())
            .and_then(crate::band::band_from_mhz)
    {
        record.set("BAND", band);
    }
    Some(record)
}

/// Whether a normalised record's band is considered (feature 003, FR-007). With a filter
/// active, an unknown band is not.
pub fn band_allowed(record: &AdifRecord, filter: Option<&BandSet>) -> bool {
    match filter {
        None => true,
        Some(set) => record
            .get_non_empty("BAND")
            .is_some_and(|band| set.contains(band)),
    }
}

/// Splits events into `(allowed, ignored)` by band.
pub fn split_by_band(
    events: Vec<QsoEvent>,
    filter: Option<&BandSet>,
) -> (Vec<QsoEvent>, Vec<QsoEvent>) {
    events
        .into_iter()
        .partition(|e| band_allowed(&e.record, filter))
}

/// Receives datagrams until shutdown and queues their events on channel 1.
pub async fn run(
    socket: UdpSocket,
    station: StationConfig,
    queue: async_channel::Sender<QsoEvent>,
    gate_capacity: usize,
    filter: Option<BandSet>,
    pipeline: Pipeline,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut gate = gate::ResendGate::new(gate_capacity);
    let mut buffer = vec![0u8; 65_535];
    loop {
        let received = tokio::select! {
            _ = shutdown.wait_for(|stop| *stop) => return,
            received = socket.recv_from(&mut buffer) => received,
        };
        let (len, from) = match received {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("udp receive failed: {e}");
                continue;
            }
        };
        let (allowed, ignored) = split_by_band(
            events_from_datagram(&buffer[..len], from, &station),
            filter.as_ref(),
        );
        for mut event in ignored {
            // A correction of a QSO the gate knows may be on the map: remove it (FR-008).
            if gate.contains(&event) && gate.check(&mut event) != gate::GateDecision::Drop {
                if pipeline.remove(&event.id, event.rev) {
                    tracing::info!(id = %event.id, "QSO corrected onto an ignored band; removed");
                }
                continue;
            }
            tracing::debug!(
                call = event.record.get("CALL").unwrap_or_default(),
                band = event.record.get("BAND").unwrap_or_default(),
                "dropping QSO on an ignored band"
            );
        }
        for mut event in allowed {
            if gate.check(&mut event) == gate::GateDecision::Drop {
                tracing::debug!(id = %event.id, "dropping an identical resend");
                continue;
            }
            if queue.send(event).await.is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from() -> SocketAddr {
        "127.0.0.1:9".parse().unwrap()
    }

    fn record(fields: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in [
            ("CALL", "K1A"),
            ("QSO_DATE", "20261008"),
            ("TIME_ON", "1500"),
        ]
        .iter()
        .chain(fields)
        {
            r.set(k, *v);
        }
        r
    }

    #[test]
    fn band_is_derived_from_freq_when_missing() {
        let r = normalise(
            record(&[("FREQ", "14.025")]),
            from(),
            &StationConfig::default(),
        )
        .unwrap();
        assert_eq!(r.get("BAND"), Some("20m"));
    }

    #[test]
    fn logger_band_is_never_overwritten() {
        let r = normalise(
            record(&[("FREQ", "14.025"), ("BAND", "40m")]),
            from(),
            &StationConfig::default(),
        )
        .unwrap();
        assert_eq!(r.get("BAND"), Some("40m"));
    }

    #[test]
    fn out_of_band_or_bad_freq_leaves_band_unknown() {
        for freq in ["5.000", "abc", ""] {
            let r =
                normalise(record(&[("FREQ", freq)]), from(), &StationConfig::default()).unwrap();
            assert_eq!(r.get("BAND"), None, "{freq}");
        }
    }

    #[test]
    fn station_callsign_default_is_filled() {
        let station = StationConfig {
            callsign: Some("IS0MZK".into()),
            ..StationConfig::default()
        };
        let r = normalise(record(&[]), from(), &station).unwrap();
        assert_eq!(r.get("STATION_CALLSIGN"), Some("IS0MZK"));
        assert!(normalise(AdifRecord::new(), from(), &station).is_none());
    }

    // --- band filter (feature 003, T007) ------------------------------------------------------

    fn contest() -> BandSet {
        crate::band_filter::resolve(&["hf-contest"]).unwrap()
    }

    #[test]
    fn no_filter_allows_everything() {
        assert!(band_allowed(&record(&[]), None));
        assert!(band_allowed(&record(&[("BAND", "2m")]), None));
    }

    #[test]
    fn filter_keeps_only_considered_bands() {
        let f = contest();
        assert!(band_allowed(&record(&[("BAND", "20m")]), Some(&f)));
        assert!(band_allowed(&record(&[("BAND", "20M")]), Some(&f)));
        assert!(!band_allowed(&record(&[("BAND", "2m")]), Some(&f)));
        assert!(!band_allowed(&record(&[("BAND", "60m")]), Some(&f)));
    }

    #[test]
    fn unknown_band_is_dropped_while_filtering() {
        let f = contest();
        assert!(!band_allowed(&record(&[]), Some(&f)));
        let out_of_band = normalise(
            record(&[("FREQ", "5.000")]),
            from(),
            &StationConfig::default(),
        )
        .unwrap();
        assert!(!band_allowed(&out_of_band, Some(&f)));
    }

    #[test]
    fn band_derived_from_freq_is_used() {
        let r = normalise(
            record(&[("FREQ", "14.025")]),
            from(),
            &StationConfig::default(),
        )
        .unwrap();
        assert!(band_allowed(&r, Some(&contest())));
    }

    #[test]
    fn split_by_band_separates_events() {
        let station = StationConfig::default();
        let data = b"<CALL:4>K1AA <QSO_DATE:8>20261009 <TIME_ON:4>1500 <BAND:3>20m <EOR><CALL:4>K2BB <QSO_DATE:8>20261009 <TIME_ON:4>1501 <BAND:2>2m <EOR>";
        let events = events_from_datagram(data, from(), &station);
        assert_eq!(events.len(), 2);
        let (allowed, ignored) = split_by_band(events, Some(&contest()));
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].record.get("CALL"), Some("K1AA"));
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].record.get("CALL"), Some("K2BB"));
        let (allowed, ignored) = split_by_band(events_from_datagram(data, from(), &station), None);
        assert_eq!((allowed.len(), ignored.len()), (2, 0));
    }
}
