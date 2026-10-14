//! Restoring records from the history file (feature 014, R3–R8, data-model "Restored record").

use crate::adif::AdifRecord;
use crate::adif::write::{parse_lat, parse_lon};
use crate::band_filter::BandSet;
use crate::history::qso_timestamp;
use crate::history_file::{ADDED_FIELD, LOOKUP_FAILED_FIELD, POS_REJECTED_FIELD};
use crate::position::{Position, PositionSource, Precision};

/// Which program is loading the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The service: records on bands not considered are parked; rejected positions are kept.
    Service,
    /// `--enrich-only`: every band is completed; rejected positions are looked up again.
    EnrichOnly,
}

/// What happens to one loaded record (data-model table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreAction {
    /// No valid `CALL`, `QSO_DATE` and `TIME_ON`: skipped and counted.
    Skip,
    /// On a band not considered (service only): written back verbatim, never queued.
    Park,
    /// A saved position, used without a lookup.
    Reuse,
    /// Enriched from the record as received, like a live QSO.
    Enrich,
}

/// The record as received: without the fields listed in `APP_LIVEQSOMAP_ADDED`, nor the list.
/// A record without the list is returned unchanged (R3).
pub fn as_received(record: &AdifRecord) -> AdifRecord {
    let mut out = record.clone();
    if let Some(list) = out.remove(ADDED_FIELD) {
        for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            out.remove(name);
        }
    }
    out
}

/// Decides what to do with a loaded record (R4, R6, R7, R8).
pub fn classify(record: &AdifRecord, mode: Mode, filter: Option<&BandSet>) -> RestoreAction {
    let valid = record.get_non_empty("CALL").is_some()
        && record
            .get_non_empty("QSO_DATE")
            .zip(record.get_non_empty("TIME_ON"))
            .and_then(|(date, time)| qso_timestamp(date, time))
            .is_some();
    if !valid {
        return RestoreAction::Skip;
    }
    if mode == Mode::Service
        && let Some(set) = filter
        && !band(record).is_some_and(|b| set.contains(&b))
    {
        return RestoreAction::Park;
    }
    if record.get_non_empty(LOOKUP_FAILED_FIELD).is_some() {
        return RestoreAction::Enrich;
    }
    if mode == Mode::EnrichOnly && record.get_non_empty(POS_REJECTED_FIELD).is_some() {
        return RestoreAction::Enrich;
    }
    match saved_position(record) {
        Some(_) => RestoreAction::Reuse,
        None => RestoreAction::Enrich,
    }
}

/// `BAND`, or the band of `FREQ` (HR-3), as ingest normalises it.
fn band(record: &AdifRecord) -> Option<String> {
    record
        .get_non_empty("BAND")
        .map(str::to_string)
        .or_else(|| {
            record
                .get_non_empty("FREQ")?
                .trim()
                .parse::<f64>()
                .ok()
                .and_then(crate::band::band_from_mhz)
                .map(str::to_string)
        })
}

