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
        assert_eq!(hello["protocol"], 4);
        // test_config(): IS0MZK at JM49NH.
        assert_eq!(hello["station"]["callsign"], "IS0MZK");
        assert_eq!(hello["station"]["locator"], "JM49NH");
        assert_eq!(hello["station"]["position"]["precision"], "locator6");
        assert_eq!(hello["station"]["position"]["source"], "config");
        assert!(hello.get("entity").is_some());
        assert_eq!(
            hello["recent"],
            serde_json::json!({"columns": ["call", "band", "time", "cqzone", "country"], "header": true})
        );
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

/// With the default bands, `hello.band_groups` offers "HF contest" only: "HF" would select the
/// same six bands (feature 012, B4, contracts/websocket-protocol.md).
#[tokio::test]
async fn default_bands_offer_a_single_band_group() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = test_config();
    config.band_filter = Some(live_qso_map::band_filter::default_set());
    let service = start(config, None).await?;
    let mut ws = connect(service.http_addr).await;
    let hello = next_json(&mut ws, Duration::from_secs(2))
        .await
        .ok_or("no hello")?;
    let names: Vec<&str> = hello["band_groups"]
        .as_array()
        .ok_or("no band_groups")?
        .iter()
        .filter_map(|g| g["name"].as_str())
        .collect();
    assert_eq!(names, ["hf-contest"]);
    Ok(())
}

/// Every module and stylesheet in `src/web/assets/` is served, byte for byte. The page tests
/// import the files from disk, so a module the server doesn't route passes them and still
/// breaks the page: `app.js` fails to load when one of its imports is missing (3.0.0).
#[tokio::test]
async fn every_page_asset_on_disk_is_served() {
    let service = start(test_config(), None).await.unwrap();
    live_qso_map::install_crypto_provider();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/web/assets");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content_type = match path.extension().and_then(|e| e.to_str()) {
            Some("js") => "text/javascript",
            Some("css") => "text/css",
            _ => continue,
        };
        let response = reqwest::get(format!("http://{}/{name}", service.http_addr))
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "/{name} is not served");
        let header = response.headers()["content-type"]
            .to_str()
            .unwrap()
            .to_string();
        assert!(header.starts_with(content_type), "/{name}: {header}");
        let body = response.text().await.unwrap();
        assert_eq!(
            body,
            std::fs::read_to_string(&path).unwrap(),
            "/{name} differs"
        );
        checked += 1;
    }
    assert!(checked >= 16, "only {checked} assets found");
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
        ("/score.js", "text/javascript"),
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
        // Browsers must revalidate, so a rebuilt binary never runs with stale page modules.
        assert_eq!(
            response.headers()["cache-control"].to_str().unwrap(),
            "no-cache",
            "{path}"
        );
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
    // Feature 011 layout: one gear-opened side panel holding the box switch; feature 007 column.
    for id in [
        r#"id="panel""#,
        r#"id="recent-shown""#,
        r#"id="corner-bl""#,
        r#"id="score""#,
    ] {
        assert!(index.contains(id), "{id}");
    }
    assert!(!index.contains(r#"class="limit-control""#));
    let app = reqwest::get(format!("http://{}/app.js", service.http_addr))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        !app.contains("class RecentControl"),
        "the box switch left the map controls"
    );
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

/// Checks one error response against `docs/http-api.md` API-3 and returns its body.
async fn json_error(response: reqwest::Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status(), status, "{code}");
    let ct = response.headers()["content-type"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("application/json"), "{code}: {ct}");
    let body: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["error"], code);
    let message = body["message"].as_str().unwrap();
    assert!(!message.is_empty());
    assert!(
        !message.contains("/nonexistent/cty.dat"),
        "no paths in errors"
    );
    body
}

#[tokio::test]
async fn every_error_response_has_a_json_body() {
    let service = start(test_config(), None).await.unwrap();
    live_qso_map::install_crypto_provider();
    let client = reqwest::Client::new();
    let base = format!("http://{}", service.http_addr);

    let missing = client.get(format!("{base}/nope")).send().await.unwrap();
    let body = json_error(missing, 404, "not_found").await;
    assert!(body["message"].as_str().unwrap().contains("/nope"));

    let wrong = client.post(format!("{base}/healthz")).send().await.unwrap();
    assert!(wrong.headers().contains_key("allow"), "405 keeps Allow");
    json_error(wrong, 405, "method_not_allowed").await;

    let plain = client.get(format!("{base}/ws")).send().await.unwrap();
    let status = plain.status().as_u16();
    assert!((400..500).contains(&status), "{status}");
    json_error(plain, status, "websocket_upgrade_required").await;

    service.shutdown().await;
}

