//! WebSocket protocol messages (contracts/websocket-protocol.md).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::adif::AdifRecord;
use crate::config::MapConfig;
use crate::history::{EnrichedQso, format_rfc3339, qso_timestamp};
use crate::position::Position;

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_LIMIT: usize = 20;
/// Limits a page may select; `None` means All.
pub const LIMITS: [Option<usize>; 7] = [
    Some(10),
    Some(20),
    Some(50),
    Some(100),
    Some(200),
    Some(500),
    None,
];

pub fn is_allowed_limit(limit: Option<usize>) -> bool {
    LIMITS.contains(&limit)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tiles {
    pub url: String,
    pub attribution: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        protocol: u32,
        limits: Vec<Option<usize>>,
        default_limit: usize,
        bands: BTreeMap<String, String>,
        fallback_color: String,
        tiles: Tiles,
    },
    Snapshot {
        limit: Option<usize>,
        qsos: Vec<QsoView>,
    },
    Upsert {
        qso: QsoView,
    },
}

impl ServerMessage {
    pub fn hello(map: &MapConfig) -> Self {
        ServerMessage::Hello {
            protocol: PROTOCOL_VERSION,
            limits: LIMITS.to_vec(),
            default_limit: DEFAULT_LIMIT,
            bands: map.bands.clone(),
            fallback_color: map.fallback_color.clone(),
            tiles: Tiles {
                url: map.tile_url.clone(),
                attribution: map.tile_attribution.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SetLimit {
        limit: Option<usize>,
    },
    #[serde(other)]
    Unknown,
}

impl ClientMessage {
    /// Parses a client frame; anything unparseable is `Unknown` and ignored by the session.
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or(ClientMessage::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QsoView {
    pub id: String,
    pub rev: u32,
    pub sort_key: String,
    pub call: Option<String>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub submode: Option<String>,
    pub datetime: Option<String>,
    pub station_call: Option<String>,
    pub remote: Option<Position>,
    pub station: Option<Position>,
    pub adif: BTreeMap<String, String>,
}

impl From<&EnrichedQso> for QsoView {
    fn from(q: &EnrichedQso) -> Self {
        let field = |name: &str| q.record.get_non_empty(name).map(str::to_string);
        let datetime = datetime_of(&q.record);
        QsoView {
            id: q.id.clone(),
            rev: q.rev,
            sort_key: format_rfc3339(q.sort_ts),
            call: field("CALL"),
            band: field("BAND"),
            mode: field("MODE"),
            submode: field("SUBMODE"),
            datetime,
            station_call: field("STATION_CALLSIGN"),
            remote: q.remote,
            station: q.station,
            adif: q.record.as_map().clone(),
        }
    }
}

fn datetime_of(record: &AdifRecord) -> Option<String> {
    let ts = qso_timestamp(record.get("QSO_DATE")?, record.get("TIME_ON")?)?;
    Some(format_rfc3339(ts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MapConfig;
    use crate::position::{Position, PositionSource, Precision};
    use serde_json::json;

    fn map_config() -> MapConfig {
        let mut bands = std::collections::BTreeMap::new();
        bands.insert("40m".to_string(), "#d62728".to_string());
        MapConfig {
            tile_url: "https://tile.example/{z}/{x}/{y}.png".into(),
            tile_attribution: "attr".into(),
            fallback_color: "#7f7f7f".into(),
            bands,
        }
    }

    fn k6qa() -> EnrichedQso {
        let mut record = AdifRecord::new();
        for (k, v) in [
            ("CALL", "K6QA"),
            ("BAND", "40m"),
            ("MODE", "SSB"),
            ("SUBMODE", "LSB"),
            ("QSO_DATE", "20261008"),
            ("TIME_ON", "150422"),
            ("STATION_CALLSIGN", "IS0MZK"),
        ] {
            record.set(k, v);
        }
        EnrichedQso {
            id: "L|IS0MZK|3809734F4".into(),
            rev: 3,
            sort_ts: 1_791_471_862,
            record,
            remote: Some(
                Position::new(37.42, -122.08, Precision::Coordinates, PositionSource::Qrz).unwrap(),
            ),
            station: None,
        }
    }

    #[test]
    fn hello_shape() {
        let v = serde_json::to_value(ServerMessage::hello(&map_config())).unwrap();
        assert_eq!(
            v,
            json!({
                "type": "hello",
                "protocol": 1,
                "limits": [10, 20, 50, 100, 200, 500, null],
                "default_limit": 20,
                "bands": {"40m": "#d62728"},
                "fallback_color": "#7f7f7f",
                "tiles": {"url": "https://tile.example/{z}/{x}/{y}.png", "attribution": "attr"}
            })
        );
    }

    #[test]
    fn qso_view_from_enriched() {
        let view = QsoView::from(&k6qa());
        let v = serde_json::to_value(&view).unwrap();
        assert_eq!(v["id"], "L|IS0MZK|3809734F4");
        assert_eq!(v["rev"], 3);
        assert_eq!(v["sort_key"], "2026-10-08T15:04:22Z");
        assert_eq!(v["datetime"], "2026-10-08T15:04:22Z");
        assert_eq!(v["call"], "K6QA");
        assert_eq!(v["band"], "40m");
        assert_eq!(v["mode"], "SSB");
        assert_eq!(v["submode"], "LSB");
        assert_eq!(v["station_call"], "IS0MZK");
        assert_eq!(
            v["remote"],
            json!({"lat": 37.42, "lon": -122.08, "precision": "coordinates", "source": "qrz"})
        );
        assert_eq!(v["station"], serde_json::Value::Null);
        assert_eq!(v["adif"]["CALL"], "K6QA");
    }

    #[test]
    fn qso_view_missing_optional_fields_are_null() {
        let mut q = k6qa();
        q.record.remove("BAND");
        q.record.remove("SUBMODE");
        q.remote = None;
        let v = serde_json::to_value(QsoView::from(&q)).unwrap();
        assert_eq!(v["band"], serde_json::Value::Null);
        assert_eq!(v["submode"], serde_json::Value::Null);
        assert_eq!(v["remote"], serde_json::Value::Null);
    }

    #[test]
    fn snapshot_and_upsert_shapes() {
        let snapshot = serde_json::to_value(ServerMessage::Snapshot {
            limit: Some(20),
            qsos: vec![QsoView::from(&k6qa())],
        })
        .unwrap();
        assert_eq!(snapshot["type"], "snapshot");
        assert_eq!(snapshot["limit"], 20);
        assert_eq!(snapshot["qsos"][0]["call"], "K6QA");

        let all = serde_json::to_value(ServerMessage::Snapshot {
            limit: None,
            qsos: vec![],
        })
        .unwrap();
        assert_eq!(all["limit"], serde_json::Value::Null);

        let upsert = serde_json::to_value(ServerMessage::Upsert {
            qso: QsoView::from(&k6qa()),
        })
        .unwrap();
        assert_eq!(upsert["type"], "upsert");
        assert_eq!(upsert["qso"]["rev"], 3);
    }

    #[test]
    fn client_messages() {
        assert_eq!(
            ClientMessage::parse(r#"{"type":"set_limit","limit":100}"#),
            ClientMessage::SetLimit { limit: Some(100) }
        );
        assert_eq!(
            ClientMessage::parse(r#"{"type":"set_limit","limit":null}"#),
            ClientMessage::SetLimit { limit: None }
        );
        assert_eq!(
            ClientMessage::parse(r#"{"type":"ping"}"#),
            ClientMessage::Unknown
        );
        assert_eq!(ClientMessage::parse("not json"), ClientMessage::Unknown);
    }

    #[test]
    fn allowed_limits() {
        assert!(is_allowed_limit(Some(10)));
        assert!(is_allowed_limit(Some(500)));
        assert!(is_allowed_limit(None));
        assert!(!is_allowed_limit(Some(7)));
    }
}
