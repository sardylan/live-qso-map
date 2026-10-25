//! WebSocket protocol messages (contracts/websocket-protocol.md).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::adif::AdifRecord;
use crate::band_filter::BandSet;
use crate::config::{BaseMap, MapConfig, StationConfig};
use crate::history::{EnrichedQso, History, QsoEntity, format_rfc3339, qso_timestamp};
use crate::pipeline::ScoreState;
use crate::position::Position;
use crate::recent_columns::Column;

pub const PROTOCOL_VERSION: u32 = 4;
pub const DEFAULT_LIMIT: usize = 20;
/// The most QSOs a page receives (feature 014, FR-024): the history itself is not limited.
pub const MAX_LIMIT: usize = 1000;
/// Limits a page may select. "All" is gone since feature 014: `null` from an older page is read
/// as [`MAX_LIMIT`].
pub const LIMITS: [Option<usize>; 7] = [
    Some(10),
    Some(20),
    Some(50),
    Some(100),
    Some(200),
    Some(500),
    Some(MAX_LIMIT),
];

pub fn is_allowed_limit(limit: Option<usize>) -> bool {
    LIMITS.contains(&limit)
}

/// One legend entry. Bands travel as an ordered list because JSON objects have no reliable key
/// order, and bands must always appear in frequency order (docs/ham-radio.md HR-2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BandColor {
    pub band: String,
    pub color: String,
}

/// The configured palette in frequency order.
pub fn bands_in_order(bands: &BTreeMap<String, String>) -> Vec<BandColor> {
    let mut list: Vec<BandColor> = bands
        .iter()
        .map(|(band, color)| BandColor {
            band: band.clone(),
            color: color.clone(),
        })
        .collect();
    list.sort_by_key(|b| crate::band::band_rank(&b.band).unwrap_or(usize::MAX));
    list
}

/// The configured station, sent once in `hello` (feature 004, FR-025).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StationView {
    pub callsign: Option<String>,
    pub locator: Option<String>,
    pub position: Option<Position>,
}

/// The station's DXCC/WAE entity and its approximate extent (feature 004, FR-010, HR-6).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntityView {
    pub prefix: String,
    pub name: String,
    /// `[west, south, east, north]`; `west > east` crosses the antimeridian.
    pub bounds: [f64; 4],
}

/// The operator's last-QSO box settings, sent in `hello` (feature 006).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecentView {
    /// Columns in display order: an array, because order matters (API-4).
    pub columns: Vec<Column>,
    pub header: bool,
}

/// The station data every `hello` carries, computed once at startup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HelloStation {
    pub station: Option<StationView>,
    pub entity: Option<EntityView>,
}

impl HelloStation {
    pub fn new(config: &StationConfig, entity: Option<EntityView>) -> Self {
        let callsign = config
            .callsign
            .as_deref()
            .map(|c| c.trim().to_ascii_uppercase())
            .filter(|c| !c.is_empty());
        let locator = config
            .locator
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string);
        let position = crate::enrich::station::config_station(config);
        let station = (callsign.is_some() || locator.is_some() || position.is_some()).then_some(
            StationView {
                callsign,
                locator,
                position,
            },
        );
        Self { station, entity }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        protocol: u32,
        /// The service's software version (feature 018, research R5).
        version: &'static str,
        limits: Vec<Option<usize>>,
        default_limit: usize,
        bands: Vec<BandColor>,
        /// The band groups with at least one considered band (feature 011, FR-012).
        band_groups: Vec<BandGroupView>,
        fallback_color: String,
        basemap: BaseMap,
        station: Option<StationView>,
        entity: Option<EntityView>,
        recent: RecentView,
        /// Degrees of arc every marker keeps inside the globe's horizon (feature 016).
        globe_border_deg: f64,
    },
    Snapshot {
        limit: Option<usize>,
        qsos: Vec<QsoView>,
    },
    Upsert {
        qso: QsoView,
    },
    /// The QSO no longer exists (feature 003).
    Remove {
        id: String,
    },
    /// The latest contest score (feature 007, contracts/websocket-protocol.md).
    Score {
        score: u64,
        qsos: u64,
        mults: u64,
        /// UTC, when the service received the score (HR-1).
        received_at: String,
        /// Seconds since then, so the page can judge staleness on its own clock (research R4).
        age_s: u64,
    },
    /// Band and operator of the most recent QSO by QSO time (feature 017,
    /// contracts/websocket-protocol.md). Every field is `null` with an empty history.
    StationStatus {
        band: Option<String>,
        operator: Option<String>,
        /// UTC, the QSO's date and time (HR-1).
        qso_at: Option<String>,
        /// Seconds from `qso_at` to the service's clock, 0 when `qso_at` is ahead (research R5).
        age_s: Option<u64>,
    },
}

