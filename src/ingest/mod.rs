//! UDP ingestion: detect the format, parse, normalise and queue QSO events (FR-001 – FR-006).

pub mod detect;
pub mod gate;
pub mod identity;

use std::net::SocketAddr;
use std::time::Instant;

use tokio::net::UdpSocket;
use tokio::sync::watch;

use crate::adif::AdifRecord;
use crate::adif::parse::parse_datagram;
use crate::config::StationConfig;
use crate::history::qso_timestamp;
use crate::ingest::detect::{SourceFormat, detect};
use crate::ingest::identity::Identity;

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
        SourceFormat::QarTest => Vec::new(),
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
    Some(record)
}

/// Receives datagrams until shutdown and queues their events on channel 1.
pub async fn run(
    socket: UdpSocket,
    station: StationConfig,
    queue: async_channel::Sender<QsoEvent>,
    gate_capacity: usize,
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
        for mut event in events_from_datagram(&buffer[..len], from, &station) {
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
