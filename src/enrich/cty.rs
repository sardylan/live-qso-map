//! AD1C cty.dat country data: parsing, callsign lookup and refresh (research R13).

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::adif::AdifRecord;
use crate::errors::{Error, Result};
use crate::position::{LookupCandidate, Position, PositionSource, Precision};

#[derive(Debug, Clone, PartialEq)]
struct Entity {
    /// Primary prefix, without the `*` that marks WAE entities.
    prefix: String,
    name: String,
    cq: u8,
    itu: u8,
    continent: String,
    lat: f64,
    /// East positive (cty.dat stores west positive).
    lon: f64,
}

/// A callsign's DXCC/WAE entity and CQ zone, borrowed from the country data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityInfo<'a> {
    pub prefix: &'a str,
    pub name: &'a str,
    pub cq_zone: u8,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Overrides {
    cq: Option<u8>,
    itu: Option<u8>,
    lat_lon: Option<(f64, f64)>,
    continent: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CountryData {
    entities: Vec<Entity>,
    exact: HashMap<String, (usize, Overrides)>,
    prefixes: HashMap<String, (usize, Overrides)>,
    max_prefix_len: usize,
}

impl CountryData {
    /// Parses a cty.dat file. Malformed entities are skipped.
    pub fn parse(text: &str) -> Self {
        let mut data = CountryData::default();
        let mut current: Option<usize> = None;
        let mut skipped = 0usize;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if !line.starts_with(char::is_whitespace) {
                current = match parse_header(line) {
                    Some(entity) => {
                        data.entities.push(entity);
                        Some(data.entities.len() - 1)
                    }
                    None => {
                        skipped += 1;
                        None
                    }
                };
                continue;
            }
            let Some(entity) = current else { continue };
            for alias in line
                .split([',', ';'])
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                data.add_alias(entity, alias);
            }
        }
        if skipped > 0 {
            tracing::warn!(skipped, "skipped malformed entities in cty.dat");
        }
        data
    }

    fn add_alias(&mut self, entity: usize, alias: &str) {
        let (exact, alias) = match alias.strip_prefix('=') {
            Some(rest) => (true, rest),
            None => (false, alias),
        };
        let (call, overrides) = split_overrides(alias);
        if call.is_empty() {
            return;
        }
        let call = call.to_ascii_uppercase();
        if exact {
            self.exact.insert(call, (entity, overrides));
        } else {
            self.max_prefix_len = self.max_prefix_len.max(call.len());
            self.prefixes.insert(call, (entity, overrides));
        }
    }

    /// Number of entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// The country-level candidate for `call`, or `None` when no entity matches.
    pub fn lookup(&self, call: &str) -> Option<LookupCandidate> {
        let call = call.trim().to_ascii_uppercase();
        if call.is_empty() {
            return None;
        }
        if let Some((entity, overrides)) = self.exact.get(&call) {
            return self.candidate(*entity, overrides);
        }
        let base = effective_call(&call)?;
        let (entity, overrides) = self.longest_prefix(&base)?;
        self.candidate(entity, overrides)
    }

    /// The entity of `call` and its CQ zone, by the same rules as `lookup`: an exact call wins,
    /// then the longest prefix; a per-callsign zone override replaces the entity's zone
    /// (features 005 and 006).
    pub fn entity_info(&self, call: &str) -> Option<EntityInfo<'_>> {
        let call = call.trim().to_ascii_uppercase();
        if call.is_empty() {
            return None;
        }
        let (entity, overrides) = match self.exact.get(&call) {
            Some((entity, overrides)) => (*entity, overrides),
            None => self.longest_prefix(&effective_call(&call)?)?,
        };
        let e = self.entities.get(entity)?;
        Some(EntityInfo {
            prefix: &e.prefix,
            name: &e.name,
            cq_zone: overrides.cq.unwrap_or(e.cq),
        })
    }

    fn longest_prefix(&self, call: &str) -> Option<(usize, &Overrides)> {
        let max = self.max_prefix_len.min(call.len());
        (1..=max)
            .rev()
            .filter_map(|len| call.get(..len))
            .find_map(|prefix| self.prefixes.get(prefix))
            .map(|(entity, overrides)| (*entity, overrides))
    }

    fn candidate(&self, entity: usize, overrides: &Overrides) -> Option<LookupCandidate> {
        let e = self.entities.get(entity)?;
        let (lat, lon) = overrides.lat_lon.unwrap_or((e.lat, e.lon));
        let mut fields = AdifRecord::new();
        fields.set("COUNTRY", e.name.clone());
        fields.set("CQZ", overrides.cq.unwrap_or(e.cq).to_string());
        fields.set("ITUZ", overrides.itu.unwrap_or(e.itu).to_string());
        fields.set(
            "CONT",
            overrides
                .continent
                .clone()
                .unwrap_or_else(|| e.continent.clone()),
        );
        Some(LookupCandidate {
            source: PositionSource::Cty,
            position: Position::new(lat, lon, Precision::Country, PositionSource::Cty).ok(),
            fields,
        })
    }
}

