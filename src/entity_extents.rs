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
    /// For each entity, the entities whose box lies entirely inside its own (feature 010, R5).
    nested: HashMap<String, Vec<String>>,
}

/// Whether `[west, south, east, north]` contains the point; `west > east` crosses 180°.
pub fn contains(bounds: [f64; 4], lat: f64, lon: f64) -> bool {
    let [w, s, e, n] = bounds;
    let in_lon = if w <= e {
        (w..=e).contains(&lon)
    } else {
        lon >= w || lon <= e
    };
    in_lon && (s..=n).contains(&lat)
}

/// The longitude intervals of a box, split in two when it crosses 180°.
fn lon_ranges(bounds: [f64; 4]) -> Vec<(f64, f64)> {
    let [w, _, e, _] = bounds;
    if w <= e {
        vec![(w, e)]
    } else {
        vec![(w, 180.0), (-180.0, e)]
    }
}

/// Whether `inner` lies entirely inside `outer` (wrap-aware), and is not the same box.
fn nests(outer: [f64; 4], inner: [f64; 4]) -> bool {
    if outer == inner || inner[1] < outer[1] || inner[3] > outer[3] {
        return false;
    }
    let outer_ranges = lon_ranges(outer);
    lon_ranges(inner)
        .iter()
        .all(|&(iw, ie)| outer_ranges.iter().any(|&(ow, oe)| ow <= iw && ie <= oe))
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
        let mut nested: HashMap<String, Vec<String>> = HashMap::new();
        for (outer, &outer_box) in &extents {
            let mut inner: Vec<String> = extents
                .iter()
                .filter(|&(prefix, &inner_box)| prefix != outer && nests(outer_box, inner_box))
                .map(|(prefix, _)| prefix.clone())
                .collect();
            if !inner.is_empty() {
                inner.sort();
                nested.insert(outer.clone(), inner);
            }
        }
        Ok(Self { extents, nested })
    }

    /// The table shipped with the binary.
    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED)
    }

    pub fn get(&self, prefix: &str) -> Option<[f64; 4]> {
        self.extents.get(prefix).copied()
    }

    /// The entities whose box lies entirely inside `prefix`'s box, sorted; empty when none.
    pub fn nested(&self, prefix: &str) -> &[String] {
        self.nested
            .get(prefix)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Whether the point belongs to `prefix`'s entity: inside its box and inside none of the
    /// boxes nested in it (a point in Cagliari is Sardinia, not Italy). `None` when the entity
    /// has no box.
    pub fn inside(&self, prefix: &str, lat: f64, lon: f64) -> Option<bool> {
        let bounds = self.get(prefix)?;
        let in_nested = self
            .nested(prefix)
            .iter()
            .filter_map(|inner| self.get(inner))
            .any(|inner| contains(inner, lat, lon));
        Some(contains(bounds, lat, lon) && !in_nested)
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

    // --- nesting and inside (feature 010, research R5) -------------------------------------

    const NESTED: &str = r#"{"source":"test","extents":{
        "I":[6.6,37.91,18.52,47.09],"IS":[8.13,38.86,9.83,41.27],"IT9":[12.03,36.65,15.66,38.82],
        "DL":[5.87,47.27,15.04,55.06],"SP":[14.12,49.0,24.15,54.84],
        "3D2":[176.8,-19.3,-178.2,-16.0],"X":[178.0,-18.0,179.0,-17.0]}}"#;

    #[test]
    fn contains_handles_normal_and_antimeridian_boxes() {
        let three_d2 = [176.8, -19.3, -178.2, -16.0];
        assert!(contains(three_d2, -18.0, 178.4), "east of 180");
        assert!(contains(three_d2, -18.0, -179.0), "west of 180");
        assert!(!contains(three_d2, -18.0, 170.0));
        assert!(!contains(three_d2, -10.0, 178.4), "latitude outside");
        assert!(contains([8.13, 38.86, 9.83, 41.27], 39.22, 9.12));
        assert!(!contains([8.13, 38.86, 9.83, 41.27], 39.22, 12.0));
    }

    #[test]
    fn nested_lists_only_fully_contained_boxes() {
        let t = ExtentTable::parse(NESTED).unwrap();
        assert_eq!(
            t.nested("I"),
            ["IS".to_string()].as_slice(),
            "IT9 reaches south of I"
        );
        assert!(t.nested("DL").is_empty(), "SP only overlaps DL");
        assert_eq!(
            t.nested("3D2"),
            ["X".to_string()].as_slice(),
            "wrap-aware nesting"
        );
        assert!(t.nested("??").is_empty());
    }

    #[test]
    fn inside_excludes_nested_entities() {
        let t = ExtentTable::parse(NESTED).unwrap();
        let cagliari = (39.22, 9.12);
        let rome = (41.9, 12.5);
        assert_eq!(
            t.inside("I", cagliari.0, cagliari.1),
            Some(false),
            "Cagliari is Sardinia"
        );
        assert_eq!(t.inside("IS", cagliari.0, cagliari.1), Some(true));
        assert_eq!(t.inside("I", rome.0, rome.1), Some(true));
        assert_eq!(t.inside("IS", rome.0, rome.1), Some(false));
        assert_eq!(
            t.inside("DL", 51.0, 14.5),
            Some(true),
            "eastern Germany stays DL"
        );
        assert_eq!(t.inside("??", 51.0, 14.5), None, "unknown entity");
    }

    #[test]
    fn embedded_table_nests_the_expected_entities() {
        let t = ExtentTable::embedded().unwrap();
        for (outer, inner) in [
            ("I", "IS"),
            ("I", "T7"),
            ("I", "HV"),
            ("I", "TK"),
            ("F", "LX"),
            ("G", "GW"),
        ] {
            assert!(
                t.nested(outer).iter().any(|p| p == inner),
                "{outer} should nest {inner}"
            );
        }
        assert!(
            !t.nested("I").iter().any(|p| p == "IT9"),
            "Sicily is not fully inside Italy"
        );
        assert!(t.nested("DL").is_empty());
    }
}
