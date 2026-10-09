#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use common::{send_udp, test_config};
use live_qso_map::ingest::events_from_datagram;
use live_qso_map::pipeline::start;

/// macOS caps loopback UDP datagrams at 9216 bytes by default.
const MAX_DATAGRAM: usize = 9000;

fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/malformed");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    assert!(files.len() >= 10);
    files
}

fn valid(call: &str) -> Vec<u8> {
    format!(
        "<CALL:{}>{call} <QSO_DATE:8>20261008 <TIME_ON:4>1500 <GRIDSQUARE:4>FN42 <EOR>",
        call.len()
    )
    .into_bytes()
}

#[test]
fn every_corpus_file_parses_without_panicking() {
    let from: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let station = live_qso_map::config::StationConfig::default();
    for (name, data) in corpus() {
        let events = events_from_datagram(&data, from, &station).events;
        assert!(events.is_empty(), "{name} produced a QSO");
        let mut mixed = valid("K1OK");
        mixed.extend_from_slice(&data);
        let events = events_from_datagram(&mixed, from, &station).events;
        assert_eq!(
            events.len(),
            1,
            "{name}: the valid record before it must survive"
        );
    }
}

#[tokio::test]
async fn service_survives_the_corpus_and_delivers_valid_records() {
    let service = start(test_config(), Some(vec![])).await.unwrap();
    let files = corpus();
    let mut expected = 0;
    for (i, (_, data)) in files.iter().enumerate() {
        // Alone.
        let alone = &data[..data.len().min(MAX_DATAGRAM)];
        if !alone.is_empty() {
            send_udp(service.udp_addr, alone).await;
        }
        // After a valid record in the same datagram.
        let mut mixed = valid(&format!("K{i}MIX"));
        let room = MAX_DATAGRAM - mixed.len();
        mixed.extend_from_slice(&data[..data.len().min(room)]);
        send_udp(service.udp_addr, &mixed).await;
        // A valid record in a separate datagram.
        send_udp(service.udp_addr, &valid(&format!("K{i}SEP"))).await;
        expected += 2;
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while service.history_len() < expected && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        service.history_len(),
        expected,
        "SC-006: every valid record delivered"
    );

    live_qso_map::install_crypto_provider();
    let health = reqwest::get(format!("http://{}/healthz", service.http_addr))
        .await
        .unwrap();
    assert_eq!(health.status(), 200);
    service.shutdown().await;
}
