#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{send_udp, test_config};
use live_qso_map::enrich::cty::CountryData;
use live_qso_map::ingest::qartest::reference_packets;
use live_qso_map::pipeline::{cty_source, start};
use live_qso_map::position::{PositionSource, Precision};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

const REFERENCE: &str =
    include_str!("../specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt");
const CTY: &str = include_str!("fixtures/cty-excerpt.dat");

/// Counts WARN and ERROR events from every thread.
struct WarnCounter(Arc<AtomicUsize>);

impl<S: Subscriber> Layer<S> for WarnCounter {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() <= Level::WARN {
            eprintln!("warning event: {event:?}");
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[tokio::test]
async fn reference_capture_gives_sc_009() {
    let warnings = Arc::new(AtomicUsize::new(0));
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(WarnCounter(warnings.clone())),
    )
    .unwrap();

    let sources = vec![cty_source(Arc::new(CountryData::parse(CTY)))];
    let service = start(test_config(), Some(sources)).await.unwrap();
    for packet in reference_packets(REFERENCE) {
        send_udp(service.udp_addr, &packet).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Packet 10 is the last score in the capture (feature 007).
    let score = service.pipeline().latest_score().expect("a score");
    assert_eq!(
        (score.score.score, score.score.qsos, score.score.mults),
        (132, 7, 11)
    );

    let history = service.pipeline().lock_history().latest(None);
    let mut calls: Vec<_> = history
        .iter()
        .map(|q| q.record.get("CALL").unwrap().to_string())
        .collect();
    calls.sort();
    assert_eq!(calls, ["B5Q", "DL5QFN", "HB9FQZ", "K2WQX", "K6QA"]);

    let by_call = |call: &str| {
        history
            .iter()
            .find(|q| q.record.get("CALL") == Some(call))
            .unwrap()
            .clone()
    };
    let k6qa = by_call("K6QA");
    assert_eq!(k6qa.rev, 3);
    assert_eq!(k6qa.record.get("SRX_STRING"), Some("3"));
    let hb9 = by_call("HB9FQZ");
    assert_eq!(hb9.rev, 2);
    assert_eq!(hb9.record.get("SRX_STRING"), Some("14"));

    for (call, country) in [
        ("DL5QFN", "Fed. Rep. of Germany"),
        ("B5Q", "China"),
        ("K6QA", "United States"),
        ("K2WQX", "United States"),
        ("HB9FQZ", "Switzerland"),
    ] {
        let q = by_call(call);
        let remote = q.remote.unwrap();
        assert_eq!(remote.precision, Precision::Country, "{call}");
        assert_eq!(remote.source, PositionSource::Cty, "{call}");
        assert_eq!(q.record.get("COUNTRY"), Some(country), "{call}");
    }

    assert_eq!(warnings.load(Ordering::SeqCst), 0, "no warnings (SC-009)");
    service.shutdown().await;
}
