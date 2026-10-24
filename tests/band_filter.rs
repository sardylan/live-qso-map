#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{next_upsert, send_udp, test_config};
use futures_util::StreamExt;
use live_qso_map::adif::AdifRecord;
use live_qso_map::band_filter::resolve;
use live_qso_map::pipeline::{LookupSource, start};
use live_qso_map::position::{LookupCandidate, Position, PositionSource, Precision};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect(http: SocketAddr) -> Client {
    let (ws, _) = connect_async(format!("ws://{http}/ws")).await.unwrap();
    ws
}

/// The next message, skipping `station_status`: it follows every snapshot, upsert and remove
/// (feature 017, contracts/websocket-protocol.md), and these tests are about the others.
async fn next_json(ws: &mut Client, within: Duration) -> Option<Value> {
    loop {
        let m = next_json_any(ws, within).await?;
        if m["type"] != "station_status" {
            return Some(m);
        }
    }
}

async fn next_json_any(ws: &mut Client, within: Duration) -> Option<Value> {
    loop {
        match tokio::time::timeout(within, ws.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => return Some(serde_json::from_str(&text).unwrap()),
            Ok(Some(Ok(_))) => continue,
            _ => return None,
        }
    }
}

fn counting_source(calls: Arc<AtomicUsize>) -> LookupSource {
    Arc::new(move |_call: String| {
        calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
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

fn adif(call: &str, minute: u32, band: &str) -> Vec<u8> {
    let band_field = if band.is_empty() {
        String::new()
    } else {
        format!("<BAND:{}>{band} ", band.len())
    };
    format!(
        "<CALL:{}>{call} <QSO_DATE:8>20261009 <TIME_ON:4>15{minute:02} {band_field}<MODE:2>CW <EOR>",
        call.len()
    )
    .into_bytes()
}

#[tokio::test]
async fn only_considered_bands_reach_the_map() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut config = test_config();
    config.band_filter = Some(resolve(&["hf-contest"]).unwrap());
    let service = start(config, Some(vec![counting_source(calls.clone())]))
        .await
        .unwrap();
    let mut rx = service.subscribe();
    let mut ws = connect(service.http_addr).await;

    let hello = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    let bands: Vec<&str> = hello["bands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["band"].as_str().unwrap())
        .collect();
    assert_eq!(bands, ["160m", "80m", "40m", "20m", "15m", "10m"]);
    assert_eq!(
        next_json(&mut ws, Duration::from_secs(2)).await.unwrap()["type"],
        "snapshot"
    );

    send_udp(service.udp_addr, &adif("K1AA", 1, "20m")).await;
    send_udp(service.udp_addr, &adif("K2BB", 2, "2m")).await;
    send_udp(service.udp_addr, &adif("K3CC", 3, "")).await;

    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("the 20m QSO");
    assert_eq!(q.record.get("CALL"), Some("K1AA"));
    let upsert = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    assert_eq!(upsert["qso"]["call"], "K1AA");

    assert!(
        next_upsert(&mut rx, Duration::from_millis(400))
            .await
            .is_none()
    );
    assert!(
        next_json(&mut ws, Duration::from_millis(100))
            .await
            .is_none()
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "ignored QSOs cost no lookups"
    );
    assert_eq!(service.history_len(), 1);
    service.shutdown().await;
}

const CTY: &str = include_str!("fixtures/cty-excerpt.dat");

/// QARTest packet #3 of the reference capture (K6QA, id 3809734F4), as `root` with `txfreq`.
fn qartest(root: &str, id: &str, txfreq: &str) -> Vec<u8> {
    let extra = if root == "contactreplace" {
        "<oldtimestamp>2026-10-08 15:04:22</oldtimestamp><oldcall>K6QA</oldcall>"
    } else {
        ""
    };
    format!(
        "<?xml version=\"1.0\"?><{root}><logger>QARTest 16.9.1</logger><contestname>CQ-WW-SSB</contestname><timestamp>2026-10-08 15:04:22</timestamp><mycall>IS0MZK</mycall><band>20</band><txfreq>{txfreq}</txfreq><operator></operator><mode>USB</mode><call>K6QA</call><countryprefix>K</countryprefix><wpxprefix>K6</wpxprefix><snt>59</snt><rcv>59</rcv><nr>6</nr><exch1>3</exch1><exch2></exch2><exch3></exch3><duplicate>False</duplicate><stationname></stationname><points>3</points><xqso>0</xqso><id>{id}</id>{extra}</{root}>"
    )
    .into_bytes()
}

#[tokio::test]
async fn correction_onto_ignored_band_removes_the_qso() {
    use live_qso_map::enrich::cty::CountryData;
    use live_qso_map::pipeline::{HistoryChange, cty_source};

    let mut config = test_config();
    config.band_filter = Some(resolve(&["hf-contest"]).unwrap());
    let sources = vec![cty_source(Arc::new(CountryData::parse(CTY)))];
    let service = start(config, Some(sources)).await.unwrap();
    let mut rx = service.subscribe();
    let mut ws = connect(service.http_addr).await;
    next_json(&mut ws, Duration::from_secs(2)).await.unwrap(); // hello
    next_json(&mut ws, Duration::from_secs(2)).await.unwrap(); // snapshot

    // Shown on 20m.
    send_udp(
        service.udp_addr,
        &qartest("contactinfo", "3809734F4", "1402500"),
    )
    .await;
    let q = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert");
    let id = q.id.clone();
    assert_eq!(
        next_json(&mut ws, Duration::from_secs(2)).await.unwrap()["type"],
        "upsert"
    );

    // Corrected onto 60m: removed everywhere within 2 s (SC-004).
    send_udp(
        service.udp_addr,
        &qartest("contactreplace", "3809734F4", "535000"),
    )
    .await;
    match tokio::time::timeout(Duration::from_secs(2), rx.recv()).await {
        Ok(Ok(HistoryChange::Remove { id: removed })) => assert_eq!(removed, id),
        other => panic!("expected a removal, got {other:?}"),
    }
    let removal = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    assert_eq!(removal["type"], "remove");
    assert_eq!(removal["id"], id.as_str());
    assert_eq!(service.history_len(), 0);

    // Corrected back onto 20m: shown again.
    send_udp(
        service.udp_addr,
        &qartest("contactreplace", "3809734F4", "1402500"),
    )
    .await;
    let back = next_upsert(&mut rx, Duration::from_secs(2))
        .await
        .expect("upsert again");
    assert_eq!(back.id, id);
    assert_eq!(service.history_len(), 1);
    assert_eq!(
        next_json(&mut ws, Duration::from_secs(2)).await.unwrap()["type"],
        "upsert"
    );

    // A correction onto 60m for a QSO never shown: nothing at all.
    send_udp(
        service.udp_addr,
        &qartest("contactreplace", "FFFFFFFF", "535000"),
    )
    .await;
    assert!(
        tokio::time::timeout(Duration::from_millis(400), rx.recv())
            .await
            .is_err()
    );
    assert!(
        next_json(&mut ws, Duration::from_millis(100))
            .await
            .is_none()
    );
    assert_eq!(service.history_len(), 1);
    service.shutdown().await;
}

#[tokio::test]
async fn mixed_stream_respects_the_filter() {
    use live_qso_map::band::BANDS_IN_ORDER;

    let calls = Arc::new(AtomicUsize::new(0));
    let hf = resolve(&["hf"]).unwrap();
    let mut config = test_config();
    config.band_filter = Some(hf.clone());
    let service = start(config, Some(vec![counting_source(calls.clone())]))
        .await
        .unwrap();

    // 1,000 callsign-only QSOs cycling over every ADIF band plus "no band" (SC-001, SC-002).
    let choices: Vec<&str> = BANDS_IN_ORDER.iter().copied().chain([""]).collect();
    let mut expected = 0;
    let mut batch = Vec::new();
    for i in 0..1000usize {
        let band = choices[i % choices.len()];
        if hf.contains(band) {
            expected += 1;
        }
        let band_field = if band.is_empty() {
            String::new()
        } else {
            format!("<BAND:{}>{band} ", band.len())
        };
        let call = format!("M{i:04}");
        let time = format!("{:02}{:02}{:02}", 10 + i / 3600, (i / 60) % 60, i % 60);
        batch.extend(
            format!("<CALL:{}>{call} <QSO_DATE:8>20261009 <TIME_ON:6>{time} {band_field}<MODE:2>CW <EOR>", call.len())
                .into_bytes(),
        );
        if (i + 1) % 50 == 0 {
            send_udp(service.udp_addr, &batch).await;
            batch.clear();
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while service.history_len() < expected && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    let history = service.pipeline().lock_history().latest(None);
    assert_eq!(
        history.len(),
        expected,
        "every considered QSO and nothing else"
    );
    assert!(
        history
            .iter()
            .all(|q| hf.contains(q.record.get("BAND").unwrap_or_default()))
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        expected,
        "lookups only for considered QSOs"
    );
    service.shutdown().await;
}
