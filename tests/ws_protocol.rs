#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use common::{send_udp, test_config};
use futures_util::{SinkExt, StreamExt};
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
        assert_eq!(hello["protocol"], 3);
        // test_config(): IS0MZK at JM49NH.
        assert_eq!(hello["station"]["callsign"], "IS0MZK");
        assert_eq!(hello["station"]["locator"], "JM49NH");
        assert_eq!(hello["station"]["position"]["precision"], "locator6");
        assert_eq!(hello["station"]["position"]["source"], "config");
        assert!(hello.get("entity").is_some());
        assert_eq!(hello["basemap"]["kind"], "style");
        assert_eq!(
            hello["basemap"]["url"],
            "https://tiles.openfreemap.org/styles/liberty"
        );
        assert!(hello.get("tiles").is_none());
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
        ("/connection.js", "text/javascript"),
        ("/view.js", "text/javascript"),
        ("/layers.js", "text/javascript"),
        ("/qth.js", "text/javascript"),
        ("/autozoom.js", "text/javascript"),
        ("/autozoom-geo.js", "text/javascript"),
        ("/recent.js", "text/javascript"),
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
    let index = reqwest::get(format!("http://{}/", service.http_addr))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(index.contains("maplibre-gl/5.24.0/maplibre-gl.js"));
    assert!(index.contains("maplibre-gl/5.24.0/maplibre-gl.css"));
    assert!(index.contains(
        "sha512-rOFgoUgWaVi69JW1dzBu6PWF9xKW9rxYfY5u2SexFABHhWk0qUWkJoWNz4UuZRMvSXsNZG66+tU5B+HlwWF5kA=="
    ));
    assert!(index.contains(
        "sha512-KIMsMWIdnoG9OwZa+oaIafmbDonqck1UCmq+/zcUi2aaZ97N9QE6TqpTM+n36EO40HGh64hU/375zxtjx7WTyQ=="
    ));
    assert!(!index.to_lowercase().contains("leaflet"));
    let missing = reqwest::get(format!("http://{}/nope", service.http_addr))
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    service.shutdown().await;
}

/// One datagram holding `count` QSOs starting at index `from`, one second apart, with locators.
fn batch(from: usize, count: usize) -> Vec<u8> {
    let mut out = String::new();
    for i in from..from + count {
        let call = format!("K{}X{:04}", i % 10, i);
        let time = format!("{:02}{:02}{:02}", 10 + i / 3600, (i / 60) % 60, i % 60);
        out.push_str(&format!(
            "<CALL:{}>{call} <QSO_DATE:8>20261008 <TIME_ON:6>{time} <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN{}{} <EOR>",
            call.len(),
            i % 10,
            (i / 10) % 10
        ));
    }
    out.into_bytes()
}

/// Sends `total` QSOs (50 per datagram: macOS caps UDP datagrams at 9216 bytes by default)
/// and waits until the history holds `expect` of them.
async fn fill(service: &live_qso_map::pipeline::RunningService, total: usize, expect: usize) {
    let mut sent = 0;
    while sent < total {
        let n = (total - sent).min(50);
        send_udp(service.udp_addr, &batch(sent, n)).await;
        sent += n;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while service.history_len() < expect && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn set_limit(ws: &mut Client, limit: Value) {
    let text = serde_json::json!({"type": "set_limit", "limit": limit}).to_string();
    ws.send(Message::Text(text.into())).await.unwrap();
}

fn ids(snapshot: &Value) -> Vec<String> {
    snapshot["qsos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["call"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn pages_choose_their_own_window() {
    let service = start(test_config(), None).await.unwrap();
    fill(&service, 150, 150).await;
    assert_eq!(service.history_len(), 150);

    let begin = tokio::time::Instant::now();
    let mut a = connect(service.http_addr).await;
    let mut b = connect(service.http_addr).await;
    for ws in [&mut a, &mut b] {
        assert_eq!(
            next_json(ws, Duration::from_secs(3)).await.unwrap()["type"],
            "hello"
        );
        let snapshot = next_json(ws, Duration::from_secs(3)).await.unwrap();
        let calls = ids(&snapshot);
        assert_eq!(calls.len(), 20);
        assert_eq!(calls.first().unwrap(), "K0X0130");
        assert_eq!(calls.last().unwrap(), "K9X0149");
    }
    assert!(begin.elapsed() < Duration::from_secs(3), "SC-003");

    set_limit(&mut a, serde_json::json!(100)).await;
    let snapshot = next_json(&mut a, Duration::from_secs(2)).await.unwrap();
    assert_eq!(snapshot["limit"], 100);
    assert_eq!(ids(&snapshot).len(), 100);

    set_limit(&mut a, Value::Null).await;
    let snapshot = next_json(&mut a, Duration::from_secs(2)).await.unwrap();
    assert_eq!(snapshot["limit"], Value::Null);
    assert_eq!(ids(&snapshot).len(), 150);

    set_limit(&mut a, serde_json::json!(7)).await;
    assert!(
        next_json(&mut a, Duration::from_millis(300))
            .await
            .is_none(),
        "disallowed limit ignored"
    );

    // The other page saw no snapshot, only the next live upsert.
    send_udp(service.udp_addr, &batch(150, 1)).await;
    let upsert = next_json(&mut b, Duration::from_secs(2)).await.unwrap();
    assert_eq!(upsert["type"], "upsert");
    service.shutdown().await;
}

#[tokio::test]
async fn history_is_capped() {
    let mut config = test_config();
    config.history_size = 1000;
    let service = start(config, None).await.unwrap();
    fill(&service, 1001, 1000).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
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
    assert_eq!(health["history"], 1000);
    service.shutdown().await;
}
