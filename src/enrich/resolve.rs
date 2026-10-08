//! Remote position resolution (FR-012 – FR-018, research R11).

use crate::adif::AdifRecord;
use crate::adif::write::{format_lat, format_lon, parse_lat, parse_lon};
use crate::enrich::station::locator_position;
use crate::maidenhead;
use crate::pipeline::LookupSource;
use crate::position::{LookupCandidate, Position, PositionSource, Precision};
use std::time::Duration;

/// The remote position supplied by the logger: `LAT`/`LON`, else a valid `GRIDSQUARE`.
/// When this returns a position, no lookup is made (FR-012).
pub fn logger_position(record: &AdifRecord) -> Option<Position> {
    let lat = record.get("LAT").and_then(parse_lat);
    let lon = record.get("LON").and_then(parse_lon);
    if let (Some(lat), Some(lon)) = (lat, lon) {
        return Position::new(lat, lon, Precision::Coordinates, PositionSource::Logger).ok();
    }
    locator_position(record.get_non_empty("GRIDSQUARE")?, PositionSource::Logger)
}

/// Queries every source concurrently, each bounded by `timeout`, and keeps the most precise
/// position; ties go to the source listed first in `order` (R11). Non-position fields are
/// merged from the winner first, then the others in `order`, never overwriting.
pub async fn resolve_remote(
    call: &str,
    sources: &[LookupSource],
    order: &[PositionSource],
    timeout: Duration,
) -> (Option<Position>, AdifRecord) {
    let handles: Vec<_> = sources
        .iter()
        .map(|source| {
            let lookup = source(call.to_string());
            tokio::spawn(tokio::time::timeout(timeout, lookup))
        })
        .collect();

    let mut candidates = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(Ok(Some(candidate)))) => candidates.push(candidate),
            Ok(Ok(Ok(None))) => {}
            Ok(Ok(Err(e))) => tracing::warn!(call, "lookup failed: {e}"),
            Ok(Err(_)) => tracing::warn!(call, "lookup timed out after {timeout:?}"),
            Err(e) => tracing::warn!(call, "lookup task failed: {e}"),
        }
    }

    let rank = |source: PositionSource| {
        order
            .iter()
            .position(|s| *s == source)
            .unwrap_or(order.len())
    };
    candidates.sort_by_key(|c| rank(c.source));
    let winner = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.position.map(|p| (i, p)))
        .max_by(|(ia, a), (ib, b)| a.precision.cmp(&b.precision).then(ib.cmp(ia)));

    let mut fields = AdifRecord::new();
    let mut merge = |candidate: &LookupCandidate| {
        for (name, value) in candidate.fields.iter() {
            if !value.trim().is_empty() {
                fields.set_if_absent(name, value);
            }
        }
    };
    if let Some((index, _)) = winner {
        merge(&candidates[index]);
    }
    for candidate in &candidates {
        merge(candidate);
    }
    (winner.map(|(_, p)| p), fields)
}

