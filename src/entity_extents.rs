//! Bundled DXCC entity extents (feature 004, research R2).
//!
//! `web/entity_extents.json` is generated from Natural Earth by `tools/entity_extents.py` and
//! embedded in the binary: nothing is downloaded at runtime.

use std::collections::HashMap;

use serde::Deserialize;

use crate::errors::{Error, Result};

const EMBEDDED: &str = include_str!("web/entity_extents.json");

/// One `[west, south, east, north]` box per cty.dat primary prefix (without the WAE `*`).
/// `west > east` means the box crosses the antimeridian.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtentTable {
    extents: HashMap<String, [f64; 4]>,
}

#[derive(Deserialize)]
struct File {
    extents: HashMap<String, Vec<f64>>,
}

fn invalid(message: String) -> Error {
    Error::Config {
        setting: "entity_extents".into(),
        message,
    }
}

impl ExtentTable {
    /// Parses and validates a table; any bad entry rejects the whole table.
    pub fn parse(text: &str) -> Result<Self> {
        let file: File =
            serde_json::from_str(text).map_err(|e| invalid(format!("malformed table: {e}")))?;
        let mut extents = HashMap::with_capacity(file.extents.len());
        for (prefix, values) in file.extents {
            let [w, s, e, n] = <[f64; 4]>::try_from(values.as_slice())
                .map_err(|_| invalid(format!("{prefix}: a box needs 4 numbers")))?;
            let lat_ok = (-90.0..=90.0).contains(&s) && (-90.0..=90.0).contains(&n) && s < n;
            let lon_ok = (-180.0..=180.0).contains(&w) && (-180.0..=180.0).contains(&e);
            if !lat_ok || !lon_ok {
                return Err(invalid(format!("{prefix}: invalid box {w},{s},{e},{n}")));
            }
            extents.insert(prefix, [w, s, e, n]);
        }
        Ok(Self { extents })
    }

    /// The table shipped with the binary.
    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED)
    }

    pub fn get(&self, prefix: &str) -> Option<[f64; 4]> {
        self.extents.get(prefix).copied()
    }

    pub fn len(&self) -> usize {
        self.extents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Error;

    const SAMPLE: &str = r#"{"source":"test","extents":{"IS":[8.13,38.86,9.83,41.31],"3D2":[176.8,-19.3,-178.2,-16.0]}}"#;

    fn contains(b: [f64; 4], lat: f64, lon: f64) -> bool {
        let [w, s, e, n] = b;
        let in_lon = if w <= e {
            (w..=e).contains(&lon)
        } else {
            lon >= w || lon <= e
        };
        in_lon && (s..=n).contains(&lat)
    }

    fn rejected(json: &str) {
        match ExtentTable::parse(json) {
            Err(Error::Config { setting, .. }) => assert_eq!(setting, "entity_extents", "{json}"),
            other => panic!("expected a config error for {json}, got {other:?}"),
        }
    }

    #[test]
    fn parses_boxes_including_antimeridian() {
        let t = ExtentTable::parse(SAMPLE).unwrap();
        assert_eq!(t.get("IS"), Some([8.13, 38.86, 9.83, 41.31]));
        assert_eq!(t.get("3D2"), Some([176.8, -19.3, -178.2, -16.0]));
        assert_eq!(t.get("XX"), None);
    }

    #[test]
    fn rejects_bad_tables() {
        rejected(r#"{"source":"t","extents":{"A":[0,10,1,10]}}"#); // s >= n
        rejected(r#"{"source":"t","extents":{"A":[0,-91,1,10]}}"#);
        rejected(r#"{"source":"t","extents":{"A":[0,0,181,10]}}"#);
        rejected(r#"{"source":"t","extents":{"A":[0,0,1]}}"#);
        rejected(r#"{"source":"t","extents":{"A":[0,0,1,"x"]}}"#);
        rejected("not json");
    }

    #[test]
    fn embedded_table_covers_the_reference_entities() {
        let t = ExtentTable::embedded().unwrap();
        let is = t.get("IS").unwrap();
        let i = t.get("I").unwrap();
        let k = t.get("K").unwrap();
        let vk = t.get("VK").unwrap();
        assert!(contains(is, 39.22, 9.12), "Cagliari in IS");
        assert!(contains(i, 41.9, 12.5), "Rome in I");
        // Sardinia sits inside Italy's box, but the boxes stay distinct: IS is only the island,
        // and I leaves Sicily (IT9) out, so its south edge is mainland Calabria.
        assert!(is[2] < 10.0 && is[0] > 8.0, "IS is the island only");
        assert!(i[1] > 37.5, "Sicily is not in I");
        assert!(contains(k, 39.1, -94.6), "Kansas City in K");
        assert!(!contains(k, 21.3, -157.86), "Honolulu not in K");
        assert!(contains(vk, -33.87, 151.21), "Sydney in VK");
    }
}
