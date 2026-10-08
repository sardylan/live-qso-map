//! Maidenhead locator conversion for 4, 6 and 8 characters (research R10).

use crate::position::Precision;

/// Returns `(lat, lon)` of the centre of the locator's square, or `None` when it is invalid.
pub fn centre(locator: &str) -> Option<(f64, f64)> {
    let chars: Vec<char> = locator
        .trim()
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if ![4, 6, 8].contains(&chars.len()) {
        return None;
    }
    let mut lon = -180.0;
    let mut lat = -90.0;
    let mut lon_step = 20.0;
    let mut lat_step = 10.0;

    lon += f64::from(index(chars[0], 'A', 18)?) * lon_step;
    lat += f64::from(index(chars[1], 'A', 18)?) * lat_step;

    lon_step /= 10.0;
    lat_step /= 10.0;
    lon += f64::from(index(chars[2], '0', 10)?) * lon_step;
    lat += f64::from(index(chars[3], '0', 10)?) * lat_step;

    if chars.len() >= 6 {
        lon_step /= 24.0;
        lat_step /= 24.0;
        lon += f64::from(index(chars[4], 'A', 24)?) * lon_step;
        lat += f64::from(index(chars[5], 'A', 24)?) * lat_step;
    }
    if chars.len() == 8 {
        lon_step /= 10.0;
        lat_step /= 10.0;
        lon += f64::from(index(chars[6], '0', 10)?) * lon_step;
        lat += f64::from(index(chars[7], '0', 10)?) * lat_step;
    }
    Some((lat + lat_step / 2.0, lon + lon_step / 2.0))
}

fn index(c: char, base: char, count: u32) -> Option<u32> {
    let i = u32::from(c).checked_sub(u32::from(base))?;
    (i < count).then_some(i)
}

pub fn is_valid(locator: &str) -> bool {
    centre(locator).is_some()
}

/// 4 characters is `Locator4`; 6 or 8 characters is `Locator6` ("6-character or better").
pub fn precision_of(locator: &str) -> Option<Precision> {
    centre(locator)?;
    match locator.trim().len() {
        4 => Some(Precision::Locator4),
        _ => Some(Precision::Locator6),
    }
}

/// Converts coordinates to an uppercase 6-character locator.
pub fn to_locator6(lat: f64, lon: f64) -> Option<String> {
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    // Clamp the upper edges into the last square.
    let lon = (lon + 180.0).min(359.999_999);
    let lat = (lat + 90.0).min(179.999_999);
    let letter = |base: char, i: f64| char::from_u32(u32::from(base) + i as u32);
    let field_lon = (lon / 20.0).floor();
    let field_lat = (lat / 10.0).floor();
    let square_lon = ((lon - field_lon * 20.0) / 2.0).floor();
    let square_lat = (lat - field_lat * 10.0).floor();
    let sub_lon = ((lon - field_lon * 20.0 - square_lon * 2.0) * 12.0).floor();
    let sub_lat = ((lat - field_lat * 10.0 - square_lat) * 24.0).floor();
    Some(
        [
            letter('A', field_lon)?,
            letter('A', field_lat)?,
            letter('0', square_lon)?,
            letter('0', square_lat)?,
            letter('A', sub_lon)?,
            letter('A', sub_lat)?,
        ]
        .iter()
        .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Precision;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn centre_of_six_char_locator() {
        assert!(close(centre("JM49NH").unwrap(), (39.3125, 9.125)));
    }

    #[test]
    fn centre_of_four_char_locator() {
        assert!(close(centre("FN42").unwrap(), (42.5, -71.0)));
    }

    #[test]
    fn lowercase_is_accepted() {
        assert_eq!(centre("jm49nh"), centre("JM49NH"));
    }

    #[test]
    fn eight_char_locator_uses_its_own_centre() {
        let c = centre("JM49NH52").unwrap();
        // subsquare NH starts at (39.2917, 9.0833); ext square 5,2 → +5/120°, +2/240°, plus half.
        assert!(close(
            c,
            (
                39.2917 + 2.0 / 240.0 + 1.0 / 480.0,
                9.0833 + 5.0 / 120.0 + 1.0 / 240.0
            )
        ));
    }

    #[test]
    fn invalid_lengths() {
        for loc in ["JM", "JM4", "JM49N", "JM49NH5", "JM49NH5212", ""] {
            assert_eq!(centre(loc), None, "{loc}");
            assert!(!is_valid(loc), "{loc}");
        }
    }

    #[test]
    fn invalid_characters() {
        for loc in ["SS00", "JM4X", "JM49OZ", "JM49NHA1", "ZZ99"] {
            assert_eq!(centre(loc), None, "{loc}");
        }
    }

    #[test]
    fn to_six_char_locator() {
        assert_eq!(to_locator6(39.3125, 9.125), Some("JM49NH".to_string()));
        assert_eq!(to_locator6(42.5, -71.0), Some("FN42MM".to_string()));
        assert_eq!(to_locator6(91.0, 0.0), None);
    }

    #[test]
    fn precision_by_length() {
        assert_eq!(precision_of("JM49"), Some(Precision::Locator4));
        assert_eq!(precision_of("JM49NH"), Some(Precision::Locator6));
        assert_eq!(precision_of("JM49NH52"), Some(Precision::Locator6));
        assert_eq!(precision_of("JM4"), None);
    }
}
