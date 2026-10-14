//! UDP ingestion: detect the format, parse, normalise and queue QSO events (FR-001 – FR-006).

pub mod detect;
pub mod gate;
pub mod identity;
pub mod qartest;
pub mod score;

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
use crate::ingest::qartest::QarTestItem;
use crate::ingest::score::ContestScore;
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
    /// The record as saved in the history file, when it is reused without a lookup or parked
    /// (feature 014, R4, R8). `record` is always the record as received.
    pub saved: Option<AdifRecord>,
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
            saved: None,
        }
    }
}

/// What one datagram carries: QSO events, and a contest score (feature 007, research R2).
#[derive(Debug, Default)]
pub struct Ingested {
    pub events: Vec<QsoEvent>,
    pub score: Option<ContestScore>,
}

/// Turns one datagram into QSO events and a score. Malformed input is logged and skipped (FR-005).
pub fn events_from_datagram(data: &[u8], from: SocketAddr, station: &StationConfig) -> Ingested {
    match detect(data) {
        SourceFormat::Adif => Ingested {
            events: adif_events(data, from, station),
            score: None,
        },
        SourceFormat::QarTest => qartest_items(data, from, station),
        SourceFormat::Unrecognised => {
            tracing::warn!(%from, len = data.len(), "dropping datagram in an unrecognised format");
            Ingested::default()
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

fn qartest_items(data: &[u8], from: SocketAddr, station: &StationConfig) -> Ingested {
    match qartest::parse(data) {
        Ok(Some(QarTestItem::Qso(message))) => Ingested {
            events: normalise(message.record, from, station)
                .map(|record| {
                    let mut event = QsoEvent::new(message.kind, record, message.logger_id, station);
                    event.previous = message.previous;
                    event
                })
                .into_iter()
                .collect(),
            score: None,
        },
        Ok(Some(QarTestItem::Score(score))) => Ingested {
            events: Vec::new(),
            score: Some(score),
        },
        Ok(None) => Ingested::default(),
        Err(e) => {
            tracing::warn!(%from, "skipping malformed QARTest message: {e}");
            Ingested::default()
        }
    }
}

/// Checks the fields the pipeline needs and fills `STATION_CALLSIGN` from the configuration.
/// This is the only place that does it, because the identity needs it.
pub(crate) fn normalise(
    mut record: AdifRecord,
    from: impl std::fmt::Display,
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

/// The records of the history file as QSO events (feature 014, R2–R8), in file order.
#[derive(Debug, Default)]
pub struct Restored {
    /// Events to queue, each with the record as loaded (written back if the service stops
    /// before a worker is done with it). `saved` is set for those reused without a lookup.
    pub events: Vec<(QsoEvent, AdifRecord)>,
    /// Records on bands not considered (service only), `saved` holding the verbatim record.
    pub parked: Vec<QsoEvent>,
    pub report: crate::history_file::LoadReport,
}

/// Turns loaded records into events like a datagram's: `normalise` on the record as received,
/// the identity with `APP_LIVEQSOMAP_LOGGER_ID` as logger id (R5), and the restore action.
/// `source` names the file in log lines.
pub fn restored_events(
    records: Vec<AdifRecord>,
    station: &StationConfig,
    mode: crate::history_file::restore::Mode,
    filter: Option<&BandSet>,
    source: &str,
) -> Restored {
    use crate::history_file::restore::{RestoreAction, as_received, classify};
    let mut restored = Restored::default();
    for saved in records {
        let action = classify(&saved, mode, filter);
        let normalised = match action {
            RestoreAction::Skip => None,
            _ => normalise(as_received(&saved), source, station),
        };
        let Some(record) = normalised else {
            restored.report.skipped += 1;
            continue;
        };
        let logger_id = record
            .get_non_empty(crate::history_file::LOGGER_ID_FIELD)
            .map(str::to_string);
        let mut event = QsoEvent::new(EventKind::New, record, logger_id, station);
        restored.report.loaded += 1;
        match action {
            RestoreAction::Reuse => {
                restored.report.reused += 1;
                event.saved = Some(saved.clone());
                restored.events.push((event, saved));
            }
            RestoreAction::Park => {
                restored.report.parked += 1;
                event.saved = Some(saved);
                restored.parked.push(event);
            }
            RestoreAction::Enrich | RestoreAction::Skip => {
                restored.report.to_enrich += 1;
                restored.events.push((event, saved));
            }
        }
    }
    restored
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

/// Receives datagrams until shutdown, queues their events on channel 1 and publishes scores.
/// `gate` already knows the QSOs restored from the history file (feature 014, R2).
pub async fn run(
    socket: UdpSocket,
    station: StationConfig,
    queue: async_channel::Sender<QsoEvent>,
    mut gate: gate::ResendGate,
    filter: Option<BandSet>,
    pipeline: Pipeline,
    mut shutdown: watch::Receiver<bool>,
) {
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
        let ingested = events_from_datagram(&buffer[..len], from, &station);
        // The logger's own contest total: no band filter, and no resend gate, because each
        // repeat refreshes the time of the last update (feature 007, research R2).
        if let Some(score) = ingested.score {
            pipeline.publish_score(score);
        }
        let (allowed, ignored) = split_by_band(ingested.events, filter.as_ref());
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
            pipeline.pending_insert(&event.id, event.rev, event.record.clone());
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
        let events = events_from_datagram(data, from(), &station).events;
        assert_eq!(events.len(), 2);
        let (allowed, ignored) = split_by_band(events, Some(&contest()));
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].record.get("CALL"), Some("K1AA"));
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].record.get("CALL"), Some("K2BB"));
        let (allowed, ignored) =
            split_by_band(events_from_datagram(data, from(), &station).events, None);
        assert_eq!((allowed.len(), ignored.len()), (2, 0));
    }

    // --- contest score (feature 007, T004) ----------------------------------------------------

    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

    fn reference(i: usize) -> Vec<u8> {
        qartest::reference_packets(include_str!(
            "../../specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt"
        ))[i]
            .clone()
    }

    /// Records the message of every WARN event.
    struct Warnings(Arc<Mutex<Vec<String>>>);

    struct Message(String);

    impl Visit for Message {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }

    impl<S: tracing::Subscriber> Layer<S> for Warnings {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            if *event.metadata().level() == tracing::Level::WARN {
                let mut message = Message(String::new());
                event.record(&mut message);
                self.0.lock().unwrap().push(message.0);
            }
        }
    }

    #[test]
    fn a_score_datagram_gives_a_score_and_no_events() {
        let ingested = events_from_datagram(&reference(3), from(), &StationConfig::default());
        assert!(ingested.events.is_empty());
        assert_eq!(
            ingested.score,
            Some(ContestScore {
                score: 90,
                qsos: 6,
                mults: 10
            })
        );
    }

    #[test]
    fn qso_datagrams_give_no_score() {
        let station = StationConfig::default();
        let qartest = events_from_datagram(&reference(0), from(), &station);
        assert_eq!(qartest.events.len(), 1);
        assert_eq!(qartest.score, None);
        let adif = events_from_datagram(
            b"<CALL:4>K1AA <QSO_DATE:8>20261009 <TIME_ON:4>1500 <EOR>",
            from(),
            &station,
        );
        assert_eq!(adif.events.len(), 1);
        assert_eq!(adif.score, None);
    }

    #[test]
    fn a_malformed_score_is_skipped_and_logged() {
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(Warnings(warnings.clone()));
        let data = String::from_utf8(reference(3))
            .unwrap()
            .replace("<score>90</score>", "<score>abc</score>");
        let ingested = tracing::subscriber::with_default(subscriber, || {
            events_from_datagram(data.as_bytes(), from(), &StationConfig::default())
        });
        assert!(ingested.events.is_empty());
        assert_eq!(ingested.score, None);
        let warnings = warnings.lock().unwrap();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("<score>"), "{}", warnings[0]);
    }

    // --- restored records (feature 014, T022) ------------------------------------------------

    fn saved_record(call: &str, extra: &[(&str, &str)]) -> AdifRecord {
        let mut r = record(&[("BAND", "20m"), ("MODE", "CW")]);
        r.set("CALL", call);
        for (k, v) in extra {
            r.set(k, *v);
        }
        r
    }

    #[test]
    fn restored_records_become_events_with_their_action() {
        use crate::history_file::restore::Mode;
        let station = StationConfig {
            callsign: Some("IS0MZK".into()),
            ..StationConfig::default()
        };
        let reused = saved_record(
            "K2ABC",
            &[
                ("LAT", "N040 00.000"),
                ("LON", "W075 00.000"),
                ("APP_LIVEQSOMAP_POS_SOURCE", "qrz"),
                ("APP_LIVEQSOMAP_POS_PRECISION", "locator6"),
                (
                    crate::history_file::ADDED_FIELD,
                    "APP_LIVEQSOMAP_POS_PRECISION,APP_LIVEQSOMAP_POS_SOURCE,LAT,LON",
                ),
            ],
        );
        let qartest = saved_record(
            "K6QA",
            &[(crate::history_file::LOGGER_ID_FIELD, "3809734F4")],
        );
        let vhf = saved_record("IS0AAA", &[("BAND", "2m")]);
        let mut invalid = saved_record("DL2XX", &[]);
        invalid.remove("TIME_ON");
        let restored = restored_events(
            vec![reused.clone(), qartest, vhf.clone(), invalid],
            &station,
            Mode::Service,
            Some(&contest()),
            "test.adi",
        );
        assert_eq!(restored.events.len(), 2);
        assert_eq!(restored.parked.len(), 1);
        assert_eq!(
            restored.report,
            crate::history_file::LoadReport {
                loaded: 3,
                skipped: 1,
                reused: 1,
                to_enrich: 1,
                parked: 1,
            }
        );

        let (first, loaded) = &restored.events[0];
        assert_eq!(loaded, &reused);
        assert_eq!(
            first.saved.as_ref(),
            Some(&reused),
            "Reuse keeps the saved record"
        );
        assert_eq!(
            first.record.get("LAT"),
            None,
            "the event carries the record as received"
        );
        assert_eq!(first.record.get("STATION_CALLSIGN"), Some("IS0MZK"));

        let (second, loaded) = &restored.events[1];
        assert_eq!(second.saved, None, "Enrich");
        assert_eq!(
            loaded.get(crate::history_file::LOGGER_ID_FIELD),
            Some("3809734F4")
        );
        assert_eq!(second.logger_id.as_deref(), Some("3809734F4"));
        assert!(second.id.starts_with("L|"), "{}", second.id);

        assert_eq!(
            restored.parked[0].saved.as_ref(),
            Some(&vhf),
            "parked verbatim"
        );
    }
}
