//! Feature 014: the QSO history file (contracts/history-file.md, spec User Stories 1 and 2).

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{Answer, FakeProvider, read_records, send_udp, temp_dir, test_config};
use live_qso_map::adif::AdifRecord;
use live_qso_map::enrich::cty::CountryData;
use live_qso_map::history_file::ADDED_FIELD;
use live_qso_map::pipeline::{RunningService, start_with_country};
use live_qso_map::position::{PositionSource, Precision};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CTY: &str = include_str!("fixtures/cty-excerpt.dat");
const FILE: &str = "qso-history.adi";

fn country() -> Arc<CountryData> {
    Arc::new(CountryData::parse(CTY))
}

async fn start_on(
    path: &Path,
    provider: &FakeProvider,
    tune: impl FnOnce(&mut live_qso_map::config::Config),
) -> Result<RunningService, Box<dyn std::error::Error>> {
    let mut config = test_config();
    config.history_file = Some(path.to_path_buf());
    tune(&mut config);
    Ok(start_with_country(config, Some(vec![provider.source()]), Some(country())).await?)
}

fn adif(call: &str, time: &str, extra: &str) -> Vec<u8> {
    format!(
        "<CALL:{}>{call} <QSO_DATE:8>20261010 <TIME_ON:4>{time} <BAND:3>20m <MODE:2>CW {extra}<EOR>",
        call.len()
    )
    .into_bytes()
}

/// Waits until the history holds `n` QSOs (or 5 s pass) and returns its length.
async fn wait_for_len(service: &RunningService, n: usize) -> usize {
    let deadline = Instant::now() + Duration::from_secs(5);
    while service.history_len() < n && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    service.history_len()
}

/// Position, precision, source and entity prefix of every QSO, by call.
fn snapshot(
    service: &RunningService,
) -> HashMap<String, (f64, f64, Precision, PositionSource, String)> {
    let history = service.pipeline().lock_history();
    history
        .iter_in_order()
        .filter_map(|q| {
            let p = q.remote?;
            let call = q.record.get("CALL")?.to_string();
            let entity = q
                .entity
                .as_ref()
                .map(|e| e.prefix.clone())
                .unwrap_or_default();
            Some((call, (p.lat, p.lon, p.precision, p.source, entity)))
        })
        .collect()
}

fn us_provider() -> FakeProvider {
    FakeProvider::new(PositionSource::Qrz)
        .found("K2ABC", 40.0, -75.0, Precision::Locator6)
        .found("K3ABC", 41.0, -74.0, Precision::Locator6)
        .found("K4ABC", 35.0, -80.0, Precision::Coordinates)
}

/// Sends the five QSOs of the round-trip tests and waits for them.
async fn send_five(service: &RunningService) -> usize {
    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    send_udp(
        service.udp_addr,
        &adif("HB9ABC", "1201", "<GRIDSQUARE:4>JN36 "),
    )
    .await;
    send_udp(service.udp_addr, &adif("K2ABC", "1202", "")).await;
    send_udp(service.udp_addr, &adif("K3ABC", "1203", "")).await;
    send_udp(service.udp_addr, &adif("K4ABC", "1204", "")).await;
    wait_for_len(service, 5).await
}

fn file_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

// --- User Story 1 (T019) --------------------------------------------------------------------

