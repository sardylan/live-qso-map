#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use live_qso_map::config::Config;
use live_qso_map::pipeline::HistoryChange;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

/// A configuration bound to localhost with ephemeral ports and no lookups.
pub fn test_config() -> Config {
    let mut config = Config {
        bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
        udp_port: 0,
        http_port: 0,
        ..Config::default()
    };
    config.station.callsign = Some("IS0MZK".into());
    config.station.locator = Some("JM49NH".into());
    config.country_data.path = "/nonexistent/cty.dat".into();
    // A closed local port: tests never download from the Internet.
    config.country_data.url = "http://127.0.0.1:9/cty.dat".into();
    config.country_data.max_age_days = 0;
    // Integration tests don't depend on the default bands; feature 011 tests the default in
    // T031–T032.
    config.band_filter = None;
    config
}

pub async fn send_udp(to: SocketAddr, payload: &[u8]) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(payload, to).await.unwrap();
}

/// Receives the next upsert within `within`, or `None`.
pub async fn next_upsert(
    rx: &mut broadcast::Receiver<HistoryChange>,
    within: Duration,
) -> Option<std::sync::Arc<live_qso_map::history::EnrichedQso>> {
    match tokio::time::timeout(within, rx.recv()).await {
        Ok(Ok(HistoryChange::Upsert(q))) => Some(q),
        _ => None,
    }
}

/// A fresh, empty folder under the system temporary folder, unique to this process and call
/// (feature 014).
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("lqm-{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every record of an ADIF file, parsed like the service parses it.
pub fn read_records(path: &std::path::Path) -> Vec<live_qso_map::adif::AdifRecord> {
    let data = std::fs::read(path).unwrap();
    live_qso_map::adif::parse::parse_datagram(&data).0
}

/// How a fake provider answers one callsign.
#[derive(Clone)]
pub enum Answer {
    Found(live_qso_map::position::LookupCandidate),
    NotFound,
    Fail,
    /// Waits until the notify is released, then answers "not found".
    Hang(std::sync::Arc<tokio::sync::Notify>),
}

/// A fake online provider: per-call answers (default "not found") and a call counter.
#[derive(Clone)]
pub struct FakeProvider {
    pub source: live_qso_map::position::PositionSource,
    pub answers: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Answer>>>,
    pub calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl FakeProvider {
    pub fn new(source: live_qso_map::position::PositionSource) -> Self {
        Self {
            source,
            answers: Default::default(),
            calls: Default::default(),
        }
    }

    /// Answers `call` with a position of the given precision at `lat`/`lon`.
    pub fn found(
        self,
        call: &str,
        lat: f64,
        lon: f64,
        precision: live_qso_map::position::Precision,
    ) -> Self {
        let candidate = live_qso_map::position::LookupCandidate {
            source: self.source,
            position: Some(
                live_qso_map::position::Position::new(lat, lon, precision, self.source).unwrap(),
            ),
            fields: live_qso_map::adif::AdifRecord::new(),
        };
        self.answer(call, Answer::Found(candidate))
    }

    pub fn answer(self, call: &str, answer: Answer) -> Self {
        self.answers
            .lock()
            .unwrap()
            .insert(call.to_ascii_uppercase(), answer);
        self
    }

    pub fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn source(&self) -> live_qso_map::pipeline::LookupSource {
        let this = self.clone();
        std::sync::Arc::new(move |call: String| {
            this.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let answer = this
                .answers
                .lock()
                .unwrap()
                .get(&call.to_ascii_uppercase())
                .cloned()
                .unwrap_or(Answer::NotFound);
            let provider = this.source.as_str();
            Box::pin(async move {
                match answer {
                    Answer::Found(candidate) => Ok(Some(candidate)),
                    Answer::NotFound => Ok(None),
                    Answer::Fail => Err(live_qso_map::errors::Error::Provider {
                        provider,
                        message: "fake failure".into(),
                    }),
                    Answer::Hang(notify) => {
                        notify.notified().await;
                        Ok(None)
                    }
                }
            })
        })
    }
}
