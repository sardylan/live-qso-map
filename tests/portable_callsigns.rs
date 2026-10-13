//! Feature 010: positions for prefixed and portable callsigns, and placeholder positions
//! (contracts/enrichment.md, behaviour table rows 1–18).

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{next_upsert, send_udp, test_config};
use live_qso_map::adif::AdifRecord;
use live_qso_map::enrich::cty::CountryData;
use live_qso_map::history::EnrichedQso;
use live_qso_map::pipeline::{LookupSource, RunningService, cty_source, start_with_country};
use live_qso_map::position::{LookupCandidate, Position, PositionSource, Precision};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CTY: &str = include_str!("fixtures/cty-portable.dat");

const CAGLIARI: (f64, f64) = (39.22, 9.12);
const BERLIN: (f64, f64) = (52.52, 13.4);

/// What a fake provider knows: callsign → (lat, lon, fields). Unknown callsigns answer `None`.
type Answers = HashMap<&'static str, ((f64, f64), Vec<(&'static str, &'static str)>)>;

/// A fake QRZ provider answering from `answers`, counting every call it receives.
fn qrz(answers: Answers, calls: Arc<AtomicUsize>) -> LookupSource {
    let answers = Arc::new(answers);
    Arc::new(move |call: String| {
        calls.fetch_add(1, Ordering::SeqCst);
        let answers = Arc::clone(&answers);
        Box::pin(async move {
            let Some(((lat, lon), fields)) = answers.get(call.as_str()) else {
                return Ok(None);
            };
            let mut record = AdifRecord::new();
            for (name, value) in fields {
                record.set(name, *value);
            }
            Ok(Some(LookupCandidate {
                source: PositionSource::Qrz,
                position: Some(Position::new(
                    *lat,
                    *lon,
                    Precision::Locator6,
                    PositionSource::Qrz,
                )?),
                fields: record,
            }))
        })
    })
}

fn qso(call: &str, minute: u32, extra: &str) -> Vec<u8> {
    format!(
        "<CALL:{}>{call} <QSO_DATE:8>20261013 <TIME_ON:4>15{minute:02} <BAND:3>20m <MODE:2>CW {extra}<EOR>",
        call.len()
    )
    .into_bytes()
}

/// Starts a service with the fake provider and the cty fixture, and returns it with the
/// provider's call counter.
async fn service(
    answers: Answers,
) -> Result<(RunningService, Arc<AtomicUsize>), Box<dyn std::error::Error>> {
    let calls = Arc::new(AtomicUsize::new(0));
    let country = Arc::new(CountryData::parse(CTY));
    let sources = vec![
        qrz(answers, Arc::clone(&calls)),
        cty_source(Arc::clone(&country)),
    ];
    let service = start_with_country(test_config(), Some(sources), Some(country)).await?;
    Ok((service, calls))
}

async fn enriched(
    service: &RunningService,
    call: &str,
    minute: u32,
    extra: &str,
) -> Result<Arc<EnrichedQso>, Box<dyn std::error::Error>> {
    let mut rx = service.subscribe();
    send_udp(service.udp_addr, &qso(call, minute, extra)).await;
    next_upsert(&mut rx, Duration::from_secs(3))
        .await
        .ok_or_else(|| format!("no upsert for {call}").into())
}

fn german_home() -> ((f64, f64), Vec<(&'static str, &'static str)>) {
    (
        BERLIN,
        vec![
            ("NAME", "Hans"),
            ("QTH", "Berlin"),
            ("COUNTRY", "Germany"),
            ("GRIDSQUARE", "JO62QM"),
        ],
    )
}

fn entity_of(q: &EnrichedQso) -> Option<(&str, &str)> {
    q.entity
        .as_ref()
        .map(|e| (e.prefix.as_str(), e.name.as_str()))
}

/// The position is the cty reference point of the entity, at country precision.
fn assert_reference(q: &EnrichedQso) -> TestResult {
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Cty);
    assert_eq!(remote.precision, Precision::Country);
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_SOURCE"), Some("cty"));
    Ok(())
}

// --- User Story 1 -------------------------------------------------------------------------