#[tokio::test]
async fn the_map_survives_a_restart_without_lookups() -> TestResult {
    let path = file_in(&temp_dir("round-trip"));
    let first = us_provider();
    let service = start_on(&path, &first, |_| {}).await?;
    assert_eq!(send_five(&service).await, 5);
    let before = snapshot(&service);
    assert_eq!(first.calls(), 3);
    service.shutdown().await;

    let saved = read_records(&path);
    assert_eq!(saved.len(), 5);
    assert!(saved.iter().all(|r| r.get(ADDED_FIELD).is_some()));

    let second = us_provider();
    let service = start_on(&path, &second, |_| {}).await?;
    assert_eq!(wait_for_len(&service, 5).await, 5);
    let after = snapshot(&service);
    assert_eq!(after.len(), before.len());
    for (call, (lat, lon, precision, source, entity)) in &before {
        let (alat, alon, aprecision, asource, aentity) = &after[call];
        assert!(
            (lat - alat).abs() < 1e-4 && (lon - alon).abs() < 1e-4,
            "{call}"
        );
        assert_eq!(
            (precision, source, entity),
            (aprecision, asource, aentity),
            "{call}"
        );
    }
    assert_eq!(
        second.calls(),
        0,
        "no lookups for restored positions (SC-002)"
    );
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn resends_corrections_and_dupes_after_a_restart() -> TestResult {
    let path = file_in(&temp_dir("resend"));
    let first = us_provider();
    let service = start_on(&path, &first, |_| {}).await?;
    send_udp(service.udp_addr, &adif("K2ABC", "1202", "")).await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    let id = service
        .pipeline()
        .lock_history()
        .iter_in_order()
        .next()
        .map(|q| q.id.clone());
    service.shutdown().await;

    let second = us_provider();
    let service = start_on(&path, &second, |_| {}).await?;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    let mut rx = service.subscribe();

    // An identical resend is dropped.
    send_udp(service.udp_addr, &adif("K2ABC", "1202", "")).await;
    assert!(
        common::next_upsert(&mut rx, Duration::from_millis(400))
            .await
            .is_none()
    );
    assert_eq!(second.calls(), 0);

    // A correction replaces the restored QSO.
    send_udp(service.udp_addr, &adif("K2ABC", "1202", "<COMMENT:4>edit ")).await;
    let corrected = common::next_upsert(&mut rx, Duration::from_secs(2)).await;
    let corrected = corrected.ok_or("no upsert for the correction")?;
    assert_eq!(Some(corrected.id.clone()), id);
    assert_eq!(corrected.rev, 2);
    assert_eq!(service.history_len(), 1);

    // A dupe (same call, another time) is another QSO (HR-9).
    send_udp(service.udp_addr, &adif("K2ABC", "1302", "")).await;
    assert_eq!(wait_for_len(&service, 2).await, 2);
    service.shutdown().await;
    Ok(())
}

/// QARTest packet #3 of the reference capture (K6QA), as `root`.
fn qartest(root: &str, id: &str, rcv: &str) -> Vec<u8> {
    let extra = if root == "contactreplace" {
        "<oldtimestamp>2026-10-08 15:04:22</oldtimestamp><oldcall>K6QA</oldcall>"
    } else {
        ""
    };
    format!(
        "<?xml version=\"1.0\"?><{root}><logger>QARTest 16.9.1</logger><contestname>CQ-WW-SSB</contestname><timestamp>2026-10-08 15:04:22</timestamp><mycall>IS0MZK</mycall><band>20</band><txfreq>1420000</txfreq><operator></operator><mode>USB</mode><call>K6QA</call><countryprefix>K</countryprefix><wpxprefix>K6</wpxprefix><snt>59</snt><rcv>{rcv}</rcv><nr>6</nr><exch1>3</exch1><exch2></exch2><exch3></exch3><duplicate>False</duplicate><stationname></stationname><points>3</points><xqso>0</xqso><id>{id}</id>{extra}</{root}>"
    )
    .into_bytes()
}

#[tokio::test]
async fn a_qartest_correction_finds_the_restored_qso_by_logger_id() -> TestResult {
    let path = file_in(&temp_dir("qartest"));
    let provider =
        FakeProvider::new(PositionSource::Qrz).found("K6QA", 37.0, -122.0, Precision::Locator6);
    let service = start_on(&path, &provider, |_| {}).await?;
    send_udp(service.udp_addr, &qartest("contactinfo", "3809734F4", "59")).await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    service.shutdown().await;

    let service = start_on(&path, &provider, |_| {}).await?;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    let mut rx = service.subscribe();
    send_udp(
        service.udp_addr,
        &qartest("contactreplace", "3809734F4", "57"),
    )
    .await;
    let corrected = common::next_upsert(&mut rx, Duration::from_secs(2)).await;
    let corrected = corrected.ok_or("no upsert for the QARTest correction")?;
    assert_eq!(corrected.rev, 2);
    assert_eq!(corrected.record.get("RST_RCVD"), Some("57"));
    assert_eq!(service.history_len(), 1);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn a_missing_file_is_created_at_the_stop() -> TestResult {
    let path = file_in(&temp_dir("missing"));
    let provider = us_provider();
    let service = start_on(&path, &provider, |_| {}).await?;
    assert_eq!(service.history_len(), 0);
    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    assert!(!path.exists());
    service.shutdown().await;
    assert_eq!(read_records(&path).len(), 1);
    Ok(())
}

#[tokio::test]
async fn a_file_deleted_while_running_is_written_again() -> TestResult {
    let path = file_in(&temp_dir("deleted"));
    let provider = us_provider();
    let service = start_on(&path, &provider, |_| {}).await?;
    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    service.shutdown().await;

    let service = start_on(&path, &provider, |_| {}).await?;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    std::fs::remove_file(&path)?;
    service.shutdown().await;
    assert_eq!(read_records(&path).len(), 1, "FR-003: a stop always saves");
    Ok(())
}

#[tokio::test]
async fn an_unreadable_file_is_moved_aside() -> TestResult {
    let dir = temp_dir("unreadable");
    let path = file_in(&dir);
    let original = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/history-not-adif.txt"),
    )?;
    std::fs::write(&path, &original)?;
    let provider = us_provider();
    let service = start_on(&path, &provider, |_| {}).await?;
    assert_eq!(service.history_len(), 0);
    let moved: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().contains(".unreadable-"))
        .collect();
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(std::fs::read(&moved[0])?, original);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn a_failed_save_leaves_no_file_behind() -> TestResult {
    let dir = temp_dir("save-fails");
    let folder = dir.join("data");
    std::fs::create_dir(&folder)?;
    let path = file_in(&folder);
    let provider = us_provider();
    let service = start_on(&path, &provider, |_| {}).await?;
    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    std::fs::remove_dir_all(&folder)?;
    service.shutdown().await;
    assert!(!folder.exists(), "the save must not recreate the folder");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn a_failed_save_keeps_the_previous_file() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("save-readonly");
    let folder = dir.join("data");
    std::fs::create_dir(&folder)?;
    let path = file_in(&folder);
    let provider = us_provider();
    let service = start_on(&path, &provider, |_| {}).await?;
    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    service.shutdown().await;
    let previous = std::fs::read(&path)?;

    let service = start_on(&path, &provider, |_| {}).await?;
    send_udp(
        service.udp_addr,
        &adif("HB9ABC", "1201", "<GRIDSQUARE:4>JN36 "),
    )
    .await;
    assert_eq!(wait_for_len(&service, 2).await, 2);
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555))?;
    service.shutdown().await;
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755))?;
    let now = std::fs::read(&path)?;
    // Root ignores folder permissions: then the save succeeds and there is nothing to check.
    if read_records(&path).len() != 2 {
        assert_eq!(now, previous);
    }
    Ok(())
}

