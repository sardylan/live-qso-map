//! Feature 014, User Story 3: `--enrich-only [FILE]` (contracts/command-line.md).

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use common::{Answer, FakeProvider, read_records, temp_dir, test_config};
use live_qso_map::adif::AdifRecord;
use live_qso_map::config::Config;
use live_qso_map::enrich::cty::CountryData;
use live_qso_map::errors::Error;
use live_qso_map::history_file::ADDED_FIELD;
use live_qso_map::pipeline::{EnrichReport, enrich_only};
use live_qso_map::position::{PositionSource, Precision};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CTY: &str = include_str!("fixtures/cty-excerpt.dat");

fn fixture_copy(name: &str, dir: &Path) -> std::io::Result<PathBuf> {
    let path = dir.join("log.adi");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
        &path,
    )?;
    Ok(path)
}

fn contest_config() -> Config {
    let mut config = test_config();
    config.band_filter = live_qso_map::band_filter::parse_setting(&["hf-contest".to_string()])
        .ok()
        .flatten();
    config
}

async fn run(config: Config, path: &Path, provider: &FakeProvider) -> Result<EnrichReport, Error> {
    enrich_only(
        config,
        path,
        Some(vec![provider.source()]),
        Some(Arc::new(CountryData::parse(CTY))),
        std::future::pending(),
    )
    .await
}

fn by_call(records: &[AdifRecord]) -> HashMap<String, AdifRecord> {
    records
        .iter()
        .map(|r| (r.get("CALL").unwrap_or_default().to_string(), r.clone()))
        .collect()
}

#[tokio::test]
async fn completes_every_band_without_listening() -> TestResult {
    let dir = temp_dir("enrich-bands");
    let path = fixture_copy("history-external.adi", &dir)?;
    // Hold the configured ports: enrich-only must not need them (FR-017).
    let udp = std::net::UdpSocket::bind("127.0.0.1:0")?;
    let tcp = std::net::TcpListener::bind("127.0.0.1:0")?;
    let mut config = contest_config();
    config.udp_port = udp.local_addr()?.port();
    config.http_port = tcp.local_addr()?.port();
    let provider = FakeProvider::new(PositionSource::Qrz)
        .found("K2ABC", 40.0, -75.0, Precision::Locator6)
        .found("HB9ABC", 46.95, 7.45, Precision::Locator6)
        .found("I2XYZ", 45.46, 9.19, Precision::Locator6)
        .found("IS0AAA", 39.22, 9.12, Precision::Locator6);
    let report = run(config, &path, &provider).await?;
    assert_eq!(provider.calls(), 4);
    let saved = by_call(&read_records(&path));
    assert_eq!(saved.len(), 7, "the malformed record is skipped");
    assert_eq!(
        saved["IS0AAA"].get("APP_LIVEQSOMAP_POS_SOURCE"),
        Some("qrz"),
        "2m is completed too (FR-026)"
    );
    assert_eq!(report.load.skipped, 1);
    assert_eq!(report.placed_by_record, 3);
    assert_eq!(report.placed_by_lookup, 4);
    assert_eq!(report.lookup_failed, 0);
    assert_eq!(report.without_position, 0);
    Ok(())
}

