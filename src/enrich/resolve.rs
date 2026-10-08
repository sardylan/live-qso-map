//! Remote position resolution (FR-012 – FR-018, research R11).

use crate::adif::AdifRecord;
use crate::adif::write::{format_lat, format_lon, parse_lat, parse_lon};
use crate::enrich::station::locator_position;
use crate::maidenhead;
use crate::position::{Position, PositionSource, Precision};

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
}