/// The saved remote position: `LAT`/`LON` with the saved precision and source (HR-7). `None`
/// unless all four are present and valid.
pub fn saved_position(record: &AdifRecord) -> Option<Position> {
    let source = PositionSource::parse(record.get("APP_LIVEQSOMAP_POS_SOURCE")?)?;
    let precision = Precision::parse(record.get("APP_LIVEQSOMAP_POS_PRECISION")?)?;
    let lat = record.get("LAT").and_then(parse_lat)?;
    let lon = record.get("LON").and_then(parse_lon)?;
    Position::new(lat, lon, precision, source).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adif::AdifRecord;
    use crate::band_filter::default_set;
    use crate::history_file::{ADDED_FIELD, LOOKUP_FAILED_FIELD, POS_REJECTED_FIELD};
    use crate::position::{PositionSource, Precision};

    fn record(fields: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in [
            ("CALL", "K2ABC"),
            ("QSO_DATE", "20261010"),
            ("TIME_ON", "1203"),
            ("BAND", "20m"),
        ]
        .iter()
        .chain(fields)
        {
            r.set(k, *v);
        }
        r
    }

    fn saved(extra: &[(&str, &str)]) -> AdifRecord {
        let mut fields = vec![
            ("LAT", "N040 00.000"),
            ("LON", "W075 00.000"),
            ("QTH", "Philadelphia"),
            ("APP_LIVEQSOMAP_POS_SOURCE", "qrz"),
            ("APP_LIVEQSOMAP_POS_PRECISION", "locator6"),
        ];
        fields.extend_from_slice(extra);
        let mut r = record(&fields);
        r.set(
            ADDED_FIELD,
            "APP_LIVEQSOMAP_POS_PRECISION,APP_LIVEQSOMAP_POS_SOURCE,LAT,LON,QTH",
        );
        r
    }

    #[test]
    fn as_received_drops_exactly_the_listed_fields() {
        let back = as_received(&saved(&[]));
        assert_eq!(back, record(&[]));
        let external = record(&[("GRIDSQUARE", "FN20")]);
        assert_eq!(
            as_received(&external),
            external,
            "no list: all the logger's"
        );
    }

    #[test]
    fn classification_table() {
        let hf = Some(default_set());
        let service = |r: &AdifRecord| classify(r, Mode::Service, hf.as_ref());
        let enrich_only = |r: &AdifRecord| classify(r, Mode::EnrichOnly, hf.as_ref());

        let mut invalid = record(&[]);
        invalid.remove("TIME_ON");
        assert_eq!(service(&invalid), RestoreAction::Skip);
        assert_eq!(enrich_only(&invalid), RestoreAction::Skip);

        let vhf = record(&[("BAND", "2m")]);
        assert_eq!(service(&vhf), RestoreAction::Park);
        assert_eq!(enrich_only(&vhf), RestoreAction::Enrich);
        let mut from_freq = record(&[("FREQ", "144.300")]);
        from_freq.remove("BAND");
        assert_eq!(service(&from_freq), RestoreAction::Park, "band from FREQ");
        assert_eq!(classify(&vhf, Mode::Service, None), RestoreAction::Enrich);

        let failed = saved(&[(LOOKUP_FAILED_FIELD, "qrz:timeout")]);
        assert_eq!(service(&failed), RestoreAction::Enrich);
        assert_eq!(enrich_only(&failed), RestoreAction::Enrich);

        let rejected = saved(&[(POS_REJECTED_FIELD, "qrz:outside-entity")]);
        assert_eq!(service(&rejected), RestoreAction::Reuse);
        assert_eq!(enrich_only(&rejected), RestoreAction::Enrich);
        let mut rejected_unplaced = rejected.clone();
        rejected_unplaced.remove("LAT");
        assert_eq!(service(&rejected_unplaced), RestoreAction::Enrich);

        assert_eq!(service(&saved(&[])), RestoreAction::Reuse);
        assert_eq!(enrich_only(&saved(&[])), RestoreAction::Reuse);

        let external = record(&[("GRIDSQUARE", "FN20")]);
        assert_eq!(service(&external), RestoreAction::Enrich);
        assert_eq!(enrich_only(&external), RestoreAction::Enrich);
    }

    #[test]
    fn saved_position_keeps_precision_and_source() {
        let p = saved_position(&saved(&[])).unwrap();
        assert_eq!(
            (p.precision, p.source),
            (Precision::Locator6, PositionSource::Qrz)
        );
        assert!((p.lat - 40.0).abs() < 1e-9 && (p.lon + 75.0).abs() < 1e-9);

        let country = record(&[
            ("LAT", "N046 57.000"),
            ("LON", "E007 27.000"),
            ("APP_LIVEQSOMAP_POS_SOURCE", "cty"),
            ("APP_LIVEQSOMAP_POS_PRECISION", "country"),
        ]);
        let p = saved_position(&country).unwrap();
        assert_eq!(
            (p.precision, p.source),
            (Precision::Country, PositionSource::Cty)
        );

        let unknown = record(&[
            ("LAT", "N046 57.000"),
            ("LON", "E007 27.000"),
            ("APP_LIVEQSOMAP_POS_SOURCE", "telepathy"),
            ("APP_LIVEQSOMAP_POS_PRECISION", "country"),
        ]);
        assert_eq!(saved_position(&unknown), None);
        assert_eq!(saved_position(&record(&[("GRIDSQUARE", "FN20")])), None);
    }
}