#[tokio::test]
async fn rejected_and_failed_lookups_are_tried_again() -> TestResult {
    let dir = temp_dir("enrich-retry");
    let path = fixture_copy("history-saved.adi", &dir)?;
    let provider = FakeProvider::new(PositionSource::Qrz)
        .found("I2XYZ", 45.46, 9.19, Precision::Locator6)
        .found("K3ABC", 38.6, -90.2, Precision::Locator6);
    let report = run(contest_config(), &path, &provider).await?;
    assert_eq!(
        provider.calls(),
        2,
        "reused records cause no lookup (FR-028, FR-029)"
    );
    let saved = by_call(&read_records(&path));
    assert_eq!(saved["I2XYZ"].get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    assert_eq!(saved["I2XYZ"].get("APP_LIVEQSOMAP_POS_SOURCE"), Some("qrz"));
    assert_eq!(saved["K3ABC"].get("APP_LIVEQSOMAP_LOOKUP_FAILED"), None);
    assert_eq!(report.placed_by_record, 1);
    assert_eq!(report.placed_by_lookup, 4);
    Ok(())
}

#[tokio::test]
async fn duplicates_in_the_file_are_merged() -> TestResult {
    let dir = temp_dir("enrich-dupes");
    let path = dir.join("log.adi");
    std::fs::write(
        &path,
        "<EOH>\n\
         <CALL:5>K7ABC <QSO_DATE:8>20261010 <TIME_ON:4>1200 <BAND:3>20m <MODE:2>CW <EOR>\n\
         <CALL:5>K8ABC <QSO_DATE:8>20261010 <TIME_ON:4>1201 <BAND:3>20m <MODE:2>CW <EOR>\n\
         <CALL:5>K7ABC <QSO_DATE:8>20261010 <TIME_ON:4>1200 <BAND:3>20m <MODE:2>CW <COMMENT:4>edit <EOR>\n\
         <CALL:5>K8ABC <QSO_DATE:8>20261010 <TIME_ON:4>1201 <BAND:3>20m <MODE:2>CW <EOR>\n",
    )?;
    let provider = FakeProvider::new(PositionSource::Qrz)
        .found("K7ABC", 40.0, -75.0, Precision::Locator6)
        .found("K8ABC", 41.0, -74.0, Precision::Locator6);
    run(contest_config(), &path, &provider).await?;
    let saved = read_records(&path);
    assert_eq!(saved.len(), 2);
    assert_eq!(by_call(&saved)["K7ABC"].get("COMMENT"), Some("edit"));
    assert_eq!(provider.calls(), 3, "the exact repeat costs no lookup");
    Ok(())
}

#[tokio::test]
async fn a_missing_file_is_an_error_and_nothing_is_created() -> TestResult {
    let dir = temp_dir("enrich-missing");
    let path = dir.join("log.adi");
    let provider = FakeProvider::new(PositionSource::Qrz);
    assert!(matches!(
        run(contest_config(), &path, &provider).await,
        Err(Error::HistoryFile { .. })
    ));
    assert!(!path.exists());
    Ok(())
}

#[tokio::test]
async fn an_unreadable_file_is_an_error_and_is_left_alone() -> TestResult {
    let dir = temp_dir("enrich-unreadable");
    let path = fixture_copy("history-not-adif.txt", &dir)?;
    let before = std::fs::read(&path)?;
    let provider = FakeProvider::new(PositionSource::Qrz);
    assert!(matches!(
        run(contest_config(), &path, &provider).await,
        Err(Error::HistoryFile { .. })
    ));
    assert_eq!(std::fs::read(&path)?, before);
    assert_eq!(std::fs::read_dir(&dir)?.count(), 1, "not moved aside");
    Ok(())
}

#[tokio::test]
async fn an_interrupt_saves_what_was_completed() -> TestResult {
    let dir = temp_dir("enrich-interrupt");
    let path = fixture_copy("history-external.adi", &dir)?;
    let release = Arc::new(tokio::sync::Notify::new());
    let provider = FakeProvider::new(PositionSource::Qrz).answer("K2ABC", Answer::Hang(release));
    let mut config = contest_config();
    config.workers = 1;
    config.lookup.timeout = Duration::from_secs(30);
    let watched = provider.clone();
    let stop = async move {
        while watched.calls() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let report = enrich_only(
        config,
        &path,
        Some(vec![provider.source()]),
        Some(Arc::new(CountryData::parse(CTY))),
        stop,
    )
    .await?;
    let saved = by_call(&read_records(&path));
    assert_eq!(saved.len(), 7);
    assert!(
        saved["DL1ABC"].get(ADDED_FIELD).is_some(),
        "completed before the stop"
    );
    assert_eq!(saved["K2ABC"].get(ADDED_FIELD), None, "saved as loaded");
    assert_eq!(report.load.loaded, 7);
    Ok(())
}
