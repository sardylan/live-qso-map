//! QARTest UDP messages to ADIF (FR-004, FR-004a, FR-005a).

use crate::adif::AdifRecord;
use crate::band::{band_from_metres, band_from_mhz};
use crate::errors::{Error, Result};
use crate::ingest::EventKind;
use crate::ingest::gate::split_timestamp;
use crate::mode::map_mode;
use crate::xml::flatten;

/// A QSO message converted to ADIF, before normalisation and identity.
#[derive(Debug, Clone, PartialEq)]
pub struct QarTestMessage {
    pub kind: EventKind,
    pub record: AdifRecord,
    pub logger_id: Option<String>,
    pub previous: Option<(String, String)>,
}

/// Fields stored for information only (FR-004a).
const INFO_FIELDS: &[&str] = &[
    "countryprefix",
    "nr",
    "points",
    "duplicate",
    "xqso",
    "stationname",
];

/// Parses one QARTest datagram. Score (`dynamicresults`) and delete (`contactdelete`) messages
/// give `None` (FR-005a).
pub fn parse(data: &[u8]) -> Result<Option<QarTestMessage>> {
    let flat = flatten(&String::from_utf8_lossy(data))?;
    let root = flat.root.as_str();
    let kind = match root {
        "contactinfo" => EventKind::New,
        "contactreplace" => EventKind::Correction,
        "dynamicresults" | "contactdelete" => {
            tracing::debug!(message = root, "ignoring QARTest message");
            return Ok(None);
        }
        other => {
            return Err(Error::QarTest {
                element: "root",
                message: format!("unexpected message <{other}>"),
            });
        }
    };
    let get = |name: &str| flat.get(root, name);
    let missing = |element: &'static str| Error::QarTest {
        element,
        message: "missing or invalid".into(),
    };

    let mut record = AdifRecord::new();
    record.set("CALL", get("call").ok_or_else(|| missing("call"))?);
    let (date, time) = get("timestamp")
        .and_then(split_timestamp)
        .ok_or_else(|| missing("timestamp"))?;
    record.set("QSO_DATE", date);
    record.set("TIME_ON", time);
    if let Some(v) = get("mycall") {
        record.set("STATION_CALLSIGN", v);
    }
    if let Some(v) = get("operator") {
        record.set("OPERATOR", v);
    }

    let mhz = get("txfreq")
        .and_then(|f| f.parse::<f64>().ok())
        .filter(|f| *f > 0.0)
        .map(|f| f / 100_000.0);
    match mhz {
        Some(mhz) => {
            record.set("FREQ", format!("{mhz:.6}"));
            if let Some(band) = band_from_mhz(mhz) {
                record.set("BAND", band);
            }
        }
        None => {
            if let Some(band) = get("band").and_then(band_from_metres) {
                record.set("BAND", band);
            }
        }
    }
    if let Some(raw) = get("mode") {
        let (mode, submode) = map_mode(raw);
        record.set("MODE", mode);
        if let Some(submode) = submode {
            record.set("SUBMODE", submode);
        }
    }
    for (element, field) in [
        ("snt", "RST_SENT"),
        ("rcv", "RST_RCVD"),
        ("wpxprefix", "PFX"),
        ("contestname", "CONTEST_ID"),
    ] {
        if let Some(v) = get(element) {
            record.set(field, v);
        }
    }
    let exchange: Vec<&str> = ["exch1", "exch2", "exch3"]
        .iter()
        .filter_map(|e| get(e))
        .collect();
    if !exchange.is_empty() {
        record.set("SRX_STRING", exchange.join(" "));
    }
    if let Some(logger) = get("logger") {
        let (program, version) = logger.split_once(' ').unwrap_or((logger, ""));
        record.set("PROGRAMID", program);
        if !version.trim().is_empty() {
            record.set("PROGRAMVERSION", version.trim());
        }
    }
    let logger_id = get("id").map(str::to_string);
    if let Some(id) = &logger_id {
        record.set("APP_LIVEQSOMAP_LOGGER_ID", id.clone());
    }
    for element in INFO_FIELDS {
        if let Some(v) = get(element) {
            record.set(
                &format!("APP_LIVEQSOMAP_QARTEST_{}", element.to_ascii_uppercase()),
                v,
            );
        }
    }
    let previous = match kind {
        EventKind::Correction => get("oldcall")
            .zip(get("oldtimestamp"))
            .map(|(c, t)| (c.to_string(), t.to_string())),
        EventKind::New => None,
    };
    Ok(Some(QarTestMessage {
        kind,
        record,
        logger_id,
        previous,
    }))
}