#[tokio::test]
async fn a_page_gets_the_station_and_qsos_that_share_it() {
    let service = start(test_config(), None).await.unwrap();
    let mut rx = service.subscribe();
    // Seven QSOs sent out of time order, with no station position of their own.
    for minute in [3, 1, 7, 2, 6, 4, 5] {
        let qso = format!(
            "<CALL:5>K{minute}ABC <QSO_DATE:8>20261009 <TIME_ON:4>15{minute:02} <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR>"
        );
        send_udp(service.udp_addr, qso.as_bytes()).await;
    }
    for _ in 0..7 {
        common::next_upsert(&mut rx, Duration::from_secs(2))
            .await
            .expect("upsert");
    }

    let mut ws = connect(service.http_addr).await;
    let hello = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    assert_eq!(hello["protocol"], 4);
    let station = hello["station"]["position"].clone();
    assert_eq!(station["source"], "config");
    let snapshot = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    let qsos = snapshot["qsos"].as_array().unwrap();
    assert_eq!(qsos.len(), 7);
    let keys: Vec<&str> = qsos
        .iter()
        .map(|q| q["sort_key"].as_str().unwrap())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted, "snapshot in sort_key order");
    for q in qsos {
        assert_eq!(q["station"], station, "{}", q["call"]);
    }
    service.shutdown().await;
}

