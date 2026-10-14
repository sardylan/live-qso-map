//! Positions, their precision and source, and lookup candidates (data-model).

use serde::Serialize;

use crate::adif::AdifRecord;
use crate::errors::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Precision {
    Country,
    Locator4,
    Locator6,
    Coordinates,
}

impl Precision {
    pub fn as_str(self) -> &'static str {
        match self {
            Precision::Country => "country",
            Precision::Locator4 => "locator4",
            Precision::Locator6 => "locator6",
            Precision::Coordinates => "coordinates",
        }
    }

    /// The inverse of [`Precision::as_str`] (feature 014: restoring a saved position).
    pub fn parse(text: &str) -> Option<Self> {
        [
            Precision::Country,
            Precision::Locator4,
            Precision::Locator6,
            Precision::Coordinates,
        ]
        .into_iter()
        .find(|p| p.as_str() == text.trim())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSource {
    Logger,
    Qrz,
    HamQth,
    QrzCq,
    Cty,
    Config,
}

impl PositionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            PositionSource::Logger => "logger",
            PositionSource::Qrz => "qrz",
            PositionSource::HamQth => "hamqth",
            PositionSource::QrzCq => "qrzcq",
            PositionSource::Cty => "cty",
            PositionSource::Config => "config",
        }
    }

    /// The inverse of [`PositionSource::as_str`] (feature 014: restoring a saved position).
    pub fn parse(text: &str) -> Option<Self> {
        [
            PositionSource::Logger,
            PositionSource::Qrz,
            PositionSource::HamQth,
            PositionSource::QrzCq,
            PositionSource::Cty,
            PositionSource::Config,
        ]
        .into_iter()
        .find(|s| s.as_str() == text.trim())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub precision: Precision,
    pub source: PositionSource,
}

impl Position {
    pub fn new(lat: f64, lon: f64, precision: Precision, source: PositionSource) -> Result<Self> {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(Error::InvalidPosition { lat, lon });
        }
        Ok(Self {
            lat,
            lon,
            precision,
            source,
        })
    }
}

/// The result of one lookup source for one callsign (data-model "LookupCandidate").
#[derive(Debug, Clone, PartialEq)]
pub struct LookupCandidate {
    pub source: PositionSource,
    pub position: Option<Position>,
    pub fields: AdifRecord,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_ordering() {
        assert!(Precision::Country < Precision::Locator4);
        assert!(Precision::Locator4 < Precision::Locator6);
        assert!(Precision::Locator6 < Precision::Coordinates);
    }

    #[test]
    fn position_validates_range() {
        assert!(Position::new(39.2, 9.2, Precision::Locator6, PositionSource::Logger).is_ok());
        for (lat, lon) in [
            (90.5, 0.0),
            (-91.0, 0.0),
            (0.0, 180.5),
            (0.0, -181.0),
            (f64::NAN, 0.0),
        ] {
            let r = Position::new(lat, lon, Precision::Coordinates, PositionSource::Qrz);
            assert!(
                matches!(r, Err(Error::InvalidPosition { .. })),
                "{lat},{lon}"
            );
        }
    }

    #[test]
    fn precision_strings() {
        assert_eq!(Precision::Coordinates.as_str(), "coordinates");
        assert_eq!(Precision::Locator6.as_str(), "locator6");
        assert_eq!(Precision::Locator4.as_str(), "locator4");
        assert_eq!(Precision::Country.as_str(), "country");
        for p in [
            Precision::Country,
            Precision::Locator4,
            Precision::Locator6,
            Precision::Coordinates,
        ] {
            assert_eq!(Precision::parse(p.as_str()), Some(p));
        }
        assert_eq!(Precision::parse("exact"), None);
        assert_eq!(PositionSource::parse("telepathy"), None);
        assert_eq!(
            serde_json::to_string(&Precision::Locator6).unwrap(),
            "\"locator6\""
        );
    }

    #[test]
    fn source_strings() {
        let all = [
            (PositionSource::Logger, "logger"),
            (PositionSource::Qrz, "qrz"),
            (PositionSource::HamQth, "hamqth"),
            (PositionSource::QrzCq, "qrzcq"),
            (PositionSource::Cty, "cty"),
            (PositionSource::Config, "config"),
        ];
        for (source, s) in all {
            assert_eq!(source.as_str(), s);
            assert_eq!(PositionSource::parse(s), Some(source));
            assert_eq!(serde_json::to_string(&source).unwrap(), format!("\"{s}\""));
        }
    }

    #[test]
    fn position_serializes_for_the_protocol() {
        let p = Position::new(37.42, -122.08, Precision::Coordinates, PositionSource::Qrz).unwrap();
        let v = serde_json::to_value(p).unwrap();
        assert_eq!(v["lat"], 37.42);
        assert_eq!(v["lon"], -122.08);
        assert_eq!(v["precision"], "coordinates");
        assert_eq!(v["source"], "qrz");
    }
}