#[tokio::test]
async fn row_1_prefixed_call_with_a_locator_inside_the_entity_keeps_it() -> TestResult {
    let (service, _) = service(HashMap::from([("IS0/DL0AAA", (CAGLIARI, vec![]))])).await?;
    let q = enriched(&service, "IS0/DL0AAA", 1, "").await?;
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Qrz);
    assert_eq!(remote.precision, Precision::Locator6);
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_2_prefixed_call_at_home_abroad_gets_the_prefix_entity() -> TestResult {
    let (service, _) = service(HashMap::from([("IS0/DL0AAA", german_home())])).await?;
    let q = enriched(&service, "IS0/DL0AAA", 2, "").await?;
    assert_reference(&q)?;
    assert_eq!(
        q.record.get("APP_LIVEQSOMAP_POS_REJECTED"),
        Some("qrz:outside-entity")
    );
    assert_eq!(
        q.record.get("QTH"),
        None,
        "no location field from the rejected record"
    );
    assert_eq!(
        q.record.get("COUNTRY"),
        Some("Sardinia"),
        "from cty, not from the German record"
    );
    assert_ne!(q.record.get("GRIDSQUARE"), Some("JO62QM"));
    assert_eq!(
        q.record.get("NAME"),
        Some("Hans"),
        "non-location fields are kept"
    );
    assert_eq!(q.record.get("CALL"), Some("IS0/DL0AAA"), "FR-010");
    assert_eq!(entity_of(&q), Some(("IS", "Sardinia")), "FR-011");
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_5_prefix_written_after_the_call_behaves_the_same() -> TestResult {
    let (service, _) = service(HashMap::from([("DL0AAA/IS0", german_home())])).await?;
    let q = enriched(&service, "DL0AAA/IS0", 5, "").await?;
    assert_reference(&q)?;
    assert_eq!(
        q.record.get("APP_LIVEQSOMAP_POS_REJECTED"),
        Some("qrz:outside-entity")
    );
    assert_eq!(q.record.get("CALL"), Some("DL0AAA/IS0"));
    assert_eq!(entity_of(&q), Some(("IS", "Sardinia")));
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_6_locator_in_a_nested_entity_is_outside_the_outer_one() -> TestResult {
    let (service, _) = service(HashMap::from([("I/HB9AAA", (CAGLIARI, vec![]))])).await?;
    let q = enriched(&service, "I/HB9AAA", 6, "").await?;
    assert_reference(&q)?;
    assert_eq!(
        q.record.get("APP_LIVEQSOMAP_POS_REJECTED"),
        Some("qrz:outside-entity")
    );
    assert_eq!(q.record.get("CALL"), Some("I/HB9AAA"));
    assert_eq!(entity_of(&q), Some(("I", "Italy")));
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_17_logger_locator_outside_the_entity_is_trusted() -> TestResult {
    let (service, calls) = service(HashMap::new()).await?;
    let q = enriched(&service, "DL0AAA", 17, "<GRIDSQUARE:4>JN18 ").await?;
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Logger);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no lookup when the logger has a position"
    );
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_18_entity_without_a_rectangle_accepts_the_online_position() -> TestResult {
    // An entity with no bundled rectangle: no country data at all gives the same result.
    let calls = Arc::new(AtomicUsize::new(0));
    let sources = vec![qrz(
        HashMap::from([("XX0AAA", (BERLIN, vec![]))]),
        Arc::clone(&calls),
    )];
    let service = start_with_country(test_config(), Some(sources), None).await?;
    let q = enriched(&service, "XX0AAA", 18, "").await?;
    assert_eq!(q.remote.ok_or("no position")?.source, PositionSource::Qrz);
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    service.shutdown().await;
    Ok(())
}

// --- User Story 2 -------------------------------------------------------------------------

const KANSAS_CITY: (f64, f64) = (39.1, -94.6);

#[tokio::test]
async fn row_3_unknown_prefixed_call_retries_the_base_and_judges_it() -> TestResult {
    let (service, calls) = service(HashMap::from([("DL0AAA", german_home())])).await?;
    let q = enriched(&service, "IS0/DL0AAA", 3, "").await?;
    assert_reference(&q)?;
    assert_eq!(
        q.record.get("APP_LIVEQSOMAP_POS_REJECTED"),
        Some("qrz@base:outside-entity")
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "logged call, then base call"
    );
    assert_eq!(q.record.get("CALL"), Some("IS0/DL0AAA"));
    assert_eq!(
        entity_of(&q),
        Some(("IS", "Sardinia")),
        "never the base call's Germany"
    );
    assert_eq!(q.record.get("COUNTRY"), Some("Sardinia"));
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_4_unknown_prefixed_call_uses_a_base_locator_inside_the_entity() -> TestResult {
    let (service, _) = service(HashMap::from([(
        "DL0AAA",
        (CAGLIARI, vec![("NAME", "Hans")]),
    )]))
    .await?;
    let q = enriched(&service, "IS0/DL0AAA", 4, "").await?;
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Qrz);
    assert_eq!(remote.precision, Precision::Locator6);
    assert_eq!(q.record.get("NAME"), Some("Hans"));
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_7_portable_call_with_its_own_record_is_not_retried() -> TestResult {
    let (service, calls) = service(HashMap::from([
        ("IS0AAA/P", (CAGLIARI, vec![("NAME", "Portable")])),
        ("IS0AAA", (CAGLIARI, vec![("NAME", "Home")])),
    ]))
    .await?;
    let q = enriched(&service, "IS0AAA/P", 7, "").await?;
    assert_eq!(q.record.get("NAME"), Some("Portable"));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "no second lookup");
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_8_unknown_portable_call_uses_the_base_record() -> TestResult {
    let (service, calls) = service(HashMap::from([(
        "IS0AAA",
        (CAGLIARI, vec![("NAME", "Home")]),
    )]))
    .await?;
    let q = enriched(&service, "IS0AAA/P", 8, "").await?;
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Qrz);
    assert_eq!(q.record.get("NAME"), Some("Home"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(q.record.get("CALL"), Some("IS0AAA/P"));
    assert_eq!(entity_of(&q), Some(("IS", "Sardinia")));
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_9_call_area_suffix_uses_the_base_record() -> TestResult {
    let (service, _) = service(HashMap::from([("K0AAA", (KANSAS_CITY, vec![]))])).await?;
    let q = enriched(&service, "K0AAA/6", 9, "").await?;
    let remote = q.remote.ok_or("no position")?;
    assert_eq!(remote.source, PositionSource::Qrz);
    assert_eq!(q.record.get("APP_LIVEQSOMAP_POS_REJECTED"), None);
    service.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn row_10_maritime_mobile_is_never_retried() -> TestResult {
    let (service, calls) = service(HashMap::from([("DL0AAA", german_home())])).await?;
    let q = enriched(&service, "DL0AAA/MM", 10, "").await?;
    assert_eq!(q.remote, None, "no entity, no position");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "no base-call lookup");
    service.shutdown().await;
    Ok(())
}