/// Splits a capture written by `tools/udp_capture.py` into its datagrams.
#[doc(hidden)]
pub fn reference_packets(capture: &str) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    let mut rest = capture;
    while let Some(start) = rest.find("===== BEGIN PACKET") {
        let after_header = &rest[start..];
        let Some(body_start) = after_header.find('\n') else {
            break;
        };
        let body = &after_header[body_start + 1..];
        let Some(end) = body.find("\n===== END PACKET") else {
            break;
        };
        packets.push(body.as_bytes()[..end].to_vec());
        rest = &body[end..];
    }
    packets
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn packets() -> Vec<Vec<u8>> {
        reference_packets(include_str!(
            "../../specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt"
        ))
    }

    fn message(i: usize) -> QarTestMessage {
        parse(&packets()[i]).unwrap().expect("a QSO message")
    }

    #[test]
    fn packet_1_converts_exactly() {
        let m = message(0);
        assert_eq!(m.kind, EventKind::New);
        assert_eq!(m.logger_id.as_deref(), Some("3807B771C"));
        assert_eq!(m.previous, None);
        let expected: BTreeMap<String, String> = [
            ("CALL", "DL5QFN"),
            ("STATION_CALLSIGN", "IS0MZK"),
            ("QSO_DATE", "20261008"),
            ("TIME_ON", "150404"),
            ("FREQ", "7.000000"),
            ("BAND", "40m"),
            ("MODE", "SSB"),
            ("SUBMODE", "LSB"),
            ("RST_SENT", "59"),
            ("RST_RCVD", "59"),
            ("SRX_STRING", "14"),
            ("PFX", "DL5"),
            ("CONTEST_ID", "CQ-WW-SSB"),
            ("PROGRAMID", "QARTest"),
            ("PROGRAMVERSION", "16.9.1"),
            ("APP_LIVEQSOMAP_LOGGER_ID", "3807B771C"),
            ("APP_LIVEQSOMAP_QARTEST_COUNTRYPREFIX", "DL"),
            ("APP_LIVEQSOMAP_QARTEST_NR", "4"),
            ("APP_LIVEQSOMAP_QARTEST_POINTS", "1"),
            ("APP_LIVEQSOMAP_QARTEST_DUPLICATE", "False"),
            ("APP_LIVEQSOMAP_QARTEST_XQSO", "0"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        assert_eq!(m.record.as_map(), &expected);
    }

    #[test]
    fn replace_packets_are_corrections() {
        for i in 5..=8 {
            let m = message(i);
            assert_eq!(m.kind, EventKind::Correction, "packet {}", i + 1);
            assert!(m.logger_id.is_some());
            assert!(m.previous.is_some());
        }
        let m = message(5);
        assert_eq!(
            m.previous,
            Some(("K6QA".into(), "2026-10-08 15:04:22".into()))
        );
        assert_eq!(m.record.get("SRX_STRING"), Some("15"));
    }

    #[test]
    fn score_and_delete_messages_give_no_event() {
        assert!(parse(&packets()[3]).unwrap().is_none());
        assert!(parse(&packets()[9]).unwrap().is_none());
        let delete = br#"<?xml version="1.0"?><contactdelete><logger>QARTest 16.9.1</logger><timestamp>2026-10-08 15:04:22</timestamp><call>K6QA</call><id>3809734F4</id></contactdelete>"#;
        assert!(parse(delete).unwrap().is_none());
    }

    fn with(replacements: &[(&str, &str)]) -> Vec<u8> {
        let mut text = String::from_utf8(packets()[0].clone()).unwrap();
        for (from, to) in replacements {
            assert!(text.contains(from), "{from}");
            text = text.replace(from, to);
        }
        text.into_bytes()
    }

    #[test]
    fn band_falls_back_to_the_band_element() {
        for freq in ["<txfreq>0</txfreq>", "<txfreq></txfreq>"] {
            let data = with(&[("<txfreq>700000</txfreq>", freq)]);
            let m = parse(&data).unwrap().unwrap();
            assert_eq!(m.record.get("BAND"), Some("40m"));
            assert_eq!(m.record.get("FREQ"), None);
        }
    }

    #[test]
    fn exchanges_are_joined() {
        let data = with(&[
            ("<exch2></exch2>", "<exch2>B</exch2>"),
            ("<exch3></exch3>", "<exch3>C D</exch3>"),
        ]);
        assert_eq!(
            parse(&data).unwrap().unwrap().record.get("SRX_STRING"),
            Some("14 B C D")
        );
    }

    #[test]
    fn xml_entities_are_decoded() {
        let data = with(&[(
            "<stationname></stationname>",
            "<stationname>Run &amp; Mult</stationname>",
        )]);
        let m = parse(&data).unwrap().unwrap();
        assert_eq!(
            m.record.get("APP_LIVEQSOMAP_QARTEST_STATIONNAME"),
            Some("Run & Mult")
        );
    }

    #[test]
    fn missing_call_or_timestamp_is_an_error() {
        let data = with(&[("<call>DL5QFN</call>", "<call></call>")]);
        assert!(matches!(
            parse(&data),
            Err(Error::QarTest {
                element: "call",
                ..
            })
        ));
        let data = with(&[("<timestamp>2026-10-08 15:04:04</timestamp>", "")]);
        assert!(matches!(
            parse(&data),
            Err(Error::QarTest {
                element: "timestamp",
                ..
            })
        ));
        let data = with(&[("2026-10-08 15:04:04", "yesterday")]);
        assert!(matches!(
            parse(&data),
            Err(Error::QarTest {
                element: "timestamp",
                ..
            })
        ));
    }

    #[test]
    fn reference_loader_splits_on_markers() {
        let p = packets();
        assert_eq!(p.len(), 10);
        assert!(p[0].starts_with(b"<?xml"));
        assert!(String::from_utf8_lossy(&p[3]).contains("<dynamicresults>"));
    }
}