impl ServerMessage {
    /// A `score` message, aged at the moment it is built.
    pub fn score(state: &ScoreState) -> Self {
        ServerMessage::Score {
            score: state.score.score,
            qsos: state.score.qsos,
            mults: state.score.mults,
            received_at: state.received_at.clone(),
            age_s: state.received.elapsed().as_secs(),
        }
    }

    /// The status of the history's most recent QSO, aged at `now` (Unix seconds).
    pub fn station_status(history: &History, fallback: Option<&str>, now: i64) -> Self {
        let latest = history.latest(Some(1));
        let Some(qso) = latest.first() else {
            return ServerMessage::StationStatus {
                band: None,
                operator: None,
                qso_at: None,
                age_s: None,
            };
        };
        let record = &qso.record;
        let ts = qso_timestamp(
            record.get("QSO_DATE").unwrap_or_default(),
            record.get("TIME_ON").unwrap_or_default(),
        );
        // OPERATOR, then the station callsign: a single-op station never sets one (research R4).
        let operator = [
            record.get("OPERATOR"),
            record.get("STATION_CALLSIGN"),
            fallback,
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|c| !c.is_empty())
        .map(str::to_ascii_uppercase);
        ServerMessage::StationStatus {
            band: record.get_non_empty("BAND").map(str::to_string),
            operator,
            qso_at: ts.map(format_rfc3339),
            age_s: ts.map(|ts| u64::try_from(now.saturating_sub(ts)).unwrap_or(0)),
        }
    }

    /// Only bands in `filter` are listed (all when `None`); `protocol` and `basemap` are
    /// unaffected (feature 003, FR-010).
    pub fn hello(map: &MapConfig, filter: Option<&BandSet>, station: &HelloStation) -> Self {
        ServerMessage::Hello {
            protocol: PROTOCOL_VERSION,
            version: env!("CARGO_PKG_VERSION"),
            limits: LIMITS.to_vec(),
            default_limit: DEFAULT_LIMIT,
            bands: bands_in_order(&map.bands)
                .into_iter()
                .filter(|b| filter.is_none_or(|f| f.contains(&b.band)))
                .collect(),
            band_groups: crate::band_filter::considered_groups(filter)
                .into_iter()
                .map(BandGroupView::from)
                .collect(),
            fallback_color: map.fallback_color.clone(),
            basemap: map.basemap(),
            station: station.station.clone(),
            entity: station.entity.clone(),
            recent: RecentView {
                columns: map.recent_columns.clone(),
                header: map.recent_header,
            },
            globe_border_deg: map.globe_border_deg,
        }
    }
}

/// A band group in `hello` (feature 011, contracts/websocket-protocol.md).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BandGroupView {
    pub name: &'static str,
    pub label: &'static str,
    /// Only the considered bands, in frequency order (HR-2).
    pub bands: Vec<&'static str>,
}