fn parse_header(line: &str) -> Option<Entity> {
    let fields: Vec<&str> = line.split(':').map(str::trim).collect();
    if fields.len() < 8 || fields[0].is_empty() || fields[7].is_empty() {
        return None;
    }
    let lat: f64 = fields[4].parse().ok()?;
    let lon_west: f64 = fields[5].parse().ok()?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon_west) {
        return None;
    }
    Some(Entity {
        prefix: fields[7].trim_start_matches('*').to_string(),
        name: fields[0].to_string(),
        cq: fields[1].parse().ok()?,
        itu: fields[2].parse().ok()?,
        continent: fields[3].to_string(),
        lat,
        lon: -lon_west,
    })
}

/// Splits `TT1ABC(40)[75]<10.00/-20.00>{AS}~2.0~` into the callsign and its overrides.
fn split_overrides(alias: &str) -> (&str, Overrides) {
    let end = alias.find(['(', '[', '<', '{', '~']).unwrap_or(alias.len());
    let call = &alias[..end];
    let rest = &alias[end..];
    let between = |open: char, close: char| -> Option<&str> {
        let start = rest.find(open)? + open.len_utf8();
        let len = rest[start..].find(close)?;
        Some(&rest[start..start + len])
    };
    let lat_lon = between('<', '>').and_then(|v| {
        let (lat, lon) = v.split_once('/')?;
        Some((
            lat.trim().parse::<f64>().ok()?,
            -lon.trim().parse::<f64>().ok()?,
        ))
    });
    let overrides = Overrides {
        cq: between('(', ')').and_then(|v| v.parse().ok()),
        itu: between('[', ']').and_then(|v| v.parse().ok()),
        lat_lon,
        continent: between('{', '}').map(str::to_string),
    };
    (call, overrides)
}

const IGNORED_SUFFIXES: &[&str] = &["P", "M", "QRP", "QRPP", "A", "B", "LH"];
const NO_ENTITY_SUFFIXES: &[&str] = &["MM", "AM"];

/// The callsign (or prefix) whose longest prefix decides the entity.
fn effective_call(call: &str) -> Option<String> {
    let parts: Vec<&str> = call.split('/').filter(|p| !p.is_empty()).collect();
    if parts.iter().skip(1).any(|p| NO_ENTITY_SUFFIXES.contains(p)) {
        return None;
    }
    let parts: Vec<&str> = parts
        .into_iter()
        .filter(|p| !IGNORED_SUFFIXES.contains(p))
        .collect();
    match parts.as_slice() {
        [] => None,
        [single] => Some(single.to_string()),
        [first, second, ..] => {
            if second.len() == 1 && second.chars().all(|c| c.is_ascii_digit()) {
                return Some(replace_call_area(first, second));
            }
            if first.len() == 1 && first.chars().all(|c| c.is_ascii_digit()) {
                return Some(replace_call_area(second, first));
            }
            Some(
                if second.len() < first.len() {
                    second
                } else {
                    first
                }
                .to_string(),
            )
        }
    }
}

