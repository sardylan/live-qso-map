#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{next_upsert, send_udp, test_config};
use live_qso_map::adif::AdifRecord;
use live_qso_map::enrich::cty::CountryData;
use live_qso_map::pipeline::{LookupSource, cty_source, start};
use live_qso_map::position::{LookupCandidate, Position, PositionSource, Precision};

const CTY: &str = include_str!("fixtures/cty-excerpt.dat");

/// A fake online provider that answers every call with the given precision and counts calls.
fn provider(source: PositionSource, precision: Precision, calls: Arc<AtomicUsize>) -> LookupSource {
    Arc::new(move |call: String| {
        calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if call.starts_with('I') {
                // Leave Italian calls to the country data.
                return Ok(None);
            }
            let mut fields = AdifRecord::new();
            fields.set("NAME", format!("{source:?}"));
            Ok(Some(LookupCandidate {
                source,
                position: Some(Position::new(40.0, -75.0, precision, source).unwrap()),
                fields,
            }))
        })
    })
}

fn qso(call: &str, minute: u32, extra: &str) -> Vec<u8> {
    format!(
        "<CALL:{}>{call} <QSO_DATE:8>20261008 <TIME_ON:4>15{minute:02} <BAND:3>20m <MODE:2>CW {extra}<EOR>",
        call.len()
    )
    .into_bytes()
}

#[tokio::test]
async fn callsign_only_qsos_get_the_best_available_position() {
    let qrz_calls = Arc::new(AtomicUsize::new(0));
    let hamqth_calls = Arc::new(AtomicUsize::new(0));
    let sources = vec![
        provider(PositionSource::Qrz, Precision::Locator4, qrz_calls.clone()),
        provider(
            PositionSource::HamQth,
            Precision::Coordinates,
            hamqth_calls.clone(),
        ),
        cty_source(Arc::new(CountryData::parse(CTY))),
    ];
    let service = start(test_config(), Some(sources)).await.unwrap();
    let mut rx = service.subscribe();

    // Scenario 1: the most precise candidate wins.
    send_udp(service.udp_addr, &qso("K1ABC", 1, "")).await;
    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert");
    let remote = q.remote.unwrap();
    assert_eq!(remote.precision, Precision::Coordinates);
    assert_eq!(remote.source, PositionSource::HamQth);
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_SOURCE"), Some("hamqth"));
    assert_eq!(
        q.record.get("APP_LIVEQSOMAP_POS_PRECISION"),
        Some("coordinates")
    );
    assert_eq!(q.record.get("NAME"), Some("HamQth"));
    assert_eq!(
        q.record.get("COUNTRY"),
        Some("United States"),
        "filled from cty"
    );

    // Scenario 5: a QSO with a locator triggers no lookup.
    let before = qrz_calls.load(Ordering::SeqCst) + hamqth_calls.load(Ordering::SeqCst);
    send_udp(service.udp_addr, &qso("K2ABC", 2, "<GRIDSQUARE:4>FN31 ")).await;
    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert");
    assert_eq!(q.remote.unwrap().source, PositionSource::Logger);
    assert_eq!(
        qrz_calls.load(Ordering::SeqCst) + hamqth_calls.load(Ordering::SeqCst),
        before
    );

    // SC-007: country-level placement follows ham-radio entities.
    for (minute, (call, entity)) in [
        ("IS0XYZ", "Sardinia"),
        ("IT9XYZ", "Sicily"),
        ("I1XYZ", "Italy"),
    ]
    .into_iter()
    .enumerate()
    {
        send_udp(service.udp_addr, &qso(call, 10 + minute as u32, "")).await;
        let q = next_upsert(&mut rx, Duration::from_secs(2))
            .await
            .expect("upsert");
        assert_eq!(q.record.get("CALL"), Some(call));
        let remote = q.remote.unwrap();
        assert_eq!(remote.precision, Precision::Country, "{call}");
        assert_eq!(remote.source, PositionSource::Cty, "{call}");
        assert_eq!(q.record.get("COUNTRY"), Some(entity), "{call}");
    }

    service.shutdown().await;
}

#[tokio::test]
async fn unresolvable_qso_is_published_without_position() {
    let service = start(test_config(), Some(vec![])).await.unwrap();
    let mut rx = service.subscribe();
    send_udp(service.udp_addr, &qso("Q1XYZ", 3, "")).await;
    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert");
    assert!(q.remote.is_none());
    assert_eq!(service.history_len(), 1);
    service.shutdown().await;
}
