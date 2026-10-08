//! ADIF writer and the ADIF `XDDD MM.MMM` latitude/longitude format.

use super::AdifRecord;

/// Serialises one record, ending with `<EOR>`. Lengths are in bytes.
pub fn to_adif_string(record: &AdifRecord) -> String {
    let mut out = String::new();
    for (name, value) in record.iter() {
        out.push_str(&format!("<{name}:{}>{value} ", value.len()));
    }
    out.push_str("<EOR>");
    out
}

pub fn format_lat(lat: f64) -> String {
    format_coordinate(lat, 'N', 'S')
}

pub fn format_lon(lon: f64) -> String {
    format_coordinate(lon, 'E', 'W')
}

fn format_coordinate(value: f64, positive: char, negative: char) -> String {
    let hemisphere = if value < 0.0 { negative } else { positive };
    // Work in thousandths of a minute so rounding carries into the degrees.
    let thousandths = (value.abs() * 60_000.0).round() as u64;
    let degrees = thousandths / 60_000;
    let rest = thousandths % 60_000;
    format!(
        "{hemisphere}{degrees:03} {:02}.{:03}",
        rest / 1000,
        rest % 1000
    )
}

pub fn parse_lat(text: &str) -> Option<f64> {
    parse_coordinate(text, 'N', 'S', 90.0)
}

pub fn parse_lon(text: &str) -> Option<f64> {
    parse_coordinate(text, 'E', 'W', 180.0)
}

fn parse_coordinate(text: &str, positive: char, negative: char, max: f64) -> Option<f64> {
    let text = text.trim();
    let mut chars = text.chars();
    let hemisphere = chars.next()?.to_ascii_uppercase();
    let sign = if hemisphere == positive {
        1.0
    } else if hemisphere == negative {
        -1.0
    } else {
        return None;
    };
    let (degrees, minutes) = chars.as_str().split_once(' ')?;
    if degrees.len() != 3 || !degrees.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let degrees: f64 = degrees.parse().ok()?;
    let minutes: f64 = minutes.trim().parse().ok()?;
    if !(0.0..60.0).contains(&minutes) {
        return None;
    }
    let value = degrees + minutes / 60.0;
    (value <= max).then_some(sign * value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adif::parse::parse_datagram;

    #[test]
    fn round_trip_keeps_fields() {
        let mut record = AdifRecord::new();
        record.set("CALL", "DL5QFN");
        record.set("NAME", "Jürgen");
        record.set("COMMENT", "<tricky> text");
        let text = to_adif_string(&record);
        let (records, errors) = parse_datagram(text.as_bytes());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(records, vec![record]);
    }

    #[test]
    fn output_length_is_in_bytes() {
        let mut record = AdifRecord::new();
        record.set("NAME", "Jürgen");
        assert!(to_adif_string(&record).contains("<NAME:7>Jürgen"));
    }

    #[test]
    fn formats_lat_lon() {
        assert_eq!(format_lat(39.3125), "N039 18.750");
        assert_eq!(format_lon(-122.08), "W122 04.800");
        assert_eq!(format_lat(-33.5), "S033 30.000");
        assert_eq!(format_lon(9.125), "E009 07.500");
        assert_eq!(format_lat(0.0), "N000 00.000");
    }

    #[test]
    fn formatting_carries_rounded_minutes() {
        // 59.99999 minutes rounds to 60.000 and must carry into the degrees.
        assert_eq!(format_lat(10.0 + 59.99999 / 60.0), "N011 00.000");
    }

    #[test]
    fn parses_lat_lon() {
        let lat = parse_lat("N039 18.750").unwrap();
        assert!((lat - 39.3125).abs() < 1e-9);
        let lon = parse_lon("W122 04.800").unwrap();
        assert!((lon + 122.08).abs() < 1e-9);
        assert!(parse_lat("s033 30.000").unwrap() < 0.0);
    }

    #[test]
    fn rejects_malformed_or_out_of_range() {
        assert_eq!(parse_lat("N091 00.000"), None);
        assert_eq!(parse_lon("E181 00.000"), None);
        assert_eq!(parse_lat("N039 60.000"), None);
        assert_eq!(parse_lat("E039 18.750"), None);
        assert_eq!(parse_lon("N009 07.500"), None);
        assert_eq!(parse_lat("39.3125"), None);
        assert_eq!(parse_lat(""), None);
    }
}
