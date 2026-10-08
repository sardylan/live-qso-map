#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use common::{send_udp, test_config};
use futures_util::StreamExt;
use live_qso_map::pipeline::start;
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

#[tokio::test]
async fn every_page_receives_hello_snapshot_and_live_upserts() {
    let service = start(test_config(), None).await.unwrap();
    let mut a = connect(service.http_addr).await;
    let mut b = connect(service.http_addr).await;

    for ws in [&mut a, &mut b] {
        let hello = next_json(ws, Duration::from_secs(2)).await.unwrap();
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["default_limit"], 20);
        let snapshot = next_json(ws, Duration::from_secs(2)).await.unwrap();
        assert_eq!(snapshot["type"], "snapshot");
        assert_eq!(snapshot["qsos"].as_array().unwrap().len(), 0);
    }

    send_udp(
        service.udp_addr,
        b"<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <GRIDSQUARE:6>JO62QM <EOR>",
    )
    .await;
    let ua = next_json(&mut a, Duration::from_secs(2)).await.unwrap();
    let ub = next_json(&mut b, Duration::from_secs(2)).await.unwrap();
    assert_eq!(ua["type"], "upsert");
    assert_eq!(ua, ub);
    assert_eq!(ua["qso"]["call"], "DL5AB");
    assert_eq!(ua["qso"]["remote"]["precision"], "locator6");

    live_qso_map::install_crypto_provider();
    let health: Value = serde_json::from_str(
        &reqwest::get(format!("http://{}/healthz", service.http_addr))
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["history"], 1);
    assert_eq!(health["sessions"], 2);

    service.shutdown().await;
}

#[tokio::test]
async fn static_assets_are_served() {
    let service = start(test_config(), None).await.unwrap();
    live_qso_map::install_crypto_provider();
    for (path, content_type) in [
        ("/", "text/html"),
        ("/app.js", "text/javascript"),
        ("/map-state.js", "text/javascript"),
        ("/geodesic.js", "text/javascript"),
        ("/app.css", "text/css"),
    ] {
        let response = reqwest::get(format!("http://{}{path}", service.http_addr))
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{path}");
        let ct = response.headers()["content-type"]
            .to_str()
            .unwrap()
            .to_string();
        assert!(ct.starts_with(content_type), "{path}: {ct}");
    }
    let missing = reqwest::get(format!("http://{}/nope", service.http_addr))
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    service.shutdown().await;
}