/// Writes the resolved position into the record without overwriting logger values (FR-017)
/// and records its source and precision (FR-018).
pub fn write_position_fields(record: &mut AdifRecord, position: &Position) {
    if position.precision == Precision::Coordinates
        && let Some(grid) = maidenhead::to_locator6(position.lat, position.lon)
    {
        record.set_if_absent("GRIDSQUARE", grid);
    }
    record.set_if_absent("LAT", format_lat(position.lat));
    record.set_if_absent("LON", format_lon(position.lon));
    record.set("APP_LIVEQSOMAP_POS_SOURCE", position.source.as_str());
    record.set("APP_LIVEQSOMAP_POS_PRECISION", position.precision.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, PositionSource, Precision};

    fn record(fields: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in fields {
            r.set(k, *v);
        }
        r
    }

    #[test]
    fn lat_lon_gives_logger_coordinates() {
        let r = record(&[
            ("LAT", "N037 25.200"),
            ("LON", "W122 04.800"),
            ("GRIDSQUARE", "CM87"),
        ]);
        let pos = logger_position(&r).unwrap();
        assert_eq!(pos.precision, Precision::Coordinates);
        assert_eq!(pos.source, PositionSource::Logger);
        assert!((pos.lat - 37.42).abs() < 1e-9 && (pos.lon + 122.08).abs() < 1e-9);
    }

    #[test]
    fn gridsquare_gives_logger_locator() {
        let pos = logger_position(&record(&[("GRIDSQUARE", "JO62QM")])).unwrap();
        assert_eq!(pos.precision, Precision::Locator6);
        assert_eq!(pos.source, PositionSource::Logger);
        let pos = logger_position(&record(&[("GRIDSQUARE", "FN42")])).unwrap();
        assert_eq!(pos.precision, Precision::Locator4);
    }

    #[test]
    fn invalid_gridsquare_gives_nothing() {
        assert_eq!(logger_position(&record(&[("GRIDSQUARE", "ZZ99")])), None);
        assert_eq!(logger_position(&record(&[("LAT", "N037 25.200")])), None);
        assert_eq!(logger_position(&AdifRecord::new()), None);
    }

    #[test]
    fn writes_source_and_precision() {
        let mut r = AdifRecord::new();
        let pos = Position::new(52.5, 13.4, Precision::Locator6, PositionSource::Logger).unwrap();
        write_position_fields(&mut r, &pos);
        assert_eq!(r.get("APP_LIVEQSOMAP_POS_SOURCE"), Some("logger"));
        assert_eq!(r.get("APP_LIVEQSOMAP_POS_PRECISION"), Some("locator6"));
        assert_eq!(r.get("LAT"), Some("N052 30.000"));
        assert_eq!(r.get("LON"), Some("E013 24.000"));
    }

    #[test]
    fn coordinates_also_fill_gridsquare_but_never_overwrite() {
        let mut r = AdifRecord::new();
        let pos =
            Position::new(39.3125, 9.125, Precision::Coordinates, PositionSource::Qrz).unwrap();
        write_position_fields(&mut r, &pos);
        assert_eq!(r.get("GRIDSQUARE"), Some("JM49NH"));

        let mut r = record(&[("GRIDSQUARE", "JM49"), ("LAT", "N039 00.000")]);
        write_position_fields(&mut r, &pos);
        assert_eq!(r.get("GRIDSQUARE"), Some("JM49"));
        assert_eq!(r.get("LAT"), Some("N039 00.000"));
    }

    #[test]
    fn country_precision_does_not_invent_a_gridsquare() {
        let mut r = AdifRecord::new();
        let pos = Position::new(51.0, 10.0, Precision::Country, PositionSource::Cty).unwrap();
        write_position_fields(&mut r, &pos);
        assert_eq!(r.get("GRIDSQUARE"), None);
        assert_eq!(r.get("APP_LIVEQSOMAP_POS_PRECISION"), Some("country"));
    }

    // --- selection (T053) ------------------------------------------------------------------

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    const ORDER: [PositionSource; 4] = [
        PositionSource::Qrz,
        PositionSource::HamQth,
        PositionSource::QrzCq,
        PositionSource::Cty,
    ];

    fn fake(
        source: PositionSource,
        precision: Option<Precision>,
        delay: Duration,
        fields: &[(&str, &str)],
    ) -> LookupSource {
        let mut record = AdifRecord::new();
        for (k, v) in fields {
            record.set(k, *v);
        }
        Arc::new(move |_call: String| {
            let record = record.clone();
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                Ok(Some(LookupCandidate {
                    source,
                    position: precision.map(|p| Position::new(10.0, 20.0, p, source).unwrap()),
                    fields: record,
                }))
            })
        })
    }

    fn failing() -> LookupSource {
        Arc::new(|_call: String| {
            Box::pin(async {
                Err(crate::errors::Error::Provider {
                    provider: "qrz",
                    message: "boom".into(),
                })
            })
        })
    }

    fn empty() -> LookupSource {
        Arc::new(|_call: String| Box::pin(async { Ok(None) }))
    }

    async fn run(sources: &[LookupSource]) -> (Option<Position>, AdifRecord) {
        resolve_remote("K1ABC", sources, &ORDER, Duration::from_millis(200)).await
    }

    const NOW: Duration = Duration::ZERO;

    #[tokio::test]
    async fn highest_precision_wins() {
        let (p, _) = run(&[
            fake(PositionSource::Cty, Some(Precision::Country), NOW, &[]),
            fake(PositionSource::HamQth, Some(Precision::Locator6), NOW, &[]),
            fake(
                PositionSource::QrzCq,
                Some(Precision::Coordinates),
                NOW,
                &[],
            ),
        ])
        .await;
        let p = p.unwrap();
        assert_eq!(p.precision, Precision::Coordinates);
        assert_eq!(p.source, PositionSource::QrzCq);

        let (p, _) = run(&[
            fake(PositionSource::Cty, Some(Precision::Country), NOW, &[]),
            fake(PositionSource::Qrz, Some(Precision::Locator4), NOW, &[]),
        ])
        .await;
        assert_eq!(p.unwrap().precision, Precision::Locator4);
    }

    #[tokio::test]
    async fn ties_go_to_the_configured_order_and_cty_loses() {
        let (p, _) = run(&[
            fake(PositionSource::QrzCq, Some(Precision::Locator6), NOW, &[]),
            fake(PositionSource::HamQth, Some(Precision::Locator6), NOW, &[]),
        ])
        .await;
        assert_eq!(p.unwrap().source, PositionSource::HamQth);

        let (p, _) = run(&[
            fake(PositionSource::Cty, Some(Precision::Country), NOW, &[]),
            fake(PositionSource::QrzCq, Some(Precision::Country), NOW, &[]),
        ])
        .await;
        assert_eq!(p.unwrap().source, PositionSource::QrzCq);
    }

    #[tokio::test]
    async fn slow_sources_are_dropped_within_the_timeout() {
        let begin = Instant::now();
        let (p, _) = run(&[
            fake(
                PositionSource::Qrz,
                Some(Precision::Coordinates),
                Duration::from_secs(5),
                &[],
            ),
            fake(PositionSource::Cty, Some(Precision::Country), NOW, &[]),
        ])
        .await;
        assert!(
            begin.elapsed() < Duration::from_millis(300),
            "{:?}",
            begin.elapsed()
        );
        assert_eq!(p.unwrap().source, PositionSource::Cty);
    }

    #[tokio::test]
    async fn sources_run_in_parallel() {
        let begin = Instant::now();
        let (p, _) = run(&[
            fake(
                PositionSource::Qrz,
                Some(Precision::Country),
                Duration::from_millis(150),
                &[],
            ),
            fake(
                PositionSource::HamQth,
                Some(Precision::Country),
                Duration::from_millis(150),
                &[],
            ),
            fake(
                PositionSource::QrzCq,
                Some(Precision::Country),
                Duration::from_millis(150),
                &[],
            ),
        ])
        .await;
        assert!(
            begin.elapsed() < Duration::from_millis(280),
            "{:?}",
            begin.elapsed()
        );
        assert_eq!(p.unwrap().source, PositionSource::Qrz);
    }

    #[tokio::test]
    async fn failing_and_empty_sources_are_skipped() {
        let (p, _) = run(&[
            failing(),
            empty(),
            fake(PositionSource::Cty, Some(Precision::Country), NOW, &[]),
        ])
        .await;
        assert_eq!(p.unwrap().source, PositionSource::Cty);
    }

    #[tokio::test]
    async fn fields_merge_winner_first_then_order() {
        let (_, fields) = run(&[
            fake(
                PositionSource::Cty,
                Some(Precision::Country),
                NOW,
                &[("COUNTRY", "cty"), ("CQZ", "14"), ("CONT", "EU")],
            ),
            fake(
                PositionSource::HamQth,
                Some(Precision::Locator6),
                NOW,
                &[("COUNTRY", "hamqth"), ("NAME", "Joe")],
            ),
            fake(
                PositionSource::Qrz,
                Some(Precision::Coordinates),
                NOW,
                &[("COUNTRY", "qrz"), ("NAME", "")],
            ),
        ])
        .await;
        assert_eq!(fields.get("COUNTRY"), Some("qrz"));
        assert_eq!(fields.get("NAME"), Some("Joe"));
        assert_eq!(fields.get("CQZ"), Some("14"));
        assert_eq!(fields.get("CONT"), Some("EU"));
    }

    #[tokio::test]
    async fn nothing_found_gives_none() {
        let (p, fields) = run(&[empty(), fake(PositionSource::Qrz, None, NOW, &[])]).await;
        assert!(p.is_none());
        assert!(fields.is_empty());
        let (p, _) = run(&[]).await;
        assert!(p.is_none());
    }

    #[tokio::test]
    async fn every_source_receives_the_callsign() {
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        let source: LookupSource = Arc::new(move |call: String| {
            assert_eq!(call, "K1ABC");
            counter.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(None) })
        });
        run(&[source.clone(), source]).await;
        assert_eq!(seen.load(Ordering::SeqCst), 2);
    }
}