/// `K1ABC` + `6` → `K6ABC`: replaces the first digit (the call area).
fn replace_call_area(call: &str, digit: &str) -> String {
    match call.find(|c: char| c.is_ascii_digit()) {
        Some(i) => format!("{}{digit}{}", &call[..i], &call[i + 1..]),
        None => call.to_string(),
    }
}

/// Loads the country data, downloading it first when the file is missing or older than
/// `max_age_days` (0 = never re-download an existing file). A failed download falls back to
/// the existing file; with no file at all, the country level is disabled.
pub async fn load_or_refresh<F, Fut>(
    path: &Path,
    url: &str,
    max_age_days: u64,
    now: SystemTime,
    fetch: F,
) -> Option<CountryData>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let stale = match modified {
        None => true,
        Some(_) if max_age_days == 0 => false,
        Some(time) => now
            .duration_since(time)
            .map(|age| age >= Duration::from_secs(max_age_days * 86_400))
            .unwrap_or(false),
    };
    if stale {
        match fetch(url.to_string()).await {
            Ok(text) => match save(path, &text) {
                Ok(()) => tracing::info!(path = %path.display(), "downloaded country data"),
                Err(e) => tracing::warn!(path = %path.display(), "cannot save country data: {e}"),
            },
            Err(e) => tracing::warn!("cannot download country data: {e}"),
        }
    }
    match std::fs::read(path) {
        Ok(bytes) => {
            let data = CountryData::parse(&String::from_utf8_lossy(&bytes));
            tracing::info!(entities = data.len(), "loaded country data");
            Some(data)
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), "no country data, country-level positions disabled: {e}");
            None
        }
    }
}