#[tokio::test]
async fn a_qso_waiting_for_a_lookup_at_the_stop_is_saved_as_received() -> TestResult {
    let path = file_in(&temp_dir("pending"));
    let release = Arc::new(tokio::sync::Notify::new());
    let hanging = FakeProvider::new(PositionSource::Qrz).answer("K5ABC", Answer::Hang(release));
    let service = start_on(&path, &hanging, |c| {
        c.lookup.timeout = Duration::from_secs(30)
    })
    .await?;
    send_udp(service.udp_addr, &adif("K5ABC", "1205", "")).await;
    let deadline = Instant::now() + Duration::from_secs(2);
    while hanging.calls() == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    service.shutdown().await;
    let saved = read_records(&path);
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].get("CALL"), Some("K5ABC"));
    assert_eq!(saved[0].get(ADDED_FIELD), None, "saved as received");

    let answering =
        FakeProvider::new(PositionSource::Qrz).found("K5ABC", 39.0, -77.0, Precision::Locator6);
    let service = start_on(&path, &answering, |_| {}).await?;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    let deadline = Instant::now() + Duration::from_secs(2);
    while snapshot(&service).is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(snapshot(&service)["K5ABC"].3, PositionSource::Qrz);
    assert_eq!(answering.calls(), 1);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn a_correction_waiting_at_the_stop_wins_over_the_old_version() -> TestResult {
    let path = file_in(&temp_dir("pending-correction"));
    let release = Arc::new(tokio::sync::Notify::new());
    let provider =
        FakeProvider::new(PositionSource::Qrz).found("K6ABC", 39.0, -77.0, Precision::Locator6);
    let service = start_on(&path, &provider, |c| {
        c.lookup.timeout = Duration::from_secs(30)
    })
    .await?;
    send_udp(service.udp_addr, &adif("K6ABC", "1206", "")).await;
    assert_eq!(wait_for_len(&service, 1).await, 1);
    let provider = provider.answer("K6ABC", Answer::Hang(release));
    send_udp(service.udp_addr, &adif("K6ABC", "1206", "<COMMENT:4>edit ")).await;
    let deadline = Instant::now() + Duration::from_secs(2);
    while provider.calls() < 2 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    service.shutdown().await;
    let saved = read_records(&path);
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].get("COMMENT"), Some("edit"));
    Ok(())
}

