#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! SC-004 load test: 50 pages, 10 QSO/s. Run manually, ideally on a Raspberry Pi 4:
//!
//!     cargo test --release --test load_ws -- --ignored --nocapture
//!
//! `LQM_LOAD_SECONDS` shortens the run (default 60).

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{send_udp, test_config};
use futures_util::StreamExt;
use live_qso_map::pipeline::start;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const CLIENTS: usize = 50;
const RATE: usize = 10;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "load test (SC-004): run manually, see the file header"]
async fn fifty_pages_receive_every_qso() {
    let seconds: usize = std::env::var("LQM_LOAD_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let total = seconds * RATE;
    let mut config = test_config();
    config.history_size = 0;
    let service = start(config, Some(vec![])).await.unwrap();
    let sent_at: Arc<Mutex<HashMap<String, Instant>>> = Arc::default();
    let latencies: Arc<Mutex<Vec<Duration>>> = Arc::default();

    let mut clients = Vec::new();
    for _ in 0..CLIENTS {
        let (mut ws, _) = connect_async(format!("ws://{}/ws", service.http_addr))
            .await
            .unwrap();
        let sent_at = sent_at.clone();
        let latencies = latencies.clone();
        clients.push(tokio::spawn(async move {
            let mut received = 0;
            while received < total {
                let Ok(Some(Ok(Message::Text(text)))) =
                    tokio::time::timeout(Duration::from_secs(10), ws.next()).await
                else {
                    break;
                };
                let v: serde_json::Value = serde_json::from_str(&text).unwrap();
                if v["type"] == "upsert" {
                    received += 1;
                    let call = v["qso"]["call"].as_str().unwrap().to_string();
                    if let Some(t) = sent_at.lock().unwrap().get(&call) {
                        latencies.lock().unwrap().push(t.elapsed());
                    }
                }
            }
            received
        }));
    }

    let mut tick = tokio::time::interval(Duration::from_millis(1000 / RATE as u64));
    for i in 0..total {
        tick.tick().await;
        let call = format!("L{i:05}");
        let time = format!("{:02}{:02}{:02}", (i / 3600) % 24, (i / 60) % 60, i % 60);
        sent_at.lock().unwrap().insert(call.clone(), Instant::now());
        let qso = format!(
            "<CALL:{}>{call} <QSO_DATE:8>20261008 <TIME_ON:6>{time} <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN{}{} <EOR>",
            call.len(),
            i % 10,
            (i / 10) % 10
        );
        send_udp(service.udp_addr, qso.as_bytes()).await;
    }

    for client in clients {
        assert_eq!(
            client.await.unwrap(),
            total,
            "every page received every QSO"
        );
    }
    let mut l = latencies.lock().unwrap().clone();
    l.sort();
    let p95 = l[(l.len() * 95 / 100).min(l.len() - 1)];
    println!(
        "{CLIENTS} pages × {total} QSOs: p50 {:?}, p95 {p95:?}, max {:?}",
        l[l.len() / 2],
        l[l.len() - 1]
    );
    assert!(p95 <= Duration::from_secs(2), "SC-001 p95 {p95:?}");
    service.shutdown().await;
}
