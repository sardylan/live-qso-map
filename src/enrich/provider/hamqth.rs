//! HamQTH XML callbook (research R12).

use super::{LookupOutcome, candidate, encode, fields_from, parse_f64, provider_error};
use crate::errors::Result;
use crate::position::{PositionSource, Precision};
use crate::xml::flatten;

const BASE: &str = "https://www.hamqth.com/xml.php";
const NAME: &str = "hamqth";

pub fn login_url(username: &str, password: &str) -> String {
    format!("{BASE}?u={}&p={}", encode(username), encode(password))
}

pub fn lookup_url(session_id: &str, call: &str) -> String {
    format!(
        "{BASE}?id={}&callsign={}&prg=liveqsomap",
        encode(session_id),
        encode(call)
    )
}

pub fn parse_login(xml: &str) -> Result<String> {
    let flat = flatten(xml)?;
    match flat.get("session", "session_id") {
        Some(id) => Ok(id.to_string()),
        None => Err(provider_error(
            NAME,
            flat.get("session", "error")
                .unwrap_or("login failed: no session id"),
        )),
    }
}

pub fn parse_lookup(xml: &str) -> Result<LookupOutcome> {
    let flat = flatten(xml)?;
    if flat.has_section("search") {
        let get = |f: &str| flat.get("search", f);
        let lat_lon = parse_f64(get("latitude")).zip(parse_f64(get("longitude")));
        let fields = fields_from(&[
            ("DXCC", get("adif")),
            ("COUNTRY", get("country")),
            ("CQZ", get("cq")),
            ("ITUZ", get("itu")),
            ("CONT", get("continent")),
            ("NAME", get("nick")),
            ("QTH", get("qth")),
            ("STATE", get("us_state")),
            ("IOTA", get("iota")),
        ]);
        return Ok(LookupOutcome::Found(candidate(
            PositionSource::HamQth,
            lat_lon,
            lat_lon.map(|_| Precision::Coordinates),
            get("grid"),
            fields,
        )));
    }
    let error = flat
        .get("session", "error")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if error.contains("not found") {
        Ok(LookupOutcome::NotFound)
    } else if error.contains("session") {
        Ok(LookupOutcome::NeedsLogin)
    } else {
        Err(provider_error(
            NAME,
            if error.is_empty() {
                "unexpected response".to_string()
            } else {
                error
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enrich::provider::LookupOutcome;
    use crate::errors::Error;
    use crate::position::{PositionSource, Precision};

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/hamqth/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn login_gives_session_id() {
        assert_eq!(
            parse_login(&fixture("login_ok.xml")).unwrap(),
            "09b0ae90050be03c452ad235a1f2915ad684393c"
        );
        let e = parse_login(&fixture("login_bad_password.xml")).unwrap_err();
        assert!(matches!(
            e,
            Error::Provider {
                provider: "hamqth",
                ..
            }
        ));
    }

    #[test]
    fn full_record_with_namespace() {
        let LookupOutcome::Found(c) = parse_lookup(&fixture("lookup_full.xml")).unwrap() else {
            panic!()
        };
        assert_eq!(c.source, PositionSource::HamQth);
        let p = c.position.unwrap();
        assert_eq!(p.precision, Precision::Coordinates);
        assert_eq!((p.lat, p.lon), (50.07, 14.42));
        let f = &c.fields;
        assert_eq!(f.get("GRIDSQUARE"), Some("jo70gg"));
        assert_eq!(f.get("DXCC"), Some("503"));
        assert_eq!(f.get("CQZ"), Some("15"));
        assert_eq!(f.get("ITUZ"), Some("28"));
        assert_eq!(f.get("CONT"), Some("EU"));
        assert_eq!(f.get("COUNTRY"), Some("Czech Republic"));
        assert_eq!(f.get("QTH"), Some("Exampletown"));
        assert_eq!(f.get("NAME"), Some("Test"));
        assert_eq!(f.get("IOTA"), Some("EU-000"));
    }

    #[test]
    fn grid_only_gives_locator_precision() {
        let LookupOutcome::Found(c) = parse_lookup(&fixture("lookup_grid_only.xml")).unwrap()
        else {
            panic!()
        };
        assert_eq!(c.position.unwrap().precision, Precision::Locator4);
    }

    #[test]
    fn not_found_and_expired_session() {
        assert!(matches!(
            parse_lookup(&fixture("not_found.xml")).unwrap(),
            LookupOutcome::NotFound
        ));
        assert!(matches!(
            parse_lookup(&fixture("session_expired.xml")).unwrap(),
            LookupOutcome::NeedsLogin
        ));
    }

    #[test]
    fn urls() {
        assert_eq!(
            login_url("IS0MZK", "a b"),
            "https://www.hamqth.com/xml.php?u=IS0MZK&p=a%20b"
        );
        assert_eq!(
            lookup_url("SID", "OK2XYZ"),
            "https://www.hamqth.com/xml.php?id=SID&callsign=OK2XYZ&prg=liveqsomap"
        );
    }
}