fn save(path: &Path, text: &str) -> Result<()> {
    if text.trim().is_empty() {
        return Err(Error::Provider {
            provider: "cty",
            message: "downloaded file is empty".into(),
        });
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("dat.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Downloads `url` with the shared HTTP client; errors never include the URL.
pub async fn fetch_url(client: reqwest::Client, url: String) -> Result<String> {
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Http(e.without_url()))?
        .error_for_status()
        .map_err(|e| Error::Http(e.without_url()))?;
    response
        .text()
        .await
        .map_err(|e| Error::Http(e.without_url()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{PositionSource, Precision};
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    const EXCERPT: &str = include_str!("../../tests/fixtures/cty-excerpt.dat");
    const OVERRIDES: &str = include_str!("../../tests/fixtures/cty-overrides.dat");

    fn data() -> CountryData {
        CountryData::parse(EXCERPT)
    }

    fn country(call: &str) -> Option<String> {
        data()
            .lookup(call)
            .and_then(|c| c.fields.get("COUNTRY").map(str::to_string))
    }

    const ZONES: &str = include_str!("../../tests/fixtures/cty-zones.dat");

    fn info(data: &CountryData, call: &str) -> Option<(String, String, u8)> {
        data.entity_info(call)
            .map(|e| (e.prefix.to_string(), e.name.to_string(), e.cq_zone))
    }

    fn triple(prefix: &str, name: &str, zone: u8) -> Option<(String, String, u8)> {
        Some((prefix.to_string(), name.to_string(), zone))
    }

    #[test]
    fn entity_info_gives_prefix_name_and_cq_zone() {
        let d = data();
        assert_eq!(info(&d, "K1ABC"), triple("K", "United States", 5));
        assert_eq!(info(&d, "IS0MZK"), triple("IS", "Sardinia", 15));
        // Exact call with its own zone override (=KH6QZ(5)[8] under United States).
        assert_eq!(info(&d, "KH6QZ"), triple("K", "United States", 5));
        assert_eq!(info(&d, "QQ9QQ"), None);
    }

    #[test]
    fn entity_info_applies_prefix_zone_overrides() {
        let z = CountryData::parse(ZONES);
        assert_eq!(info(&z, "VE3ABC"), triple("VE", "Canada", 5));
        assert_eq!(info(&z, "VY1ABC"), triple("VE", "Canada", 1));
        assert_eq!(info(&z, "UA9AA"), triple("UA9", "Asiatic Russia", 17));
        assert_eq!(info(&z, "R8HAA"), triple("UA9", "Asiatic Russia", 18));
    }

    #[test]
    fn entity_info_zone_matches_the_lookup_for_20_callsigns() {
        // SC-007: the zone shown equals the CQZ cty.dat gives for that callsign.
        let excerpt = data();
        let zones = CountryData::parse(ZONES);
        let cases: [(&CountryData, &str); 20] = [
            (&excerpt, "K1ABC"),
            (&excerpt, "W6XYZ"),
            (&excerpt, "KH6XX"),
            (&excerpt, "KH6QZ"),
            (&excerpt, "IS0MZK"),
            (&excerpt, "I2XYZ"),
            (&excerpt, "IT9ABC"),
            (&excerpt, "IT9QZW/4"),
            (&excerpt, "DL1ABC"),
            (&excerpt, "HB9ABC"),
            (&excerpt, "BY1AA"),
            (&excerpt, "KH6/W1XX"),
            (&excerpt, "DL1ABC/P"),
            (&excerpt, "I2QXJ/RM"),
            (&zones, "VE3ABC"),
            (&zones, "VY1ABC"),
            (&zones, "VY1ABC/P"),
            (&zones, "UA9AA"),
            (&zones, "R8HAA"),
            (&zones, "R8H/UA9AA"),
        ];
        for (d, call) in cases {
            let zone = d.entity_info(call).map(|e| e.cq_zone.to_string());
            let cqz = d
                .lookup(call)
                .and_then(|c| c.fields.get("CQZ").map(str::to_string));
            assert!(zone.is_some(), "{call} resolves");
            assert_eq!(zone, cqz, "{call}");
        }
    }

    #[test]
    fn entity_info_resolves_prefix_and_name() {
        let d = data();
        assert_eq!(
            d.entity_info("IS0MZK").map(|e| (e.prefix, e.name)),
            Some(("IS", "Sardinia"))
        );
        assert_eq!(
            d.entity_info("is0mzk").map(|e| (e.prefix, e.name)),
            Some(("IS", "Sardinia"))
        );
        // WAE entity: the `*` marker is not part of the prefix.
        assert_eq!(
            d.entity_info("IT9ABC").map(|e| (e.prefix, e.name)),
            Some(("IT9", "Sicily"))
        );
        assert_eq!(
            d.entity_info("I2XYZ").map(|e| (e.prefix, e.name)),
            Some(("I", "Italy"))
        );
        assert_eq!(
            d.entity_info("K1ABC").map(|e| (e.prefix, e.name)),
            Some(("K", "United States"))
        );
        assert_eq!(
            d.entity_info("KH6XX").map(|e| (e.prefix, e.name)),
            Some(("KH6", "Hawaii"))
        );
        assert_eq!(
            d.entity_info("IS0MZK/P").map(|e| (e.prefix, e.name)),
            Some(("IS", "Sardinia"))
        );
        assert_eq!(d.entity_info("QQ9QQ"), None);
        assert_eq!(d.entity_info(""), None);
    }

    #[test]
    fn entity_info_matches_lookup() {
        let d = data();
        for call in [
            "IS0MZK", "IT9ABC", "I2XYZ", "K1ABC", "KH6XX", "DL1ABC", "HB9XX",
        ] {
            let by_lookup = country(call);
            let by_entity = d.entity_info(call).map(|e| e.name.to_string());
            assert_eq!(by_entity, by_lookup, "{call}");
        }
    }

    #[test]
    fn parses_every_entity() {
        assert_eq!(data().len(), 8);
        assert_eq!(CountryData::parse(OVERRIDES).len(), 2);
    }

    #[test]
    fn longitude_is_converted_to_east_positive() {
        let c = data().lookup("DL1ABC").unwrap();
        let p = c.position.unwrap();
        assert_eq!((p.lat, p.lon), (51.0, 10.0));
        assert_eq!(p.precision, Precision::Country);
        assert_eq!(p.source, PositionSource::Cty);
        assert_eq!(c.source, PositionSource::Cty);
        assert_eq!(c.fields.get("CQZ"), Some("14"));
        assert_eq!(c.fields.get("ITUZ"), Some("28"));
        assert_eq!(c.fields.get("CONT"), Some("EU"));
        assert_eq!(c.fields.get("DXCC"), None);
        let us = data().lookup("K1ABC").unwrap().position.unwrap();
        assert!(us.lon < 0.0, "United States is west: {}", us.lon);
    }

    #[test]
    fn longest_prefix_wins() {
        assert_eq!(country("IS0ABC").as_deref(), Some("Sardinia"));
        assert_eq!(country("IM0ABC").as_deref(), Some("Sardinia"));
        assert_eq!(country("IT9ABC").as_deref(), Some("Sicily"));
        assert_eq!(country("I1ABC").as_deref(), Some("Italy"));
        assert_eq!(country("KH6ABC").as_deref(), Some("Hawaii"));
        assert_eq!(country("BY1AA").as_deref(), Some("China"));
        assert_eq!(country("HB9XYZ").as_deref(), Some("Switzerland"));
    }

    #[test]
    fn exact_callsign_wins_over_prefix() {
        assert_eq!(country("IY0QZ").as_deref(), Some("Sardinia"));
        assert_eq!(country("I2QXJ/RM").as_deref(), Some("Italy"));
        assert_eq!(country("IW0QZD/9").as_deref(), Some("Sicily"));
    }

    #[test]
    fn alias_overrides_apply() {
        let d = CountryData::parse(OVERRIDES);
        let base = d.lookup("TT1XYZ").unwrap();
        assert_eq!(base.fields.get("CQZ"), Some("14"));
        let zones = d.lookup("TT5XYZ").unwrap();
        assert_eq!(zones.fields.get("CQZ"), Some("15"));
        assert_eq!(zones.fields.get("ITUZ"), Some("29"));
        let p = d.lookup("TT6XYZ").unwrap().position.unwrap();
        assert_eq!((p.lat, p.lon), (45.5, 12.25));
        assert_eq!(d.lookup("TT7XYZ").unwrap().fields.get("CONT"), Some("AF"));
        let exact = d.lookup("TT1ABC").unwrap();
        assert_eq!(exact.fields.get("CQZ"), Some("40"));
        assert_eq!(exact.fields.get("ITUZ"), Some("75"));
        assert_eq!(exact.fields.get("CONT"), Some("AS"));
        let p = exact.position.unwrap();
        assert_eq!((p.lat, p.lon), (10.0, 20.0));
    }

    #[test]
    fn real_zone_overrides_apply() {
        assert_eq!(
            data().lookup("3H0ABC").unwrap().fields.get("CQZ"),
            Some("23")
        );
    }

    #[test]
    fn portable_suffixes_are_ignored() {
        for call in ["DL1ABC/P", "DL1ABC/M", "DL1ABC/QRP", "DL1ABC/A"] {
            assert_eq!(
                country(call).as_deref(),
                Some("Fed. Rep. of Germany"),
                "{call}"
            );
        }
    }

    #[test]
    fn maritime_and_aeronautical_have_no_entity() {
        assert!(data().lookup("DL1ABC/MM").is_none());
        assert!(data().lookup("DL1ABC/AM").is_none());
    }

    #[test]
    fn prefix_part_of_a_compound_call_wins() {
        assert_eq!(country("IS0/DL1ABC").as_deref(), Some("Sardinia"));
        assert_eq!(country("DL1ABC/IS0").as_deref(), Some("Sardinia"));
        assert_eq!(country("KH6/K1ABC").as_deref(), Some("Hawaii"));
    }

    #[test]
    fn call_area_digit_suffix_keeps_the_entity() {
        assert_eq!(country("K1ABC/6").as_deref(), Some("United States"));
        assert_eq!(country("I1ABC/0").as_deref(), Some("Italy"));
    }

    #[test]
    fn lowercase_and_whitespace_are_normalised() {
        assert_eq!(country(" is0abc ").as_deref(), Some("Sardinia"));
    }

    #[test]
    fn unknown_callsign_gives_none() {
        assert!(data().lookup("Q1ABC").is_none());
        assert!(data().lookup("").is_none());
        assert!(data().lookup("/").is_none());
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let text = format!(
            "garbage line without colons\nBroken:  xx: 28: EU: 1: 2: 0: BR:\n    BR;\n{OVERRIDES}"
        );
        let d = CountryData::parse(&text);
        assert_eq!(d.len(), 2);
        assert!(d.lookup("BR1AA").is_none());
        assert!(d.lookup("TT1AA").is_some());
    }

    // --- refresh (T049) -------------------------------------------------------------------

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lqm-cty-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cty.dat")
    }

    fn mtime(path: &std::path::Path) -> SystemTime {
        std::fs::metadata(path).unwrap().modified().unwrap()
    }

    const DAY: Duration = Duration::from_secs(86_400);

    #[tokio::test]
    async fn missing_file_is_downloaded_and_saved() {
        let path = temp_path("missing");
        let calls = Cell::new(0);
        let data = load_or_refresh(
            &path,
            "https://example/cty.dat",
            30,
            SystemTime::now(),
            |url| {
                calls.set(calls.get() + 1);
                assert_eq!(url, "https://example/cty.dat");
                async { Ok(OVERRIDES.to_string()) }
            },
        )
        .await
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(data.len(), 2);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), OVERRIDES);
    }

    #[tokio::test]
    async fn young_file_is_not_downloaded() {
        let path = temp_path("young");
        std::fs::write(&path, OVERRIDES).unwrap();
        let now = mtime(&path) + DAY;
        let data = load_or_refresh(&path, "u", 30, now, |_| async { panic!("must not fetch") })
            .await
            .unwrap();
        assert_eq!(data.len(), 2);
    }

    #[tokio::test]
    async fn old_file_is_kept_when_download_fails() {
        let path = temp_path("old");
        std::fs::write(&path, OVERRIDES).unwrap();
        let now = mtime(&path) + DAY * 31;
        let data = load_or_refresh(&path, "u", 30, now, |_| async {
            Err(Error::Provider {
                provider: "cty",
                message: "offline".into(),
            })
        })
        .await
        .unwrap();
        assert_eq!(data.len(), 2);
    }

    #[tokio::test]
    async fn old_file_is_replaced_when_download_succeeds() {
        let path = temp_path("replace");
        std::fs::write(&path, OVERRIDES).unwrap();
        let now = mtime(&path) + DAY * 31;
        let data = load_or_refresh(&path, "u", 30, now, |_| async { Ok(EXCERPT.to_string()) })
            .await
            .unwrap();
        assert_eq!(data.len(), 8);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), EXCERPT);
    }

    #[tokio::test]
    async fn no_file_and_failed_download_gives_none() {
        let path = temp_path("none");
        let data = load_or_refresh(&path, "u", 30, SystemTime::now(), |_| async {
            Err(Error::Provider {
                provider: "cty",
                message: "offline".into(),
            })
        })
        .await;
        assert!(data.is_none());
    }

    #[tokio::test]
    async fn max_age_zero_never_downloads_an_existing_file() {
        let path = temp_path("zero");
        std::fs::write(&path, OVERRIDES).unwrap();
        let now = mtime(&path) + DAY * 3650;
        let data = load_or_refresh(&path, "u", 0, now, |_| async { panic!("must not fetch") })
            .await
            .unwrap();
        assert_eq!(data.len(), 2);
    }
}
