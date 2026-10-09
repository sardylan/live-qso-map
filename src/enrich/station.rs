//! Station position from the record, then from the configured defaults (FR-011).

use crate::adif::AdifRecord;
use crate::adif::write::{parse_lat, parse_lon};
use crate::config::StationConfig;
use crate::maidenhead;
use crate::position::{Position, PositionSource, Precision};

/// Fills `MY_GRIDSQUARE` from the configuration when absent and returns the station position:
/// `MY_LAT`/`MY_LON`, then the record's `MY_GRIDSQUARE`, then the configured coordinates, then
/// the configured locator. `STATION_CALLSIGN` is filled at ingest, not here.
pub fn apply_station(record: &mut AdifRecord, station: &StationConfig) -> Option<Position> {
    let position = logger_station(record).or_else(|| config_station(station));
    if let Some(locator) = &station.locator {
        record.set_if_absent("MY_GRIDSQUARE", locator.to_ascii_uppercase());
    }
    if let Some(p) = &position {
        record.set("APP_LIVEQSOMAP_STATION_POS_SOURCE", p.source.as_str());
    }
    position
}

fn logger_station(record: &AdifRecord) -> Option<Position> {
    let lat = record.get("MY_LAT").and_then(parse_lat);
    let lon = record.get("MY_LON").and_then(parse_lon);
    if let (Some(lat), Some(lon)) = (lat, lon) {
        return Position::new(lat, lon, Precision::Coordinates, PositionSource::Logger).ok();
    }
    let grid = record.get_non_empty("MY_GRIDSQUARE")?;
    locator_position(grid, PositionSource::Logger)
}

pub(crate) fn config_station(station: &StationConfig) -> Option<Position> {
    if let (Some(lat), Some(lon)) = (station.latitude, station.longitude) {
        return Position::new(lat, lon, Precision::Coordinates, PositionSource::Config).ok();
    }
    locator_position(station.locator.as_deref()?, PositionSource::Config)
}

pub(crate) fn locator_position(locator: &str, source: PositionSource) -> Option<Position> {
    let (lat, lon) = maidenhead::centre(locator)?;
    let precision = maidenhead::precision_of(locator)?;
    Position::new(lat, lon, precision, source).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Precision;

    fn record(fields: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in fields {
            r.set(k, *v);
        }
        r
    }

    fn config(locator: Option<&str>, lat_lon: Option<(f64, f64)>) -> StationConfig {
        StationConfig {
            callsign: Some("IS0MZK".into()),
            locator: locator.map(str::to_string),
            latitude: lat_lon.map(|p| p.0),
            longitude: lat_lon.map(|p| p.1),
        }
    }

    #[test]
    fn record_values_win_over_config() {
        let mut r = record(&[("MY_GRIDSQUARE", "JN61")]);
        let pos = apply_station(&mut r, &config(Some("JM49NH"), None)).unwrap();
        assert_eq!(r.get("MY_GRIDSQUARE"), Some("JN61"));
        assert_eq!(pos.source, PositionSource::Logger);
        assert_eq!(pos.precision, Precision::Locator4);
    }

    #[test]
    fn config_fills_my_gridsquare_when_absent() {
        let mut r = record(&[("CALL", "K1AA")]);
        apply_station(&mut r, &config(Some("JM49NH"), None));
        assert_eq!(r.get("MY_GRIDSQUARE"), Some("JM49NH"));
        assert_eq!(
            r.get("STATION_CALLSIGN"),
            None,
            "filled at ingest, not here"
        );
    }

    #[test]
    fn my_lat_lon_comes_first() {
        let mut r = record(&[
            ("MY_LAT", "N039 18.750"),
            ("MY_LON", "E009 07.500"),
            ("MY_GRIDSQUARE", "JN61"),
        ]);
        let pos = apply_station(&mut r, &config(Some("FN42"), Some((1.0, 2.0)))).unwrap();
        assert_eq!(pos.precision, Precision::Coordinates);
        assert_eq!(pos.source, PositionSource::Logger);
        assert!((pos.lat - 39.3125).abs() < 1e-6 && (pos.lon - 9.125).abs() < 1e-6);
        assert_eq!(r.get("APP_LIVEQSOMAP_STATION_POS_SOURCE"), Some("logger"));
    }

    #[test]
    fn then_my_gridsquare_from_the_record() {
        let mut r = record(&[("MY_GRIDSQUARE", "JM49NH")]);
        let pos = apply_station(&mut r, &config(None, Some((1.0, 2.0)))).unwrap();
        assert_eq!(pos.source, PositionSource::Logger);
        assert_eq!(pos.precision, Precision::Locator6);
    }

    #[test]
    fn then_config_coordinates() {
        let mut r = AdifRecord::new();
        let pos = apply_station(&mut r, &config(Some("FN42"), Some((39.2, 9.2)))).unwrap();
        assert_eq!(pos.source, PositionSource::Config);
        assert_eq!(pos.precision, Precision::Coordinates);
        assert_eq!((pos.lat, pos.lon), (39.2, 9.2));
        assert_eq!(r.get("APP_LIVEQSOMAP_STATION_POS_SOURCE"), Some("config"));
    }

    #[test]
    fn then_config_locator() {
        let mut r = AdifRecord::new();
        let pos = apply_station(&mut r, &config(Some("FN42"), None)).unwrap();
        assert_eq!(pos.source, PositionSource::Config);
        assert_eq!(pos.precision, Precision::Locator4);
        assert_eq!((pos.lat, pos.lon), (42.5, -71.0));
    }

    #[test]
    fn none_without_any_station_position() {
        let mut r = record(&[("MY_GRIDSQUARE", "ZZ99")]);
        assert_eq!(apply_station(&mut r, &StationConfig::default()), None);
        assert_eq!(r.get("APP_LIVEQSOMAP_STATION_POS_SOURCE"), None);
    }
}
