//! QRZ.com XML data API (research R12).

use super::{LookupOutcome, candidate, encode, fields_from, parse_f64, provider_error};
use crate::errors::{Error, Result};
use crate::maidenhead;
use crate::position::{PositionSource, Precision};
use crate::xml::flatten;

const BASE: &str = "https://xmldata.qrz.com/xml/current/";
const NAME: &str = "qrz";

pub fn login_url(username: &str, password: &str) -> String {
    format!(
        "{BASE}?username={}&password={}&agent={}",
        encode(username),
        encode(password),
        encode(super::AGENT)
    )
}

pub fn lookup_url(key: &str, call: &str) -> String {
    format!("{BASE}?s={}&callsign={}", encode(key), encode(call))
}

pub fn parse_login(xml: &str) -> Result<String> {
    let flat = flatten(xml)?;
    match flat.get("session", "key") {
        Some(key) => Ok(key.to_string()),
        None => Err(provider_error(
            NAME,
            flat.get("session", "error")
                .unwrap_or("login failed: no session key"),
        )),
    }
}

/// Maps QRZ `geoloc` to what its lat/lon is worth (research R11).
fn coordinates_precision(geoloc: Option<&str>, grid: Option<&str>) -> Option<Precision> {
    match geoloc.map(str::to_ascii_lowercase).as_deref() {
        Some("user" | "geocode" | "zip") => Some(Precision::Coordinates),
        Some("state" | "dxcc") => Some(Precision::Country),
        Some(_) => None,
        None if grid.is_some_and(maidenhead::is_valid) => None,
        None => Some(Precision::Coordinates),
    }
}

pub fn parse_lookup(xml: &str) -> Result<LookupOutcome> {
    let flat = flatten(xml)?;
    if flat.has_section("callsign") {
        let get = |f: &str| flat.get("callsign", f);
        let geoloc = get("geoloc");
        let none = geoloc.is_some_and(|g| g.eq_ignore_ascii_case("none"));
        let grid = if none { None } else { get("grid") };
        let lat_lon = parse_f64(get("lat"))
            .zip(parse_f64(get("lon")))
            .filter(|_| !none);
        let name = match (get("fname"), get("name")) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            (first, last) => first.or(last).map(str::to_string),
        };
        let county = get("state")
            .zip(get("county"))
            .map(|(s, c)| format!("{s},{c}"));
        let fields = fields_from(&[
            ("DXCC", get("dxcc")),
            ("COUNTRY", get("land")),
            ("CQZ", get("cqzone")),
            ("ITUZ", get("ituzone")),
            ("NAME", name.as_deref()),
            ("QTH", get("addr2")),
            ("STATE", get("state")),
            ("CNTY", county.as_deref()),
            ("IOTA", get("iota")),
        ]);
        return Ok(LookupOutcome::Found(candidate(
            PositionSource::Qrz,
            lat_lon,
            coordinates_precision(geoloc, grid),
            grid,
            fields,
        )));
    }
    let error = flat.get("session", "error").unwrap_or_default();
    if error.to_ascii_lowercase().starts_with("not found") {
        return Ok(LookupOutcome::NotFound);
    }
    if flat.get("session", "key").is_none() {
        return Ok(LookupOutcome::NeedsLogin);
    }
    Err(Error::Provider {
        provider: NAME,
        message: if error.is_empty() {
            "unexpected response".into()
        } else {
            error.to_string()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enrich::provider::LookupOutcome;
    use crate::position::{PositionSource, Precision};

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/qrz/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn found(name: &str) -> crate::position::LookupCandidate {
        match parse_lookup(&fixture(name)).unwrap() {
            LookupOutcome::Found(c) => c,
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn login_gives_session_key() {
        assert_eq!(
            parse_login(&fixture("login_ok.xml")).unwrap(),
            "2331uf894c4bd29f3923f3bacf02c532d7bd9"
        );
    }

    #[test]
    fn bad_password_is_a_provider_error() {
        let e = parse_login(&fixture("login_bad_password.xml")).unwrap_err();
        assert!(
            matches!(
                e,
                Error::Provider {
                    provider: "qrz",
                    ..
                }
            ),
            "{e:?}"
        );
        assert!(e.to_string().contains("incorrect"));
    }

    #[test]
    fn user_geoloc_is_coordinates_and_fields_are_mapped() {
        let c = found("lookup_user.xml");
        assert_eq!(c.source, PositionSource::Qrz);
        let p = c.position.unwrap();
        assert_eq!(p.precision, Precision::Coordinates);
        assert_eq!(p.source, PositionSource::Qrz);
        assert_eq!((p.lat, p.lon), (37.42, -122.08));
        let f = &c.fields;
        assert_eq!(f.get("GRIDSQUARE"), Some("CM87ui"));
        assert_eq!(f.get("LAT"), Some("N037 25.200"));
        assert_eq!(f.get("LON"), Some("W122 04.800"));
        assert_eq!(f.get("DXCC"), Some("291"));
        assert_eq!(f.get("COUNTRY"), Some("United States"));
        assert_eq!(f.get("CQZ"), Some("3"));
        assert_eq!(f.get("ITUZ"), Some("6"));
        assert_eq!(f.get("NAME"), Some("Test Operator"));
        assert_eq!(f.get("QTH"), Some("Exampleville"));
        assert_eq!(f.get("STATE"), Some("CA"));
        assert_eq!(f.get("CNTY"), Some("CA,Example"));
        assert_eq!(f.get("IOTA"), Some("NA-123"));
    }

    #[test]
    fn grid_geoloc_takes_precision_from_grid_length() {
        let p = found("lookup_grid.xml").position.unwrap();
        assert_eq!(p.precision, Precision::Locator4);
        assert_eq!(found("lookup_grid.xml").fields.get("LAT"), None);
    }

    #[test]
    fn dxcc_geoloc_is_country_level() {
        let c = found("lookup_dxcc.xml");
        assert_eq!(c.position.unwrap().precision, Precision::Country);
        assert_eq!(
            c.fields.get("GRIDSQUARE"),
            None,
            "a DXCC centre grid is not a real locator"
        );
    }

    #[test]
    fn geoloc_none_gives_no_position() {
        let xml =
            fixture("lookup_user.xml").replace("<geoloc>user</geoloc>", "<geoloc>none</geoloc>");
        let LookupOutcome::Found(c) = parse_lookup(&xml).unwrap() else {
            panic!()
        };
        assert!(c.position.is_none());
    }

    #[test]
    fn not_found_and_session_states() {
        assert!(matches!(
            parse_lookup(&fixture("not_found.xml")).unwrap(),
            LookupOutcome::NotFound
        ));
        assert!(matches!(
            parse_lookup(&fixture("session_timeout.xml")).unwrap(),
            LookupOutcome::NeedsLogin
        ));
    }

    #[test]
    fn urls_encode_credentials() {
        let url = login_url("IS0MZK", "p&ss word");
        assert!(url.starts_with("https://xmldata.qrz.com/xml/current/?username=IS0MZK&password=p%26ss%20word&agent=live-qso-map-"));
        assert_eq!(
            lookup_url("KEY", "IS0/DL1ABC"),
            "https://xmldata.qrz.com/xml/current/?s=KEY&callsign=IS0%2FDL1ABC"
        );
    }
}
