#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{next_upsert, send_udp, test_config};
use live_qso_map::adif::AdifRecord;
use live_qso_map::pipeline::{LookupSource, start};
use live_qso_map::position::{LookupCandidate, Position, PositionSource, Precision};

/// A provider that counts calls; the first `slow_calls` answers take `delay`.
fn counting_provider(calls: Arc<AtomicUsize>, slow_calls: usize, delay: Duration) -> LookupSource {
    Arc::new(move |_call: String| {
        let n = calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if n < slow_calls {
                tokio::time::sleep(delay).await;
            }
            Ok(Some(LookupCandidate {
                source: PositionSource::Qrz,
                position: Some(
                    Position::new(40.0, -75.0, Precision::Locator6, PositionSource::Qrz).unwrap(),
                ),
                fields: AdifRecord::new(),
            }))
        })
    })
}

fn qso(time: &str, exchange: &str) -> Vec<u8> {
    format!(
        "<CALL:4>K6QA <QSO_DATE:8>20261008 <TIME_ON:{}>{time} <BAND:3>40m <MODE:3>SSB <SRX_STRING:{}>{exchange} <EOR>",
        time.len(),
        exchange.len()
    )
    .into_bytes()
}

#[tokio::test]
async fn identical_resends_cost_no_lookup_and_corrections_update_in_place() {
    let calls = Arc::new(AtomicUsize::new(0));
    let service = start(
        test_config(),
        Some(vec![counting_provider(calls.clone(), 0, Duration::ZERO)]),
    )
    .await
    .unwrap();
    let mut rx = service.subscribe();

    send_udp(service.udp_addr, &qso("150422", "3")).await;
    let first = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("first");
    assert_eq!(first.rev, 1);
    send_udp(service.udp_addr, &qso("150422", "3")).await;
    assert!(
        next_upsert(&mut rx, Duration::from_millis(300))
            .await
            .is_none()
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the resend never reached the providers"
    );

    send_udp(service.udp_addr, &qso("150422", "15")).await;
    let fixed = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("correction");
    assert_eq!(fixed.rev, 2);
    assert_eq!(fixed.id, first.id);
    assert_eq!(fixed.record.get("SRX_STRING"), Some("15"));
    assert_eq!(service.history_len(), 1, "SC-005");

    // FR-016: two different QSOs with the same callsign are looked up twice.
    let before = calls.load(Ordering::SeqCst);
    send_udp(service.udp_addr, &qso("151000", "3")).await;
    send_udp(service.udp_addr, &qso("152000", "3")).await;
    next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("second qso");
    next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("third qso");
    assert_eq!(calls.load(Ordering::SeqCst) - before, 2);
    service.shutdown().await;
}

#[tokio::test]
async fn newest_revision_wins_with_parallel_workers() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut config = test_config();
    config.workers = 4;
    let service = start(
        config,
        Some(vec![counting_provider(
            calls,
            1,
            Duration::from_millis(300),
        )]),
    )
    .await
    .unwrap();
    let mut rx = service.subscribe();

    // rev 1 waits 300 ms on its lookup; rev 2 answers at once and finishes first.
    send_udp(service.udp_addr, &qso("150422", "3")).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    send_udp(service.udp_addr, &qso("150422", "15")).await;

    let upsert = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert");
    assert_eq!(upsert.rev, 2);
    assert!(
        next_upsert(&mut rx, Duration::from_millis(600))
            .await
            .is_none(),
        "the stale rev 1 is never broadcast"
    );
    let stored = service.pipeline().lock_history().latest(None);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].rev, 2);
    service.shutdown().await;
}