// --- live QSOs first (T020) -------------------------------------------------------------------

fn external_file(path: &Path, calls: &[String]) -> std::io::Result<()> {
    let mut text = String::from("<EOH>\n");
    for (i, call) in calls.iter().enumerate() {
        let mut r = AdifRecord::new();
        r.set("CALL", call.as_str());
        r.set("QSO_DATE", "20261010");
        r.set("TIME_ON", format!("{:02}{:02}", 10 + i / 60, i % 60));
        r.set("BAND", "20m");
        r.set("MODE", "CW");
        text.push_str(&live_qso_map::adif::write::to_adif_string(&r));
        text.push('\n');
    }
    std::fs::write(path, text)
}

#[tokio::test]
async fn live_qsos_never_wait_for_restored_lookups() -> TestResult {
    let path = file_in(&temp_dir("live-first"));
    let calls: Vec<String> = (0..20).map(|i| format!("K{i}XYZ")).collect();
    external_file(&path, &calls)?;
    let release = Arc::new(tokio::sync::Notify::new());
    let mut provider = FakeProvider::new(PositionSource::Qrz);
    for call in &calls {
        provider = provider.answer(call, Answer::Hang(Arc::clone(&release)));
    }
    let service = start_on(&path, &provider, |c| {
        c.workers = 1;
        c.lookup.timeout = Duration::from_secs(30);
    })
    .await?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while provider.calls() == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(provider.calls() >= 1, "a restored lookup is in flight");

    send_udp(
        service.udp_addr,
        &adif("DL1ABC", "1200", "<GRIDSQUARE:6>JO62QM "),
    )
    .await;
    assert_eq!(
        wait_for_len(&service, 1).await,
        1,
        "the live QSO did not wait"
    );
    live_qso_map::install_crypto_provider();
    let health = reqwest::get(format!("http://{}/healthz", service.http_addr)).await?;
    assert_eq!(health.status(), 200);
    service.shutdown().await;
    Ok(())
}

// --- scale (T021, SC-003) ---------------------------------------------------------------------

fn saved_file(path: &Path, n: usize) -> std::io::Result<()> {
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        let mut r = AdifRecord::new();
        r.set("CALL", format!("K{}AB", i % 10));
        r.set("QSO_DATE", format!("202610{:02}", 1 + (i / 1440) % 28));
        r.set("TIME_ON", format!("{:02}{:02}", (i / 60) % 24, i % 60));
        r.set(
            "BAND",
            ["20m", "40m", "15m", "10m", "80m", "160m", "30m"][i % 7],
        );
        r.set("MODE", format!("M{}", i / 40_320));
        r.set("STATION_CALLSIGN", "IS0MZK");
        r.set("LAT", "N040 00.000");
        r.set("LON", "W075 00.000");
        r.set("APP_LIVEQSOMAP_POS_SOURCE", "qrz");
        r.set("APP_LIVEQSOMAP_POS_PRECISION", "locator6");
        r.set(
            ADDED_FIELD,
            "APP_LIVEQSOMAP_POS_PRECISION,APP_LIVEQSOMAP_POS_SOURCE,LAT,LON",
        );
        records.push(r);
    }
    std::fs::write(path, live_qso_map::history_file::write::render(&records, 0))
}

#[tokio::test]
async fn ten_thousand_qsos_load_and_save_within_five_seconds() -> TestResult {
    let path = file_in(&temp_dir("scale"));
    saved_file(&path, 10_000)?;
    let provider = us_provider();
    let started = Instant::now();
    let service = start_on(&path, &provider, |_| {}).await?;
    live_qso_map::install_crypto_provider();
    let health = reqwest::get(format!("http://{}/healthz", service.http_addr)).await?;
    assert_eq!(health.status(), 200);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "page after {:?}",
        started.elapsed()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while service.history_len() < 10_000 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(service.history_len(), 10_000);
    assert_eq!(provider.calls(), 0);
    let stopping = Instant::now();
    service.shutdown().await;
    assert!(
        stopping.elapsed() < Duration::from_secs(5),
        "stop took {:?}",
        stopping.elapsed()
    );
    assert_eq!(read_records(&path).len(), 10_000);
    Ok(())
}
