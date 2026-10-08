#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::time::Duration;

use common::{next_upsert, send_udp, test_config};
use live_qso_map::maidenhead;
use live_qso_map::pipeline::start;
use live_qso_map::position::{PositionSource, Precision};

const QUICKSTART_2: &[u8] =
    b"<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <GRIDSQUARE:6>JO62QM <EOR>";

const QUICKSTART_3: &[u8] = b"<ADIF_VER:5>3.1.4 <EOH><CALL:4>K1AA <QSO_DATE:8>20261008 <TIME_ON:4>1510 <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR><CALL:4>K2BB <QSO_DATE:8>20261008 <TIME_ON:4>1511 <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR><CALL:3>XX1 <QSO_DATE:3>BAD<EOR>";

#[tokio::test]
async fn adif_qso_with_positions_is_published() {
    let service = start(test_config(), None).await.unwrap();
    let mut rx = service.subscribe();

    send_udp(service.udp_addr, QUICKSTART_2).await;
    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert within 2 s");
    assert_eq!(q.record.get("CALL"), Some("DL5AB"));
    assert_eq!(q.record.get("STATION_CALLSIGN"), Some("IS0MZK"));
    assert_eq!(q.rev, 1);

    let remote = q.remote.unwrap();
    let (lat, lon) = maidenhead::centre("JO62QM").unwrap();
    assert_eq!(remote.precision, Precision::Locator6);
    assert_eq!(remote.source, PositionSource::Logger);
    assert!((remote.lat - lat).abs() < 1e-9 && (remote.lon - lon).abs() < 1e-9);

    let station = q.station.unwrap();
    assert_eq!(station.source, PositionSource::Config);
    assert_eq!(station.precision, Precision::Locator6);

    send_udp(service.udp_addr, QUICKSTART_3).await;
    let a = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("first record");
    let b = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("second record");
    let mut calls = vec![
        a.record.get("CALL").unwrap().to_string(),
        b.record.get("CALL").unwrap().to_string(),
    ];
    calls.sort();
    assert_eq!(calls, ["K1AA", "K2BB"]);
    assert!(
        next_upsert(&mut rx, Duration::from_millis(300))
            .await
            .is_none(),
        "malformed record skipped"
    );

    // Still alive after the malformed record.
    send_udp(
        service.udp_addr,
        b"<CALL:4>K3CC <QSO_DATE:8>20261008 <TIME_ON:4>1512 <GRIDSQUARE:4>FN31 <EOR>",
    )
    .await;
    assert!(next_upsert(&mut rx, Duration::from_secs(2)).await.is_some());
    assert_eq!(service.history_len(), 4);
    service.shutdown().await;
}

#[tokio::test]
async fn unrecognised_datagram_is_dropped() {
    let service = start(test_config(), None).await.unwrap();
    let mut rx = service.subscribe();
    send_udp(service.udp_addr, b"hello world").await;
    assert!(
        next_upsert(&mut rx, Duration::from_millis(300))
            .await
            .is_none()
    );
    assert_eq!(service.history_len(), 0);
    service.shutdown().await;
}