impl From<crate::band_filter::ConsideredGroup> for BandGroupView {
    fn from(group: crate::band_filter::ConsideredGroup) -> Self {
        Self {
            name: group.name,
            label: group.label,
            bands: group.bands,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SetLimit {
        limit: Option<usize>,
        /// Only QSOs on these bands in the snapshot; absent or null: every band (feature 011).
        #[serde(default)]
        bands: Option<Vec<String>>,
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
    /// The remote station's DXCC/WAE entity from cty.dat, or `null` (feature 005).
    pub entity: Option<QsoEntity>,
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
            entity: q.entity.clone(),
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
        bands.insert("10m".to_string(), "#393b79".to_string());
        bands.insert("160m".to_string(), "#8c564b".to_string());
        MapConfig {
            style_url: Some("https://style.example/style.json".into()),
            style_attribution: Some("attr".into()),
            tile_url: None,
            tile_attribution: None,
            fallback_color: "#7f7f7f".into(),
            bands,
            recent_columns: crate::recent_columns::DEFAULT_COLUMNS.to_vec(),
            recent_header: true,
            globe_border_deg: 10.0,
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
            entity: None,
        }
    }

    #[test]
    fn hello_shape() {
        let mut v = serde_json::to_value(ServerMessage::hello(
            &map_config(),
            None,
            &HelloStation::default(),
        ))
        .unwrap();
        // Every band considered: every group, with all its bands (feature 011).
        let groups = v.as_object_mut().unwrap().remove("band_groups").unwrap();
        assert_eq!(groups.as_array().unwrap().len(), 8);
        assert_eq!(
            groups[2],
            json!({"name": "hf-contest", "label": "HF contest", "bands": ["160m", "80m", "40m", "20m", "15m", "10m"]})
        );
        assert_eq!(
            v,
            json!({
                "type": "hello",
                "protocol": 4,
                "version": env!("CARGO_PKG_VERSION"),
                "limits": [10, 20, 50, 100, 200, 500, 1000],
                "default_limit": 20,
                "bands": [
                    {"band": "160m", "color": "#8c564b"},
                    {"band": "40m", "color": "#d62728"},
                    {"band": "10m", "color": "#393b79"}
                ],
                "fallback_color": "#7f7f7f",
                "basemap": {"kind": "style", "url": "https://style.example/style.json", "attribution": "attr"},
                "station": null,
                "entity": null,
                "recent": {"columns": ["call", "band", "time", "cqzone", "country"], "header": true},
                "globe_border_deg": 10.0
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
            ClientMessage::SetLimit {
                limit: Some(100),
                bands: None
            }
        );
        assert_eq!(
            ClientMessage::parse(r#"{"type":"set_limit","limit":null}"#),
            ClientMessage::SetLimit {
                limit: None,
                bands: None
            }
        );
        assert_eq!(
            ClientMessage::parse(r#"{"type":"set_limit","limit":20,"bands":["40m","20m"]}"#),
            ClientMessage::SetLimit {
                limit: Some(20),
                bands: Some(vec!["40m".into(), "20m".into()])
            }
        );
        assert_eq!(
            ClientMessage::parse(r#"{"type":"set_limit","limit":20,"bands":null}"#),
            ClientMessage::SetLimit {
                limit: Some(20),
                bands: None
            }
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
        assert!(is_allowed_limit(Some(1000)));
        assert!(
            !is_allowed_limit(None),
            "the session reads null as 1000 first"
        );
        assert!(!is_allowed_limit(Some(7)));
        assert!(!is_allowed_limit(Some(2000)));
    }

    #[test]
    fn raster_basemap_shape() {
        let mut map = map_config();
        map.style_url = None;
        map.style_attribution = None;
        map.tile_url = Some("https://t/{z}/{x}/{y}.png".into());
        map.tile_attribution = Some("tiles".into());
        let v = serde_json::to_value(ServerMessage::hello(&map, None, &HelloStation::default()))
            .unwrap();
        assert_eq!(
            v["basemap"],
            json!({"kind": "raster", "url": "https://t/{z}/{x}/{y}.png", "attribution": "tiles"})
        );
    }

    #[test]
    fn hello_filters_bands_but_keeps_protocol_and_basemap() {
        let mut map = map_config();
        map.bands.clear();
        for (band, color) in [
            ("2m", "#843c39"),
            ("20m", "#1f77b4"),
            ("60m", "#bcbd22"),
            ("160m", "#8c564b"),
        ] {
            map.bands.insert(band.to_string(), color.to_string());
        }
        let contest = crate::band_filter::resolve(&["hf-contest"]).unwrap();
        let filtered = serde_json::to_value(ServerMessage::hello(
            &map,
            Some(&contest),
            &HelloStation::default(),
        ))
        .unwrap();
        let all = serde_json::to_value(ServerMessage::hello(&map, None, &HelloStation::default()))
            .unwrap();
        let bands = |v: &serde_json::Value| -> Vec<String> {
            v["bands"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b["band"].as_str().unwrap().to_string())
                .collect()
        };
        assert_eq!(bands(&filtered), ["160m", "20m"]);
        assert_eq!(bands(&all), ["160m", "60m", "20m", "2m"]);
        for v in [&filtered, &all] {
            assert_eq!(v["protocol"], 4);
            assert_eq!(v["basemap"], serde_json::to_value(map.basemap()).unwrap());
        }
    }

    fn station(
        callsign: Option<&str>,
        locator: Option<&str>,
        lat_lon: Option<(f64, f64)>,
    ) -> crate::config::StationConfig {
        crate::config::StationConfig {
            callsign: callsign.map(str::to_string),
            locator: locator.map(str::to_string),
            latitude: lat_lon.map(|(lat, _)| lat),
            longitude: lat_lon.map(|(_, lon)| lon),
        }
    }

    fn hello_with(station: &HelloStation) -> serde_json::Value {
        serde_json::to_value(ServerMessage::hello(&map_config(), None, station)).unwrap()
    }

    #[test]
    fn hello_station_from_callsign_and_locator() {
        let v = hello_with(&HelloStation::new(
            &station(Some(" is0mzk "), Some("JM49NH"), None),
            None,
        ));
        let (lat, lon) = crate::maidenhead::centre("JM49NH").unwrap();
        assert_eq!(
            v["station"],
            json!({
                "callsign": "IS0MZK",
                "locator": "JM49NH",
                "position": {"lat": lat, "lon": lon, "precision": "locator6", "source": "config"}
            })
        );
        assert_eq!(v["entity"], serde_json::Value::Null);
    }

    #[test]
    fn hello_station_coordinates_win_over_locator() {
        let v = hello_with(&HelloStation::new(
            &station(Some("IS0MZK"), Some("JM49NH"), Some((39.2, 9.1))),
            None,
        ));
        assert_eq!(
            v["station"]["position"],
            json!({"lat": 39.2, "lon": 9.1, "precision": "coordinates", "source": "config"})
        );
    }

    #[test]
    fn hello_station_with_only_a_callsign_has_no_position() {
        let v = hello_with(&HelloStation::new(
            &station(Some("IS0MZK"), None, None),
            None,
        ));
        assert_eq!(
            v["station"],
            json!({"callsign": "IS0MZK", "locator": null, "position": null})
        );
    }

    #[test]
    fn hello_station_is_null_when_nothing_is_configured() {
        let v = hello_with(&HelloStation::new(&station(None, None, None), None));
        assert_eq!(v["station"], serde_json::Value::Null);
    }

    #[test]
    fn hello_entity_shape() {
        let entity = EntityView {
            prefix: "IS".into(),
            name: "Sardinia".into(),
            bounds: [8.13, 38.86, 9.83, 41.31],
        };
        let v = hello_with(&HelloStation::new(
            &station(Some("IS0MZK"), Some("JM49NH"), None),
            Some(entity),
        ));
        assert_eq!(
            v["entity"],
            json!({"prefix": "IS", "name": "Sardinia", "bounds": [8.13, 38.86, 9.83, 41.31]})
        );
        // The rest of hello is unchanged by the station data.
        let plain = hello_with(&HelloStation::default());
        for key in [
            "bands",
            "basemap",
            "limits",
            "default_limit",
            "fallback_color",
        ] {
            assert_eq!(v[key], plain[key], "{key}");
        }
    }

    #[test]
    fn hello_carries_the_operator_box_settings() {
        use crate::recent_columns::Column;
        let mut map = map_config();
        map.recent_columns = vec![Column::Country, Column::Call, Column::Mode];
        map.recent_header = false;
        let v = serde_json::to_value(ServerMessage::hello(&map, None, &HelloStation::default()))
            .unwrap();
        assert_eq!(
            v["recent"],
            json!({"columns": ["country", "call", "mode"], "header": false})
        );
        assert_eq!(v["protocol"], 4);
    }

    #[test]
    fn hello_carries_the_globe_border() {
        let mut map = map_config();
        map.globe_border_deg = 25.5;
        let v = serde_json::to_value(ServerMessage::hello(&map, None, &HelloStation::default()))
            .unwrap();
        assert_eq!(v["globe_border_deg"], 25.5);
        assert_eq!(v["protocol"], 4);
    }

    #[test]
    fn qso_view_carries_the_entity_or_null() {
        let mut q = k6qa();
        q.entity = Some(crate::history::QsoEntity {
            prefix: "IS".into(),
            name: "Sardinia".into(),
            cq_zone: 15,
        });
        let v = serde_json::to_value(QsoView::from(&q)).unwrap();
        assert_eq!(
            v["entity"],
            json!({"prefix": "IS", "name": "Sardinia", "cq_zone": 15})
        );
        q.entity = None;
        let v = serde_json::to_value(QsoView::from(&q)).unwrap();
        assert_eq!(v["entity"], serde_json::Value::Null);
        assert_eq!(
            PROTOCOL_VERSION, 4,
            "protocol 4: hello.band_groups and set_limit.bands"
        );
    }

    #[test]
    fn remove_shape() {
        let v = serde_json::to_value(ServerMessage::Remove {
            id: "L|IS0MZK|3809734F4".into(),
        })
        .unwrap();
        assert_eq!(v, json!({"type": "remove", "id": "L|IS0MZK|3809734F4"}));
    }

    // --- contest score (feature 007, T006) ----------------------------------------------------

    fn score_state(received: std::time::Instant) -> crate::pipeline::ScoreState {
        crate::pipeline::ScoreState {
            score: crate::ingest::score::ContestScore {
                score: 132,
                qsos: 7,
                mults: 11,
            },
            received,
            received_at: "2026-10-08T15:04:29Z".into(),
        }
    }

    #[test]
    fn score_shape_with_its_age() {
        let Some(received) =
            std::time::Instant::now().checked_sub(std::time::Duration::from_secs(42))
        else {
            return; // The monotonic clock started less than 42 s ago: nothing to check.
        };
        let json = serde_json::to_value(ServerMessage::score(&score_state(received))).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "score",
                "score": 132,
                "qsos": 7,
                "mults": 11,
                "received_at": "2026-10-08T15:04:29Z",
                "age_s": 42
            })
        );
    }

    #[test]
    fn a_receive_time_in_the_future_has_age_zero() {
        let future = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let ServerMessage::Score { age_s, .. } = ServerMessage::score(&score_state(future)) else {
            panic!("expected a score");
        };
        assert_eq!(age_s, 0);
    }

    // --- station status (feature 017, T004) ---------------------------------------------------

    fn logged(id: &str, band: Option<&str>, time: &str, fields: &[(&str, &str)]) -> EnrichedQso {
        let mut record = AdifRecord::new();
        record.set("CALL", "DL5QFN");
        record.set("QSO_DATE", "20261008");
        record.set("TIME_ON", time);
        if let Some(band) = band {
            record.set("BAND", band);
        }
        for (k, v) in fields {
            record.set(k, *v);
        }
        EnrichedQso {
            id: id.into(),
            rev: 1,
            sort_ts: crate::history::qso_timestamp("20261008", time).unwrap(),
            record,
            remote: None,
            station: None,
            entity: None,
        }
    }

    fn history_of(qsos: Vec<EnrichedQso>) -> crate::history::History {
        let mut history = crate::history::History::new();
        for qso in qsos {
            history.upsert(qso);
        }
        history
    }

    fn status_json(
        history: &crate::history::History,
        fallback: Option<&str>,
        now: i64,
    ) -> serde_json::Value {
        serde_json::to_value(ServerMessage::station_status(history, fallback, now)).unwrap()
    }

    #[test]
    fn station_status_of_an_empty_history_is_all_null() {
        assert_eq!(
            status_json(&crate::history::History::new(), Some("IS0MZK"), 0),
            json!({"type": "station_status", "band": null, "operator": null, "qso_at": null, "age_s": null})
        );
    }

    #[test]
    fn station_status_follows_the_latest_qso_by_qso_time() {
        let later = logged("B", Some("20m"), "150410", &[("OPERATOR", "IS0XYZ")]);
        let earlier = logged("A", Some("40m"), "150404", &[("OPERATOR", "IK0ABC")]);
        let now = later.sort_ts + 42;
        let history = history_of(vec![later, earlier]);
        assert_eq!(
            status_json(&history, None, now),
            json!({
                "type": "station_status",
                "band": "20m",
                "operator": "IS0XYZ",
                "qso_at": "2026-10-08T15:04:10Z",
                "age_s": 42
            })
        );
    }

    #[test]
    fn station_status_without_a_band_still_has_the_operator() {
        let qso = logged("A", None, "150404", &[("OPERATOR", "IK0ABC")]);
        let now = qso.sort_ts;
        let v = status_json(&history_of(vec![qso]), None, now);
        assert_eq!(v["band"], json!(null));
        assert_eq!(v["operator"], "IK0ABC");
    }

    #[test]
    fn station_status_operator_is_trimmed_and_upper_case() {
        let qso = logged("A", Some("40m"), "150404", &[("OPERATOR", " ik0abc ")]);
        let now = qso.sort_ts;
        assert_eq!(
            status_json(&history_of(vec![qso]), None, now)["operator"],
            "IK0ABC"
        );
    }

    // --- station status: fallback and age (feature 017, T015) ---------------------------------

    fn operator_of(fields: &[(&str, &str)], fallback: Option<&str>) -> serde_json::Value {
        let qso = logged("A", Some("40m"), "150404", fields);
        let now = qso.sort_ts;
        status_json(&history_of(vec![qso]), fallback, now)["operator"].clone()
    }

    #[test]
    fn station_status_operator_falls_back_to_the_station_callsign() {
        let station = [("OPERATOR", ""), ("STATION_CALLSIGN", "is0mzk")];
        assert_eq!(operator_of(&station, Some("IQ0AA")), "IS0MZK");
        let blank = [("OPERATOR", "  "), ("STATION_CALLSIGN", " ")];
        assert_eq!(operator_of(&blank, Some("IQ0AA")), "IQ0AA");
        assert_eq!(operator_of(&blank, None), json!(null));
    }

    #[test]
    fn station_status_of_a_qso_in_the_future_has_age_zero() {
        let qso = logged("A", Some("40m"), "150404", &[]);
        let now = qso.sort_ts - 60;
        assert_eq!(status_json(&history_of(vec![qso]), None, now)["age_s"], 0);
    }

    #[test]
    fn hello_band_groups_follow_the_considered_bands() {
        let hf = crate::band_filter::parse_setting(&["hf".to_string()]).unwrap();
        let v = serde_json::to_value(ServerMessage::hello(
            &map_config(),
            hf.as_ref(),
            &HelloStation::default(),
        ))
        .unwrap();
        let names: Vec<&str> = v["band_groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["hf", "hf-contest", "warc"]);
        assert_eq!(
            v["band_groups"][2],
            json!({"name": "warc", "label": "WARC", "bands": ["30m", "17m", "12m"]})
        );
    }
}
