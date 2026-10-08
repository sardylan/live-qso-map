//! ADIF 3.1 band plan (research R9).

/// (band, lower edge MHz, upper edge MHz), edges inclusive, in frequency order (HR-2).
const BANDS: &[(&str, f64, f64)] = &[
    ("2190m", 0.1357, 0.1378),
    ("630m", 0.472, 0.479),
    ("560m", 0.501, 0.504),
    ("160m", 1.8, 2.0),
    ("80m", 3.5, 4.0),
    ("60m", 5.06, 5.45),
    ("40m", 7.0, 7.3),
    ("30m", 10.1, 10.15),
    ("20m", 14.0, 14.35),
    ("17m", 18.068, 18.168),
    ("15m", 21.0, 21.45),
    ("12m", 24.89, 24.99),
    ("10m", 28.0, 29.7),
    ("8m", 40.0, 45.0),
    ("6m", 50.0, 54.0),
    ("5m", 54.000001, 69.9),
    ("4m", 70.0, 71.0),
    ("2m", 144.0, 148.0),
    ("1.25m", 222.0, 225.0),
    ("70cm", 420.0, 450.0),
    ("33cm", 902.0, 928.0),
    ("23cm", 1240.0, 1300.0),
    ("13cm", 2300.0, 2450.0),
    ("9cm", 3300.0, 3500.0),
    ("6cm", 5650.0, 5925.0),
    ("3cm", 10000.0, 10500.0),
    ("1.25cm", 24000.0, 24250.0),
    ("6mm", 47000.0, 47200.0),
    ("4mm", 75500.0, 81000.0),
    ("2.5mm", 119980.0, 123000.0),
    ("2mm", 134000.0, 149000.0),
    ("1mm", 241000.0, 250000.0),
    ("submm", 300000.0, 7500000.0),
];

pub fn band_from_mhz(mhz: f64) -> Option<&'static str> {
    BANDS
        .iter()
        .find(|(_, low, high)| (*low..=*high).contains(&mhz))
        .map(|(name, _, _)| *name)
}

/// Converts a band given in metres (e.g. QARTest `<band>40</band>`) to its ADIF name.
pub fn band_from_metres(value: &str) -> Option<&'static str> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    [value.clone(), format!("{value}m"), format!("{value}cm")]
        .iter()
        .find_map(|candidate| canonical_band(candidate))
}

/// Position of the band in frequency order (lowest frequency first), for sorting (HR-2).
pub fn band_rank(name: &str) -> Option<usize> {
    let name = name.trim();
    BANDS
        .iter()
        .position(|(band, _, _)| band.eq_ignore_ascii_case(name))
}

pub fn is_adif_band(name: &str) -> bool {
    canonical_band(name).is_some()
}

/// Returns the ADIF band name matching `name` case-insensitively.
pub fn canonical_band(name: &str) -> Option<&'static str> {
    let name = name.trim();
    BANDS
        .iter()
        .find(|(band, _, _)| band.eq_ignore_ascii_case(name))
        .map(|(band, _, _)| *band)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_from_frequency() {
        assert_eq!(band_from_mhz(7.0), Some("40m"));
        assert_eq!(band_from_mhz(14.2), Some("20m"));
        assert_eq!(band_from_mhz(144.3), Some("2m"));
        assert_eq!(band_from_mhz(432.1), Some("70cm"));
        assert_eq!(band_from_mhz(50.1), Some("6m"));
        assert_eq!(band_from_mhz(1.84), Some("160m"));
    }

    #[test]
    fn band_edges_are_inclusive() {
        assert_eq!(band_from_mhz(7.3), Some("40m"));
        assert_eq!(band_from_mhz(14.35), Some("20m"));
    }

    #[test]
    fn frequency_outside_every_band() {
        assert_eq!(band_from_mhz(5.0), None);
        assert_eq!(band_from_mhz(0.0), None);
        assert_eq!(band_from_mhz(-7.0), None);
    }

    #[test]
    fn band_from_metres_value() {
        assert_eq!(band_from_metres("40"), Some("40m"));
        assert_eq!(band_from_metres("2"), Some("2m"));
        assert_eq!(band_from_metres(" 20 "), Some("20m"));
        assert_eq!(band_from_metres("40m"), Some("40m"));
        assert_eq!(band_from_metres("21"), None);
        assert_eq!(band_from_metres(""), None);
    }

    #[test]
    fn adif_band_names() {
        assert!(is_adif_band("20m"));
        assert!(is_adif_band("70CM"));
        assert!(!is_adif_band("21m"));
    }

    #[test]
    fn bands_rank_by_frequency_not_alphabet() {
        let mut names = vec![
            "10m", "160m", "70cm", "12m", "2m", "15m", "80m", "6m", "40m", "20m",
        ];
        names.sort_by_key(|b| band_rank(b));
        assert_eq!(
            names,
            [
                "160m", "80m", "40m", "20m", "15m", "12m", "10m", "6m", "2m", "70cm"
            ]
        );
        assert!(band_rank("2190m") < band_rank("160m"));
        assert_eq!(band_rank("21m"), None);
        assert_eq!(band_rank("70CM"), band_rank("70cm"));
    }
}