#[tokio::test]
async fn a_page_receives_each_qso_entity() {
    use live_qso_map::enrich::cty::CountryData;
    use live_qso_map::pipeline::start_with_country;
    use std::sync::Arc;

    let country = Arc::new(CountryData::parse(include_str!("fixtures/cty-excerpt.dat")));
    let service = start_with_country(test_config(), Some(vec![]), Some(country))
        .await
        .unwrap();
    let mut ws = connect(service.http_addr).await;
    next_json(&mut ws, Duration::from_secs(2)).await.unwrap(); // hello
    next_json(&mut ws, Duration::from_secs(2)).await.unwrap(); // snapshot

    let mut seen = Vec::new();
    for (minute, call) in [(1, "K1ABC"), (2, "QQ9QQ")] {
        let qso = format!(
            "<CALL:{}>{call} <QSO_DATE:8>20261009 <TIME_ON:4>150{minute} <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR>",
            call.len()
        );
        send_udp(service.udp_addr, qso.as_bytes()).await;
        // SC-004: within the 2 s a QSO may take to reach the page.
        let upsert = next_json(&mut ws, Duration::from_secs(2))
            .await
            .expect("upsert within 2 s");
        seen.push((
            upsert["qso"]["call"].as_str().unwrap().to_string(),
            upsert["qso"]["entity"].clone(),
        ));
    }
    let expected = serde_json::json!({"prefix": "K", "name": "United States", "cq_zone": 5});
    assert_eq!(seen[0], ("K1ABC".to_string(), expected.clone()));
    assert_eq!(seen[1], ("QQ9QQ".to_string(), Value::Null));

    let mut late = connect(service.http_addr).await;
    next_json(&mut late, Duration::from_secs(2)).await.unwrap(); // hello
    let snapshot = next_json(&mut late, Duration::from_secs(2)).await.unwrap();
    let entities: Vec<Value> = snapshot["qsos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["entity"].clone())
        .collect();
    assert_eq!(entities, vec![expected, Value::Null]);
    service.shutdown().await;
}

// --- contest score (feature 007, T008) --------------------------------------------------------

const REFERENCE: &str =
    include_str!("../specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt");

fn score_packet(i: usize) -> Vec<u8> {
    live_qso_map::ingest::qartest::reference_packets(REFERENCE)[i].clone()
}

/// Skips `hello` and `snapshot`.
async fn handshake(ws: &mut Client) {
    for kind in ["hello", "snapshot"] {
        let m = next_json(ws, Duration::from_secs(2)).await.unwrap();
        assert_eq!(m["type"], kind);
    }
}

fn assert_score(m: &Value, (score, qsos, mults): (u64, u64, u64)) {
    assert_eq!(m["type"], "score", "{m}");
    assert_eq!(m["score"], score, "{m}");
    assert_eq!(m["qsos"], qsos, "{m}");
    assert_eq!(m["mults"], mults, "{m}");
    let at = m["received_at"].as_str().unwrap();
    assert!(at.len() == 20 && at.ends_with('Z'), "{at}");
    assert!(m["age_s"].as_u64().unwrap() <= 1, "{m}");
}

#[tokio::test]
async fn a_score_reaches_open_pages_and_pages_opened_later() {
    let service = start(test_config(), None).await.unwrap();
    let mut early = connect(service.http_addr).await;
    handshake(&mut early).await;
    assert!(
        next_json(&mut early, Duration::from_millis(500))
            .await
            .is_none(),
        "no score is held yet"
    );

    // Live, within 2 s (SC-001).
    send_udp(service.udp_addr, &score_packet(3)).await;
    let live = next_json(&mut early, Duration::from_secs(2)).await.unwrap();
    assert_score(&live, (90, 6, 10));

    // On connect, right after the handshake (SC-002).
    let mut late = connect(service.http_addr).await;
    handshake(&mut late).await;
    let held = next_json(&mut late, Duration::from_secs(1)).await.unwrap();
    assert_score(&held, (90, 6, 10));

    // The latest score wins, whatever its station callsign (spec Edge Cases).
    let other = String::from_utf8(score_packet(9))
        .unwrap()
        .replace("<call>IS0MZK</call>", "<call>IQ0ZZZ</call>");
    send_udp(service.udp_addr, other.as_bytes()).await;
    for ws in [&mut early, &mut late] {
        let m = next_json(ws, Duration::from_secs(2)).await.unwrap();
        assert_score(&m, (132, 7, 11));
    }
    let mut last = connect(service.http_addr).await;
    handshake(&mut last).await;
    assert_score(
        &next_json(&mut last, Duration::from_secs(1)).await.unwrap(),
        (132, 7, 11),
    );
    assert!(
        next_json(&mut last, Duration::from_millis(300))
            .await
            .is_none()
    );
    service.shutdown().await;
}

#[tokio::test]
async fn malformed_scores_change_nothing() {
    let service = start(test_config(), None).await.unwrap();
    let mut ws = connect(service.http_addr).await;
    handshake(&mut ws).await;
    let bad = String::from_utf8(score_packet(3))
        .unwrap()
        .replace("<score>90</score>", "<score>x</score>");
    for _ in 0..100 {
        send_udp(service.udp_addr, bad.as_bytes()).await;
    }
    send_udp(
        service.udp_addr,
        b"<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <EOR>",
    )
    .await;
    let m = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    assert_eq!(m["type"], "upsert", "{m}");
    assert!(
        next_json(&mut ws, Duration::from_millis(300))
            .await
            .is_none()
    );
    assert!(service.latest_score().is_none());
    service.shutdown().await;
}

#[tokio::test]
async fn the_band_filter_does_not_hide_the_score() {
    let mut config = test_config();
    config.band_filter = Some(live_qso_map::band_filter::resolve(&["20m"]).unwrap());
    let service = start(config, None).await.unwrap();
    let mut ws = connect(service.http_addr).await;
    handshake(&mut ws).await;
    send_udp(service.udp_addr, &score_packet(3)).await;
    let m = next_json(&mut ws, Duration::from_secs(2)).await.unwrap();
    assert_score(&m, (90, 6, 10));
    service.shutdown().await;
}

/// Like `batch`, with every third QSO on 40 m and the others on 20 m (feature 011).
fn mixed_batch(count: usize) -> Vec<u8> {
    let mut out = String::new();
    for i in 0..count {
        let call = format!("K{}Y{:04}", i % 10, i);
        let time = format!("{:02}{:02}{:02}", 10 + i / 3600, (i / 60) % 60, i % 60);
        let band = if i % 3 == 0 { "40m" } else { "20m" };
        out.push_str(&format!(
            "<CALL:{}>{call} <QSO_DATE:8>20261013 <TIME_ON:6>{time} <BAND:3>{band} <MODE:2>CW <GRIDSQUARE:4>FN{}{} <EOR>",
            call.len(),
            i % 10,
            (i / 10) % 10
        ));
    }
    out.into_bytes()
}

#[tokio::test]
async fn pages_can_ask_for_the_last_n_on_chosen_bands() {
    let service = start(test_config(), None).await.unwrap();
    send_udp(service.udp_addr, &mixed_batch(45)).await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while service.history_len() < 45 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let mut page = connect(service.http_addr).await;
    let hello = next_json(&mut page, Duration::from_secs(3)).await.unwrap();
    assert_eq!(hello["protocol"], 4);
    let group = &hello["band_groups"].as_array().unwrap()[2];
    assert_eq!(group["name"], "hf-contest");
    assert_eq!(group["label"], "HF contest");
    next_json(&mut page, Duration::from_secs(3)).await.unwrap(); // first snapshot

    let text = serde_json::json!({"type": "set_limit", "limit": 10, "bands": ["40m"]}).to_string();
    page.send(Message::Text(text.into())).await.unwrap();
    let snapshot = next_json(&mut page, Duration::from_secs(2)).await.unwrap();
    assert_eq!(snapshot["type"], "snapshot");
    let bands: Vec<&str> = snapshot["qsos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["band"].as_str().unwrap())
        .collect();
    assert_eq!(
        bands.len(),
        10,
        "15 QSOs are on 40 m: the newest 10 of them"
    );
    assert!(bands.iter().all(|b| *b == "40m"), "{bands:?}");

    // A page sending no bands still gets every QSO.
    let mut old = connect(service.http_addr).await;
    next_json(&mut old, Duration::from_secs(3)).await.unwrap();
    let snapshot = next_json(&mut old, Duration::from_secs(3)).await.unwrap();
    assert_eq!(ids(&snapshot).len(), 20);
    service.shutdown().await;
}
