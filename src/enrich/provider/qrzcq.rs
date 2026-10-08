//! QRZCQ XML API (research R12).

use super::{LookupOutcome, candidate, encode, fields_from, parse_f64, provider_error};
use crate::errors::Result;
use crate::position::{PositionSource, Precision};
use crate::xml::flatten;

const BASE: &str = "https://ssl.qrzcq.com/xml";
const NAME: &str = "qrzcq";

pub fn login_url(username: &str, password: &str) -> String {
    format!(
        "{BASE}?username={}&password={}&agent={}",
        encode(username),
        encode(password),
        encode(super::AGENT)
    )
}

pub fn lookup_url(key: &str, call: &str) -> String {
    format!(
        "{BASE}?s={}&callsign={}&agent={}",
        encode(key),
        encode(call),
        encode(super::AGENT)
    )
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

pub fn parse_lookup(xml: &str) -> Result<LookupOutcome> {
    let flat = flatten(xml)?;
    if flat.has_section("callsign") {
        let get = |f: &str| flat.get("callsign", f);
        let lat_lon = parse_f64(get("latitude")).zip(parse_f64(get("longitude")));
        let fields = fields_from(&[
            ("DXCC", get("dxcc")),
            ("COUNTRY", get("country")),
            ("CQZ", get("cq")),
            ("ITUZ", get("itu")),
            ("CONT", get("continent")),
            ("NAME", get("name")),
            ("QTH", get("qth")),
            ("STATE", get("state")),
            ("IOTA", get("iota")),
        ]);
        return Ok(LookupOutcome::Found(candidate(
            PositionSource::QrzCq,
            lat_lon,
            lat_lon.map(|_| Precision::Coordinates),
            get("locator"),
            fields,
        )));
    }
    let error = flat.get("session", "error").unwrap_or_default();
    let lower = error.to_ascii_lowercase();
    if lower.starts_with("not found") {
        Ok(LookupOutcome::NotFound)
    } else if lower.contains("session")
        || (error.is_empty() && flat.get("session", "key").is_none())
    {
        Ok(LookupOutcome::NeedsLogin)
    } else {
        Err(provider_error(
            NAME,
            if error.is_empty() {
                "unexpected response"
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
            "{}/tests/fixtures/qrzcq/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn login_gives_key() {
        assert_eq!(
            parse_login(&fixture("login_ok.xml")).unwrap(),
            "c7d1a5b0f4e94e3a8f7e1d2c3b4a5968"
        );
    }

    #[test]
    fn full_record() {
        let LookupOutcome::Found(c) = parse_lookup(&fixture("lookup_full.xml")).unwrap() else {
            panic!()
        };
        assert_eq!(c.source, PositionSource::QrzCq);
        let p = c.position.unwrap();
        assert_eq!(p.precision, Precision::Coordinates);
        assert_eq!((p.lat, p.lon), (53.3199308, 9.8597973));
        let f = &c.fields;
        assert_eq!(f.get("GRIDSQUARE"), Some("JO43WH"));
        assert_eq!(f.get("DXCC"), Some("230"));
        assert_eq!(f.get("CQZ"), Some("14"));
        assert_eq!(f.get("ITUZ"), Some("28"));
        assert_eq!(f.get("CONT"), Some("EU"));
        assert_eq!(f.get("COUNTRY"), Some("Germany"));
        assert_eq!(f.get("IOTA"), None, "empty elements are absent");
    }

    #[test]
    fn locator_only_gives_locator_precision() {
        let xml = fixture("lookup_full.xml")
            .replace("<latitude>53.3199308</latitude>", "")
            .replace("<longitude>9.8597973</longitude>", "");
        let LookupOutcome::Found(c) = parse_lookup(&xml).unwrap() else {
            panic!()
        };
        assert_eq!(c.position.unwrap().precision, Precision::Locator6);
    }

    #[test]
    fn errors_and_session_states() {
        assert!(matches!(
            parse_lookup(&fixture("not_found.xml")).unwrap(),
            LookupOutcome::NotFound
        ));
        assert!(matches!(
            parse_lookup(&fixture("session_timeout.xml")).unwrap(),
            LookupOutcome::NeedsLogin
        ));
        let e = parse_lookup(&fixture("premium_required.xml")).unwrap_err();
        assert!(matches!(
            e,
            Error::Provider {
                provider: "qrzcq",
                ..
            }
        ));
    }

    #[test]
    fn urls() {
        assert!(login_url("IS0MZK", "x").starts_with(
            "https://ssl.qrzcq.com/xml?username=IS0MZK&password=x&agent=live-qso-map-"
        ));
        assert!(
            lookup_url("K", "DO1XYZ")
                .starts_with("https://ssl.qrzcq.com/xml?s=K&callsign=DO1XYZ&agent=live-qso-map-")
        );
    }
}
