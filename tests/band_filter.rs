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

async fn next_json(ws: &mut Client, within: Duration) -> Option<Value> {
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
