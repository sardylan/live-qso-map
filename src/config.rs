//! Configuration: command line, then `LQM_*` environment, then TOML file, then defaults
//! (FR-032, FR-033, contracts/configuration.md).

use crate::recent_columns::{Column, DEFAULT_COLUMNS, parse as parse_columns};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::band::{band_rank, canonical_band};
use crate::band_filter::{BandSet, parse_setting};
use crate::errors::{Error, Result};
use crate::maidenhead;

/// Built-in band palette (T023).
const DEFAULT_BANDS: &[(&str, &str)] = &[
    ("160m", "#8c564b"),
    ("80m", "#9467bd"),
    ("60m", "#bcbd22"),
    ("40m", "#d62728"),
    ("30m", "#17becf"),
    ("20m", "#1f77b4"),
    ("17m", "#ff7f0e"),
    ("15m", "#2ca02c"),
    ("12m", "#e377c2"),
    ("10m", "#393b79"),
    ("6m", "#637939"),
    ("2m", "#843c39"),
    ("70cm", "#7b4173"),
];

/// Default base map: OpenFreeMap Liberty (feature 002, research R2).
pub const DEFAULT_STYLE_URL: &str = "https://tiles.openfreemap.org/styles/liberty";

/// OpenFreeMap's required credit: its style carries no attribution of its own (FR-009).
pub const DEFAULT_STYLE_ATTRIBUTION: &str = "<a href=\"https://openfreemap.org\" target=\"_blank\">OpenFreeMap</a> \u{a9} <a href=\"https://www.openmaptiles.org/\" target=\"_blank\">OpenMapTiles</a> Data from <a href=\"https://www.openstreetmap.org/copyright\" target=\"_blank\">OpenStreetMap</a>";

const LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];

/// Command-line options. Every option also has an `LQM_*` environment variable, read by
/// [`Config::load`]; the command line wins.
#[derive(Debug, Default, Parser)]
#[command(name = "live-qso-map", version, about = "Live map of contest QSOs")]
pub struct Cli {
    /// TOML configuration file [env: LQM_CONFIG]
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Address for the UDP and HTTP listeners [env: LQM_BIND_ADDRESS] [default: 0.0.0.0]
    #[arg(long)]
    pub bind_address: Option<IpAddr>,
    /// UDP port for QSO messages [env: LQM_UDP_PORT] [default: 12060]
    #[arg(long)]
    pub udp_port: Option<u32>,
    /// HTTP port for the map [env: LQM_HTTP_PORT] [default: 8080]
    #[arg(long)]
    pub http_port: Option<u32>,
    /// QSOs kept in memory, 0 = unbounded [env: LQM_HISTORY_SIZE] [default: 10000]
    #[arg(long)]
    pub history_size: Option<usize>,
    /// Enrichment workers [env: LQM_WORKERS] [default: 1]
    #[arg(long)]
    pub workers: Option<usize>,
    /// Capacity of the ingest queue [env: LQM_INGEST_QUEUE] [default: 1024]
    #[arg(long)]
    pub ingest_queue: Option<usize>,
    /// Capacity of the broadcast queue [env: LQM_BROADCAST_QUEUE] [default: 1024]
    #[arg(long)]
    pub broadcast_queue: Option<usize>,
    /// error, warn, info, debug or trace [env: LQM_LOG_LEVEL] [default: info]
    #[arg(long)]
    pub log_level: Option<String>,
    /// Default station callsign [env: LQM_STATION_CALLSIGN]
    #[arg(long)]
    pub station_callsign: Option<String>,
    /// Default station locator [env: LQM_STATION_LOCATOR]
    #[arg(long)]
    pub station_locator: Option<String>,
    /// Default station latitude [env: LQM_STATION_LATITUDE]
    #[arg(long, allow_hyphen_values = true)]
    pub station_latitude: Option<f64>,
    /// Default station longitude [env: LQM_STATION_LONGITUDE]
    #[arg(long, allow_hyphen_values = true)]
    pub station_longitude: Option<f64>,
    /// Lookup timeout per provider, seconds [env: LQM_LOOKUP_TIMEOUT] [default: 5]
    #[arg(long)]
    pub lookup_timeout: Option<u64>,
    /// Provider order for ties, comma-separated [env: LQM_PROVIDER_ORDER] [default: qrz,hamqth,qrzcq]
    #[arg(long)]
    pub provider_order: Option<String>,
    /// [env: LQM_QRZ_USERNAME]
    #[arg(long)]
    pub qrz_username: Option<String>,
    /// [env: LQM_QRZ_PASSWORD]
    #[arg(long)]
    pub qrz_password: Option<String>,
    /// [env: LQM_HAMQTH_USERNAME]
    #[arg(long)]
    pub hamqth_username: Option<String>,
    /// [env: LQM_HAMQTH_PASSWORD]
    #[arg(long)]
    pub hamqth_password: Option<String>,
    /// [env: LQM_QRZCQ_USERNAME]
    #[arg(long)]
    pub qrzcq_username: Option<String>,
    /// [env: LQM_QRZCQ_PASSWORD]
    #[arg(long)]
    pub qrzcq_password: Option<String>,
    /// cty.dat location [env: LQM_CTY_PATH] [default: ./cty.dat]
    #[arg(long)]
    pub cty_path: Option<PathBuf>,
    /// cty.dat download URL [env: LQM_CTY_URL]
    #[arg(long)]
    pub cty_url: Option<String>,
    /// Re-download cty.dat after this many days, 0 = never [env: LQM_CTY_MAX_AGE_DAYS] [default: 30]
    #[arg(long)]
    pub cty_max_age_days: Option<u64>,
    /// Base map style URL, MapLibre style JSON [env: LQM_STYLE_URL] [default: OpenFreeMap Liberty]
    #[arg(long)]
    pub style_url: Option<String>,
    /// Base map style attribution, HTML [env: LQM_STYLE_ATTRIBUTION]
    #[arg(long)]
    pub style_attribution: Option<String>,
    /// Raster tile URL template instead of a style [env: LQM_TILE_URL]
    #[arg(long)]
    pub tile_url: Option<String>,
    /// Map tile attribution [env: LQM_TILE_ATTRIBUTION]
    #[arg(long)]
    pub tile_attribution: Option<String>,
    /// Colour for unknown bands, #RRGGBB [env: LQM_FALLBACK_COLOR] [default: #7f7f7f]
    #[arg(long)]
    pub fallback_color: Option<String>,
    /// BAND=#RRGGBB, repeatable [env: LQM_BAND_COLORS, comma-separated]
    #[arg(long = "band-color")]
    pub band_color: Vec<String>,
    /// Bands to consider: ADIF bands or groups (lf-mf, hf, hf-contest, warc, vhf, uhf, shf, ehf),
    /// comma-separated or repeated; prefix with - to exclude [env: LQM_BANDS] [default: all]
    #[arg(long = "bands")]
    pub bands: Vec<String>,
    /// Last-QSO box columns in order: call, band, mode, date, time, cqzone, country;
    /// comma-separated or repeated [env: LQM_RECENT_COLUMNS] [default: call,band,time,cqzone,country]
    #[arg(long = "recent-columns")]
    pub recent_columns: Vec<String>,
    /// Show the last-QSO box header row: true or false [env: LQM_RECENT_HEADER] [default: true]
    #[arg(long = "recent-header")]
    pub recent_header: Option<bool>,
    /// Write the effective configuration to PATH (- = standard output), then exit
    #[arg(long, value_name = "PATH", conflicts_with = "dump_config")]
    pub save_config: Option<PathBuf>,
    /// Write the effective configuration to PATH (- = standard output), then run
    #[arg(long, value_name = "PATH")]
    pub dump_config: Option<PathBuf>,
}

/// What to do with the effective configuration before running (feature 008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportAction {
    /// Write it, then exit.
    Save(PathBuf),
    /// Write it, then run.
    Dump(PathBuf),
}

impl Cli {
    /// The requested export, if any. Command line only: there is no `LQM_*` variable.
    pub fn export_action(&self) -> Option<ExportAction> {
        match (&self.save_config, &self.dump_config) {
            (Some(path), _) => Some(ExportAction::Save(path.clone())),
            (None, Some(path)) => Some(ExportAction::Dump(path.clone())),
            (None, None) => None,
        }
    }
}

/// A secret value whose `Debug` output is redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub password: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Qrz,
    HamQth,
    QrzCq,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Qrz => "qrz",
            ProviderKind::HamQth => "hamqth",
            ProviderKind::QrzCq => "qrzcq",
        }
    }
}

impl FromStr for ProviderKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "qrz" => Ok(ProviderKind::Qrz),
            "hamqth" => Ok(ProviderKind::HamQth),
            "qrzcq" => Ok(ProviderKind::QrzCq),
            other => Err(format!("unknown provider '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StationConfig {
    pub callsign: Option<String>,
    pub locator: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookupConfig {
    pub timeout: Duration,
    pub provider_order: Vec<ProviderKind>,
    pub qrz: Option<Credentials>,
    pub hamqth: Option<Credentials>,
    pub qrzcq: Option<Credentials>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CountryDataConfig {
    pub path: PathBuf,
    pub url: String,
    pub max_age_days: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapConfig {
    pub style_url: Option<String>,
    pub style_attribution: Option<String>,
    pub tile_url: Option<String>,
    pub tile_attribution: Option<String>,
    pub fallback_color: String,
    pub bands: BTreeMap<String, String>,
    /// Last-QSO box columns, in display order (feature 006).
    pub recent_columns: Vec<Column>,
    /// Whether the last-QSO box shows its header row (feature 006).
    pub recent_header: bool,
}

/// The background map sent to pages as `hello.basemap` (feature 002, data-model).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BaseMap {
    Style { url: String, attribution: String },
    Raster { url: String, attribution: String },
}

impl MapConfig {
    /// Raster tiles when configured, else the configured style, else the OpenFreeMap default
    /// with its credit. `Config::load` rejects configuring both.
    pub fn basemap(&self) -> BaseMap {
        if let Some(url) = &self.tile_url {
            return BaseMap::Raster {
                url: url.clone(),
                attribution: self.tile_attribution.clone().unwrap_or_default(),
            };
        }
        match &self.style_url {
            Some(url) => BaseMap::Style {
                url: url.clone(),
                attribution: self.style_attribution.clone().unwrap_or_default(),
            },
            None => BaseMap::Style {
                url: DEFAULT_STYLE_URL.to_string(),
                attribution: DEFAULT_STYLE_ATTRIBUTION.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub bind_address: IpAddr,
    pub udp_port: u16,
    pub http_port: u16,
    pub history_size: usize,
    pub workers: usize,
    pub ingest_queue: usize,
    pub broadcast_queue: usize,
    pub log_level: String,
    pub station: StationConfig,
    pub lookup: LookupConfig,
    pub country_data: CountryDataConfig,
    pub map: MapConfig,
    /// Considered bands (feature 003); `None` means every band, no filter.
    pub band_filter: Option<BandSet>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            udp_port: 12060,
            http_port: 8080,
            history_size: 10_000,
            workers: 1,
            ingest_queue: 1024,
            broadcast_queue: 1024,
            log_level: "info".into(),
            station: StationConfig::default(),
            lookup: LookupConfig {
                timeout: Duration::from_secs(5),
                provider_order: vec![ProviderKind::Qrz, ProviderKind::HamQth, ProviderKind::QrzCq],
                qrz: None,
                hamqth: None,
                qrzcq: None,
            },
            country_data: CountryDataConfig {
                path: PathBuf::from("./cty.dat"),
                url: "https://www.country-files.com/cty/cty.dat".into(),
                max_age_days: 30,
            },
            band_filter: None,
            map: MapConfig {
                style_url: None,
                style_attribution: None,
                tile_url: None,
                tile_attribution: None,
                fallback_color: "#7f7f7f".into(),
                bands: DEFAULT_BANDS
                    .iter()
                    .map(|(b, c)| (b.to_string(), c.to_string()))
                    .collect(),
                recent_columns: DEFAULT_COLUMNS.to_vec(),
                recent_header: true,
            },
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    bands: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bind_address: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    history_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ingest_queue: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    broadcast_queue: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,
    #[serde(default, skip_serializing_if = "FileStation::is_empty")]
    station: FileStation,
    #[serde(default)]
    lookup: FileLookup,
    #[serde(default)]
    country_data: FileCountryData,
    #[serde(default)]
    map: FileMap,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileStation {
    #[serde(skip_serializing_if = "Option::is_none")]
    callsign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    longitude: Option<f64>,
}

impl FileStation {
    fn is_empty(&self) -> bool {
        self.callsign.is_none()
            && self.locator.is_none()
            && self.latitude.is_none()
            && self.longitude.is_none()
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileLookup {
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "FileCredentials::is_empty")]
    qrz: FileCredentials,
    #[serde(default, skip_serializing_if = "FileCredentials::is_empty")]
    hamqth: FileCredentials,
    #[serde(default, skip_serializing_if = "FileCredentials::is_empty")]
    qrzcq: FileCredentials,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileCredentials {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

impl FileCredentials {
    fn is_empty(&self) -> bool {
        self.username.is_none() && self.password.is_none()
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileCountryData {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_age_days: Option<u64>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileMap {
    #[serde(skip_serializing_if = "Option::is_none")]
    style_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style_attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tile_attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recent_columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recent_header: Option<bool>,
    // Last: a sub-table. Written in frequency order (HR-2), not alphabetically.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_bands_by_frequency"
    )]
    bands: BTreeMap<String, String>,
}

fn serialize_bands_by_frequency<S: serde::Serializer>(
    bands: &BTreeMap<String, String>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut ordered: Vec<(&String, &String)> = bands.iter().collect();
    ordered.sort_by_key(|(band, _)| (band_rank(band).unwrap_or(usize::MAX), band.as_str()));
    let mut map = serializer.serialize_map(Some(ordered.len()))?;
    for (band, colour) in ordered {
        map.serialize_entry(band, colour)?;
    }
    map.end()
}

impl FileConfig {
    /// The effective configuration in file form: every setting, unset values left out
    /// (feature 008, data-model "Exported configuration").
    pub(crate) fn from_config(config: &Config) -> Self {
        let credentials = |c: &Option<Credentials>| match c {
            Some(c) => FileCredentials {
                username: Some(c.username.clone()),
                password: Some(c.password.expose().to_string()),
            },
            None => FileCredentials::default(),
        };
        FileConfig {
            bands: config
                .band_filter
                .as_ref()
                .map(|set| set.bands().iter().map(|b| b.to_string()).collect()),
            bind_address: Some(config.bind_address),
            udp_port: Some(u32::from(config.udp_port)),
            http_port: Some(u32::from(config.http_port)),
            history_size: Some(config.history_size),
            workers: Some(config.workers),
            ingest_queue: Some(config.ingest_queue),
            broadcast_queue: Some(config.broadcast_queue),
            log_level: Some(config.log_level.clone()),
            station: FileStation {
                callsign: config.station.callsign.clone(),
                locator: config.station.locator.clone(),
                latitude: config.station.latitude,
                longitude: config.station.longitude,
            },
            lookup: FileLookup {
                timeout_secs: Some(config.lookup.timeout.as_secs()),
                provider_order: Some(
                    config
                        .lookup
                        .provider_order
                        .iter()
                        .map(|p| p.as_str().to_string())
                        .collect(),
                ),
                qrz: credentials(&config.lookup.qrz),
                hamqth: credentials(&config.lookup.hamqth),
                qrzcq: credentials(&config.lookup.qrzcq),
            },
            country_data: FileCountryData {
                path: Some(config.country_data.path.clone()),
                url: Some(config.country_data.url.clone()),
                max_age_days: Some(config.country_data.max_age_days),
            },
            map: FileMap {
                style_url: config.map.style_url.clone(),
                style_attribution: config.map.style_attribution.clone(),
                tile_url: config.map.tile_url.clone(),
                tile_attribution: config.map.tile_attribution.clone(),
                fallback_color: Some(config.map.fallback_color.clone()),
                recent_columns: Some(
                    config
                        .map
                        .recent_columns
                        .iter()
                        .map(|c| c.keyword().to_string())
                        .collect(),
                ),
                recent_header: Some(config.map.recent_header),
                bands: config.map.bands.clone(),
            },
        }
    }
}

/// Looks up an environment variable; injected so tests never touch the process environment.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Name of the configuration file looked up next to the program (feature 008, R10).
pub const PROGRAM_FOLDER_CONFIG: &str = "live-qso-map.toml";

/// The configuration file path: `--config`, then `LQM_CONFIG`, then `live-qso-map.toml` in the
/// program's folder (`exe_dir`) when `exists` says it is there.
pub fn config_path(
    cli: &Cli,
    env: EnvLookup,
    exe_dir: Option<&Path>,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    cli.config
        .clone()
        .or_else(|| env("LQM_CONFIG").map(PathBuf::from))
        .or_else(|| {
            exe_dir
                .map(|dir| dir.join(PROGRAM_FOLDER_CONFIG))
                .filter(|path| exists(path))
        })
}

/// Whether relative data paths resolve against the program's folder: on Windows and macOS,
/// where a double-clicked program starts in an unrelated folder (feature 008, R10).
pub fn program_folder_base() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

/// `path` joined to the program's folder when it is relative and `program_folder_base` is set;
/// otherwise `path` unchanged.
pub fn resolve_data_path(
    path: &Path,
    exe_dir: Option<&Path>,
    program_folder_base: bool,
) -> PathBuf {
    match exe_dir {
        Some(dir) if program_folder_base && path.is_relative() => dir.join(path),
        _ => path.to_path_buf(),
    }
}

fn config_error(setting: &str, message: impl Into<String>) -> Error {
    Error::Config {
        setting: setting.to_string(),
        message: message.into(),
    }
}

/// Command line, then environment, then file.
fn layer<T: FromStr>(
    setting: &str,
    cli: Option<T>,
    env: EnvLookup,
    var: &str,
    file: Option<T>,
) -> Result<Option<T>> {
    if cli.is_some() {
        return Ok(cli);
    }
    if let Some(raw) = env(var) {
        return raw
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| config_error(setting, format!("cannot parse {var}={raw}")));
    }
    Ok(file)
}

fn port(setting: &str, value: Option<u32>, default: u16) -> Result<u16> {
    match value {
        None => Ok(default),
        Some(p) if (1..=65_535).contains(&p) => Ok(p as u16),
        Some(p) => Err(config_error(
            setting,
            format!("{p} is not between 1 and 65535"),
        )),
    }
}

fn positive(setting: &str, value: Option<usize>, default: usize) -> Result<usize> {
    match value {
        None => Ok(default),
        Some(0) => Err(config_error(setting, "must be at least 1")),
        Some(v) => Ok(v),
    }
}

fn is_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_order(raw: &[String]) -> Result<Vec<ProviderKind>> {
    raw.iter()
        .map(|s| s.parse::<ProviderKind>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|m| config_error("lookup.provider_order", m))
}

fn apply_band(bands: &mut BTreeMap<String, String>, band: &str, color: &str) -> Result<()> {
    let name = canonical_band(band)
        .ok_or_else(|| config_error("map.bands", format!("'{band}' is not an ADIF band")))?;
    let color = color.trim();
    if !is_color(color) {
        return Err(config_error(
            "map.bands",
            format!("'{color}' for {name} is not #RRGGBB"),
        ));
    }
    bands.insert(name.to_string(), color.to_string());
    Ok(())
}

fn apply_band_entries<'a>(
    bands: &mut BTreeMap<String, String>,
    entries: impl Iterator<Item = &'a str>,
) -> Result<()> {
    for entry in entries.map(str::trim).filter(|e| !e.is_empty()) {
        let (band, color) = entry
            .split_once('=')
            .ok_or_else(|| config_error("map.bands", format!("'{entry}' is not BAND=#RRGGBB")))?;
        apply_band(bands, band, color)?;
    }
    Ok(())
}

impl Config {
    /// Merges the layers and validates the result. Returns the configuration plus warnings
    /// to log (e.g. a provider disabled by incomplete credentials).
    pub fn load(
        cli: &Cli,
        env: EnvLookup,
        file_text: Option<&str>,
    ) -> Result<(Config, Vec<String>)> {
        let file: FileConfig = match file_text {
            Some(text) => {
                toml::from_str(text).map_err(|e| config_error("config", e.to_string()))?
            }
            None => FileConfig::default(),
        };
        let defaults = Config::default();
        let mut warnings = Vec::new();

        let bind_address = layer(
            "bind_address",
            cli.bind_address,
            env,
            "LQM_BIND_ADDRESS",
            file.bind_address,
        )?
        .unwrap_or(defaults.bind_address);
        let udp_port = port(
            "udp_port",
            layer("udp_port", cli.udp_port, env, "LQM_UDP_PORT", file.udp_port)?,
            defaults.udp_port,
        )?;
        let http_port = port(
            "http_port",
            layer(
                "http_port",
                cli.http_port,
                env,
                "LQM_HTTP_PORT",
                file.http_port,
            )?,
            defaults.http_port,
        )?;
        let history_size = layer(
            "history_size",
            cli.history_size,
            env,
            "LQM_HISTORY_SIZE",
            file.history_size,
        )?
        .unwrap_or(defaults.history_size);
        let workers = positive(
            "workers",
            layer("workers", cli.workers, env, "LQM_WORKERS", file.workers)?,
            defaults.workers,
        )?;
        let ingest_queue = positive(
            "ingest_queue",
            layer(
                "ingest_queue",
                cli.ingest_queue,
                env,
                "LQM_INGEST_QUEUE",
                file.ingest_queue,
            )?,
            defaults.ingest_queue,
        )?;
        let broadcast_queue = positive(
            "broadcast_queue",
            layer(
                "broadcast_queue",
                cli.broadcast_queue,
                env,
                "LQM_BROADCAST_QUEUE",
                file.broadcast_queue,
            )?,
            defaults.broadcast_queue,
        )?;
        let log_level = layer(
            "log_level",
            cli.log_level.clone(),
            env,
            "LQM_LOG_LEVEL",
            file.log_level,
        )?
        .unwrap_or(defaults.log_level)
        .to_ascii_lowercase();
        if !LOG_LEVELS.contains(&log_level.as_str()) {
            return Err(config_error(
                "log_level",
                format!("'{log_level}' is not one of {LOG_LEVELS:?}"),
            ));
        }

        let station = StationConfig {
            callsign: layer(
                "station.callsign",
                cli.station_callsign.clone(),
                env,
                "LQM_STATION_CALLSIGN",
                file.station.callsign,
            )?
            .map(|c| c.trim().to_ascii_uppercase())
            .filter(|c| !c.is_empty()),
            locator: layer(
                "station.locator",
                cli.station_locator.clone(),
                env,
                "LQM_STATION_LOCATOR",
                file.station.locator,
            )?
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty()),
            latitude: layer(
                "station.latitude",
                cli.station_latitude,
                env,
                "LQM_STATION_LATITUDE",
                file.station.latitude,
            )?,
            longitude: layer(
                "station.longitude",
                cli.station_longitude,
                env,
                "LQM_STATION_LONGITUDE",
                file.station.longitude,
            )?,
        };
        if let Some(locator) = &station.locator
            && !maidenhead::is_valid(locator)
        {
            return Err(config_error(
                "station.locator",
                format!("'{locator}' is not a 4, 6 or 8 character locator"),
            ));
        }
        match (station.latitude, station.longitude) {
            (Some(lat), Some(lon)) => {
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(config_error(
                        "station.latitude",
                        format!("{lat} is out of range"),
                    ));
                }
                if !(-180.0..=180.0).contains(&lon) {
                    return Err(config_error(
                        "station.longitude",
                        format!("{lon} is out of range"),
                    ));
                }
            }
            (Some(_), None) => {
                return Err(config_error(
                    "station.longitude",
                    "latitude is set but longitude is not",
                ));
            }
            (None, Some(_)) => {
                return Err(config_error(
                    "station.latitude",
                    "longitude is set but latitude is not",
                ));
            }
            (None, None) => {}
        }

        let timeout_secs = layer(
            "lookup.timeout_secs",
            cli.lookup_timeout,
            env,
            "LQM_LOOKUP_TIMEOUT",
            file.lookup.timeout_secs,
        )?
        .unwrap_or(defaults.lookup.timeout.as_secs());
        if timeout_secs == 0 {
            return Err(config_error("lookup.timeout_secs", "must be at least 1"));
        }
        let order_raw = match cli
            .provider_order
            .clone()
            .or_else(|| env("LQM_PROVIDER_ORDER"))
        {
            Some(text) => Some(text.split(',').map(str::to_string).collect::<Vec<_>>()),
            None => file.lookup.provider_order,
        };
        let provider_order = match order_raw {
            Some(raw) => parse_order(&raw)?,
            None => defaults.lookup.provider_order,
        };

        let mut credentials = |kind: ProviderKind,
                               cli_user: &Option<String>,
                               cli_pass: &Option<String>,
                               file: FileCredentials| {
            let name = kind.as_str();
            let upper = name.to_ascii_uppercase();
            let user = cli_user
                .clone()
                .or_else(|| env(&format!("LQM_{upper}_USERNAME")))
                .or(file.username)
                .filter(|u| !u.trim().is_empty());
            let pass = cli_pass
                .clone()
                .or_else(|| env(&format!("LQM_{upper}_PASSWORD")))
                .or(file.password)
                .filter(|p| !p.is_empty());
            match (user, pass) {
                (Some(username), Some(password)) => Some(Credentials {
                    username: username.trim().to_string(),
                    password: Secret::new(password),
                }),
                (None, None) => None,
                _ => {
                    warnings.push(format!(
                        "{name}: username and password must both be set; provider disabled"
                    ));
                    None
                }
            }
        };
        let qrz = credentials(
            ProviderKind::Qrz,
            &cli.qrz_username,
            &cli.qrz_password,
            file.lookup.qrz,
        );
        let hamqth = credentials(
            ProviderKind::HamQth,
            &cli.hamqth_username,
            &cli.hamqth_password,
            file.lookup.hamqth,
        );
        let qrzcq = credentials(
            ProviderKind::QrzCq,
            &cli.qrzcq_username,
            &cli.qrzcq_password,
            file.lookup.qrzcq,
        );

        let country_data = CountryDataConfig {
            path: layer(
                "country_data.path",
                cli.cty_path.clone(),
                env,
                "LQM_CTY_PATH",
                file.country_data.path,
            )?
            .unwrap_or(defaults.country_data.path),
            url: layer(
                "country_data.url",
                cli.cty_url.clone(),
                env,
                "LQM_CTY_URL",
                file.country_data.url,
            )?
            .unwrap_or(defaults.country_data.url),
            max_age_days: layer(
                "country_data.max_age_days",
                cli.cty_max_age_days,
                env,
                "LQM_CTY_MAX_AGE_DAYS",
                file.country_data.max_age_days,
            )?
            .unwrap_or(defaults.country_data.max_age_days),
        };

        let fallback_color = layer(
            "map.fallback_color",
            cli.fallback_color.clone(),
            env,
            "LQM_FALLBACK_COLOR",
            file.map.fallback_color,
        )?
        .unwrap_or(defaults.map.fallback_color);
        if !is_color(&fallback_color) {
            return Err(config_error(
                "map.fallback_color",
                format!("'{fallback_color}' is not #RRGGBB"),
            ));
        }
        let mut bands = defaults.map.bands;
        for (band, color) in &file.map.bands {
            apply_band(&mut bands, band, color)?;
        }
        if let Some(raw) = env("LQM_BAND_COLORS") {
            apply_band_entries(&mut bands, raw.split(','))?;
        }
        apply_band_entries(&mut bands, cli.band_color.iter().map(String::as_str))?;
        let map = MapConfig {
            style_url: layer(
                "map.style_url",
                cli.style_url.clone(),
                env,
                "LQM_STYLE_URL",
                file.map.style_url,
            )?
            .filter(|u| !u.trim().is_empty()),
            style_attribution: layer(
                "map.style_attribution",
                cli.style_attribution.clone(),
                env,
                "LQM_STYLE_ATTRIBUTION",
                file.map.style_attribution,
            )?,
            tile_url: layer(
                "map.tile_url",
                cli.tile_url.clone(),
                env,
                "LQM_TILE_URL",
                file.map.tile_url,
            )?
            .filter(|u| !u.trim().is_empty()),
            tile_attribution: layer(
                "map.tile_attribution",
                cli.tile_attribution.clone(),
                env,
                "LQM_TILE_ATTRIBUTION",
                file.map.tile_attribution,
            )?,
            fallback_color,
            bands,
            // A higher layer replaces the whole list, as for `bands` (feature 006, research R2).
            recent_columns: {
                let entries: Vec<String> = if !cli.recent_columns.is_empty() {
                    cli.recent_columns.clone()
                } else if let Some(raw) = env("LQM_RECENT_COLUMNS") {
                    vec![raw]
                } else if let Some(list) = file.map.recent_columns {
                    list
                } else {
                    DEFAULT_COLUMNS
                        .iter()
                        .map(|c| c.keyword().to_string())
                        .collect()
                };
                parse_columns(&entries)?
            },
            recent_header: layer(
                "map.recent_header",
                cli.recent_header,
                env,
                "LQM_RECENT_HEADER",
                file.map.recent_header,
            )?
            .unwrap_or(true),
        };
        if map.style_url.is_some() && map.tile_url.is_some() {
            return Err(config_error(
                "map.style_url",
                "style_url and tile_url are mutually exclusive",
            ));
        }
        if map.tile_url.is_some()
            && map
                .tile_attribution
                .as_deref()
                .is_none_or(|a| a.trim().is_empty())
        {
            return Err(config_error(
                "map.tile_attribution",
                "raster tiles need an attribution (FR-009)",
            ));
        }

        // A higher layer replaces the whole list (feature 003, research R4).
        let band_entries: Vec<String> = if !cli.bands.is_empty() {
            cli.bands.clone()
        } else if let Some(raw) = env("LQM_BANDS") {
            vec![raw]
        } else {
            file.bands.unwrap_or_default()
        };
        let band_filter = parse_setting(&band_entries)?;

        let config = Config {
            bind_address,
            udp_port,
            http_port,
            history_size,
            workers,
            ingest_queue,
            broadcast_queue,
            log_level,
            station,
            lookup: LookupConfig {
                timeout: Duration::from_secs(timeout_secs),
                provider_order,
                qrz,
                hamqth,
                qrzcq,
            },
            country_data,
            map,
            band_filter,
        };
        Ok((config, warnings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cli(args: &[&str]) -> Cli {
        let mut argv = vec!["live-qso-map"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv).unwrap()
    }

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn load(
        args: &[&str],
        env_pairs: &[(&str, &str)],
        file: Option<&str>,
    ) -> Result<(Config, Vec<String>)> {
        let env = env(env_pairs);
        Config::load(&cli(args), &|k| env.get(k).cloned(), file)
    }

    fn config_err(r: Result<(Config, Vec<String>)>, setting: &str) {
        match r {
            Err(Error::Config { setting: s, .. }) => assert_eq!(s, setting),
            other => panic!("expected config error for {setting}, got {other:?}"),
        }
    }

    #[test]
    fn defaults_match_the_contract() {
        let (c, warnings) = load(&[], &[], None).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(c.bind_address.to_string(), "0.0.0.0");
        assert_eq!(c.udp_port, 12060);
        assert_eq!(c.http_port, 8080);
        assert_eq!(c.history_size, 10000);
        assert_eq!(c.workers, 1);
        assert_eq!(c.ingest_queue, 1024);
        assert_eq!(c.broadcast_queue, 1024);
        assert_eq!(c.log_level, "info");
        assert_eq!(c.station, StationConfig::default());
        assert_eq!(c.lookup.timeout, Duration::from_secs(5));
        assert_eq!(
            c.lookup.provider_order,
            [ProviderKind::Qrz, ProviderKind::HamQth, ProviderKind::QrzCq]
        );
        assert!(c.lookup.qrz.is_none() && c.lookup.hamqth.is_none() && c.lookup.qrzcq.is_none());
        assert_eq!(c.country_data.path, PathBuf::from("./cty.dat"));
        assert_eq!(
            c.country_data.url,
            "https://www.country-files.com/cty/cty.dat"
        );
        assert_eq!(c.country_data.max_age_days, 30);
        assert_eq!(c.map.tile_url, None);
        assert_eq!(c.map.tile_attribution, None);
        assert_eq!(c.map.style_url, None);
        assert_eq!(c.map.style_attribution, None);
        assert_eq!(
            c.map.basemap(),
            BaseMap::Style {
                url: "https://tiles.openfreemap.org/styles/liberty".into(),
                attribution: DEFAULT_STYLE_ATTRIBUTION.into(),
            }
        );
        assert!(DEFAULT_STYLE_ATTRIBUTION.contains("OpenFreeMap"));
        assert!(DEFAULT_STYLE_ATTRIBUTION.contains("OpenMapTiles"));
        assert!(DEFAULT_STYLE_ATTRIBUTION.contains("OpenStreetMap"));
        assert_eq!(c.map.fallback_color, "#7f7f7f");
        assert_eq!(c.map.bands.get("40m").map(String::as_str), Some("#d62728"));
        assert_eq!(c.map.bands.get("20m").map(String::as_str), Some("#1f77b4"));
        assert_eq!(c.map.bands.len(), 13);
    }

    const FILE: &str = r##"
udp_port = 2000
http_port = 2001
history_size = 50

[station]
callsign = "IS0MZK"
locator = "JM49NH"

[lookup]
timeout_secs = 7
provider_order = ["hamqth", "qrz"]

[lookup.qrz]
username = "IS0MZK"
password = "secret-qrz"

[country_data]
path = "/var/lib/lqm/cty.dat"
max_age_days = 0

[map]
fallback_color = "#000000"

[map.bands]
"40m" = "#111111"
"##;

    #[test]
    fn file_values_apply_when_cli_and_env_are_unset() {
        let (c, _) = load(&[], &[], Some(FILE)).unwrap();
        assert_eq!(c.udp_port, 2000);
        assert_eq!(c.http_port, 2001);
        assert_eq!(c.history_size, 50);
        assert_eq!(c.station.callsign.as_deref(), Some("IS0MZK"));
        assert_eq!(c.station.locator.as_deref(), Some("JM49NH"));
        assert_eq!(c.lookup.timeout, Duration::from_secs(7));
        assert_eq!(
            c.lookup.provider_order,
            [ProviderKind::HamQth, ProviderKind::Qrz]
        );
        let qrz = c.lookup.qrz.as_ref().unwrap();
        assert_eq!(qrz.username, "IS0MZK");
        assert_eq!(qrz.password.expose(), "secret-qrz");
        assert_eq!(c.country_data.path, PathBuf::from("/var/lib/lqm/cty.dat"));
        assert_eq!(c.country_data.max_age_days, 0);
        assert_eq!(c.map.fallback_color, "#000000");
        assert_eq!(c.map.bands["40m"], "#111111");
        assert_eq!(c.map.bands["20m"], "#1f77b4");
    }

    #[test]
    fn env_overrides_file() {
        let (c, _) = load(
            &[],
            &[
                ("LQM_UDP_PORT", "3000"),
                ("LQM_STATION_CALLSIGN", "IS0ENV"),
                ("LQM_QRZ_PASSWORD", "from-env"),
            ],
            Some(FILE),
        )
        .unwrap();
        assert_eq!(c.udp_port, 3000);
        assert_eq!(c.http_port, 2001);
        assert_eq!(c.station.callsign.as_deref(), Some("IS0ENV"));
        assert_eq!(c.lookup.qrz.as_ref().unwrap().password.expose(), "from-env");
    }

    #[test]
    fn cli_overrides_env() {
        let (c, _) = load(
            &["--udp-port", "4000", "--provider-order", "qrzcq,qrz"],
            &[("LQM_UDP_PORT", "3000"), ("LQM_PROVIDER_ORDER", "hamqth")],
            Some(FILE),
        )
        .unwrap();
        assert_eq!(c.udp_port, 4000);
        assert_eq!(
            c.lookup.provider_order,
            [ProviderKind::QrzCq, ProviderKind::Qrz]
        );
    }

    #[test]
    fn band_colors_override_only_their_band() {
        let (c, _) = load(
            &["--band-color", "20m=#112233"],
            &[("LQM_BAND_COLORS", "40m=#445566, 15m=#778899")],
            Some(FILE),
        )
        .unwrap();
        assert_eq!(c.map.bands["20m"], "#112233");
        assert_eq!(c.map.bands["40m"], "#445566");
        assert_eq!(c.map.bands["15m"], "#778899");
        assert_eq!(c.map.bands["10m"], "#393b79");

        let (c, _) = load(
            &["--band-color", "40m=#aaaaaa"],
            &[("LQM_BAND_COLORS", "40m=#445566")],
            None,
        )
        .unwrap();
        assert_eq!(c.map.bands["40m"], "#aaaaaa");
    }

    #[test]
    fn validation_errors() {
        config_err(load(&["--udp-port", "0"], &[], None), "udp_port");
        config_err(load(&[], &[("LQM_HTTP_PORT", "0")], None), "http_port");
        config_err(load(&[], &[("LQM_HTTP_PORT", "70000")], None), "http_port");
        config_err(load(&["--workers", "0"], &[], None), "workers");
        config_err(load(&["--ingest-queue", "0"], &[], None), "ingest_queue");
        config_err(
            load(&["--broadcast-queue", "0"], &[], None),
            "broadcast_queue",
        );
        config_err(
            load(&["--station-locator", "ZZ99"], &[], None),
            "station.locator",
        );
        config_err(
            load(&["--station-latitude", "39.2"], &[], None),
            "station.longitude",
        );
        config_err(
            load(
                &["--station-latitude", "95", "--station-longitude", "9"],
                &[],
                None,
            ),
            "station.latitude",
        );
        config_err(
            load(&["--fallback-color", "red"], &[], None),
            "map.fallback_color",
        );
        config_err(
            load(&["--band-color", "40m=#12345"], &[], None),
            "map.bands",
        );
        config_err(
            load(&["--band-color", "21m=#123456"], &[], None),
            "map.bands",
        );
        config_err(
            load(&["--provider-order", "qrz,foo"], &[], None),
            "lookup.provider_order",
        );
        config_err(load(&["--log-level", "loud"], &[], None), "log_level");
        config_err(
            load(&[], &[("LQM_HISTORY_SIZE", "many")], None),
            "history_size",
        );
        config_err(
            load(&["--lookup-timeout", "0"], &[], None),
            "lookup.timeout_secs",
        );
    }

    #[test]
    fn file_errors() {
        config_err(load(&[], &[], Some("udp_port = ")), "config");
        config_err(load(&[], &[], Some("unknown_key = 1")), "config");
    }

    #[test]
    fn half_credentials_disable_provider_with_warning() {
        let (c, warnings) = load(&["--hamqth-username", "IS0MZK"], &[], None).unwrap();
        assert!(c.lookup.hamqth.is_none());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("hamqth"));
    }

    #[test]
    fn full_credentials_enable_provider() {
        let (c, warnings) = load(
            &["--qrzcq-username", "IS0MZK"],
            &[("LQM_QRZCQ_PASSWORD", "pw")],
            None,
        )
        .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(c.lookup.qrzcq.as_ref().unwrap().username, "IS0MZK");
    }

    #[test]
    fn debug_output_never_contains_passwords() {
        let (c, _) = load(&[], &[("LQM_QRZ_PASSWORD", "hunter2-secret")], Some(FILE)).unwrap();
        let text = format!("{c:?}");
        assert!(!text.contains("hunter2-secret"));
        assert!(!text.contains("secret-qrz"));
        assert!(text.contains("***"));
    }

    #[test]
    fn config_file_path_comes_from_cli_then_env_then_program_folder() {
        let exe_dir = Path::new("/opt/lqm");
        let exists = |p: &Path| p == Path::new("/opt/lqm/live-qso-map.toml");
        let missing = |_: &Path| false;
        let env = env(&[("LQM_CONFIG", "/b.toml")]);
        let with_env = |k: &str| env.get(k).cloned();

        assert_eq!(
            config_path(
                &cli(&["--config", "/a.toml"]),
                &with_env,
                Some(exe_dir),
                &exists
            ),
            Some(PathBuf::from("/a.toml"))
        );
        assert_eq!(
            config_path(&cli(&[]), &with_env, Some(exe_dir), &exists),
            Some(PathBuf::from("/b.toml"))
        );
        assert_eq!(
            config_path(&cli(&[]), &|_| None, Some(exe_dir), &exists),
            Some(PathBuf::from("/opt/lqm/live-qso-map.toml"))
        );
        assert_eq!(
            config_path(&cli(&[]), &|_| None, Some(exe_dir), &missing),
            None
        );
        assert_eq!(config_path(&cli(&[]), &|_| None, None, &exists), None);
    }

    // --- feature 008: data paths next to the program (T029) ---

    #[test]
    fn relative_data_path_joins_the_program_folder_when_enabled() {
        let exe_dir = Path::new("/opt/lqm");
        assert_eq!(
            resolve_data_path(Path::new("./cty.dat"), Some(exe_dir), true),
            PathBuf::from("/opt/lqm/./cty.dat")
        );
        assert_eq!(
            resolve_data_path(Path::new("data/cty.dat"), Some(exe_dir), true),
            PathBuf::from("/opt/lqm/data/cty.dat")
        );
        assert_eq!(
            resolve_data_path(Path::new("cty.dat"), None, true),
            PathBuf::from("cty.dat")
        );
    }

    #[test]
    fn absolute_or_disabled_paths_are_unchanged() {
        let exe_dir = Path::new("/opt/lqm");
        let absolute = std::env::temp_dir().join("cty.dat");
        assert_eq!(resolve_data_path(&absolute, Some(exe_dir), true), absolute);
        assert_eq!(
            resolve_data_path(Path::new("./cty.dat"), Some(exe_dir), false),
            PathBuf::from("./cty.dat")
        );
    }

    #[test]
    fn program_folder_base_only_on_windows_and_macos() {
        assert_eq!(
            program_folder_base(),
            cfg!(windows) || cfg!(target_os = "macos")
        );
    }

    // --- base map (feature 002, T017) ---------------------------------------------------------

    #[test]
    fn style_settings_follow_precedence() {
        let file = r#"
[map]
style_url = "https://file/style.json"
style_attribution = "file credit"
"#;
        let (c, _) = load(&[], &[], Some(file)).unwrap();
        assert_eq!(
            c.map.basemap(),
            BaseMap::Style {
                url: "https://file/style.json".into(),
                attribution: "file credit".into()
            }
        );
        let (c, _) = load(
            &[],
            &[("LQM_STYLE_URL", "https://env/style.json")],
            Some(file),
        )
        .unwrap();
        assert_eq!(c.map.style_url.as_deref(), Some("https://env/style.json"));
        let (c, _) = load(
            &[
                "--style-url",
                "https://cli/style.json",
                "--style-attribution",
                "cli credit",
            ],
            &[
                ("LQM_STYLE_URL", "https://env/style.json"),
                ("LQM_STYLE_ATTRIBUTION", "env credit"),
            ],
            Some(file),
        )
        .unwrap();
        assert_eq!(
            c.map.basemap(),
            BaseMap::Style {
                url: "https://cli/style.json".into(),
                attribution: "cli credit".into()
            }
        );
    }

    #[test]
    fn custom_style_without_attribution_has_empty_credit() {
        let (c, _) = load(&["--style-url", "https://s/style.json"], &[], None).unwrap();
        assert_eq!(
            c.map.basemap(),
            BaseMap::Style {
                url: "https://s/style.json".into(),
                attribution: String::new()
            }
        );
    }

    #[test]
    fn tile_url_gives_raster() {
        let (c, _) = load(
            &["--tile-url", "https://t/{z}/{x}/{y}.png"],
            &[("LQM_TILE_ATTRIBUTION", "&copy; tiles")],
            None,
        )
        .unwrap();
        assert_eq!(
            c.map.basemap(),
            BaseMap::Raster {
                url: "https://t/{z}/{x}/{y}.png".into(),
                attribution: "&copy; tiles".into()
            }
        );
    }

    #[test]
    fn style_and_tiles_are_mutually_exclusive() {
        let r = load(
            &["--style-url", "https://s/style.json"],
            &[
                ("LQM_TILE_URL", "https://t/{z}/{x}/{y}.png"),
                ("LQM_TILE_ATTRIBUTION", "x"),
            ],
            None,
        );
        config_err(r, "map.style_url");
        let file = "[map]\nstyle_url = \"https://s\"\ntile_url = \"https://t\"\ntile_attribution = \"x\"\n";
        config_err(load(&[], &[], Some(file)), "map.style_url");
    }

    #[test]
    fn raster_tiles_need_an_attribution() {
        config_err(
            load(&["--tile-url", "https://t/{z}/{x}/{y}.png"], &[], None),
            "map.tile_attribution",
        );
        config_err(
            load(
                &[
                    "--tile-url",
                    "https://t/{z}/{x}/{y}.png",
                    "--tile-attribution",
                    "  ",
                ],
                &[],
                None,
            ),
            "map.tile_attribution",
        );
    }

    // --- band selection (feature 003, T003) ---------------------------------------------------

    fn band_names(c: &Config) -> Vec<&str> {
        c.band_filter.as_ref().unwrap().bands().to_vec()
    }

    #[test]
    fn band_filter_defaults_to_none() {
        let (c, _) = load(&[], &[], None).unwrap();
        assert_eq!(c.band_filter, None);
        let (c, _) = load(&[], &[("LQM_BANDS", "")], None).unwrap();
        assert_eq!(c.band_filter, None);
    }

    #[test]
    fn band_filter_layers_replace_the_whole_list() {
        let file = "bands = [\"hf-contest\"]\n";
        let (c, _) = load(&[], &[], Some(file)).unwrap();
        assert_eq!(band_names(&c), ["160m", "80m", "40m", "20m", "15m", "10m"]);

        let (c, _) = load(&[], &[("LQM_BANDS", "hf,-60m")], Some(file)).unwrap();
        assert_eq!(
            band_names(&c),
            [
                "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m"
            ]
        );

        let (c, _) = load(
            &["--bands", "warc", "--bands", "2m"],
            &[("LQM_BANDS", "hf")],
            Some(file),
        )
        .unwrap();
        assert_eq!(band_names(&c), ["30m", "17m", "12m", "2m"]);
        let (c, _) = load(&["--bands", "warc,2m"], &[("LQM_BANDS", "hf")], Some(file)).unwrap();
        assert_eq!(band_names(&c), ["30m", "17m", "12m", "2m"]);
    }

    #[test]
    fn band_filter_errors() {
        config_err(load(&["--bands", "hf-dx"], &[], None), "bands");
        config_err(
            load(&[], &[], Some("bands = [\"warc\", \"-warc\"]\n")),
            "bands",
        );
    }

    fn columns(c: &Config) -> Vec<&str> {
        c.map
            .recent_columns
            .iter()
            .map(|col| col.keyword())
            .collect()
    }

    #[test]
    fn recent_box_defaults() {
        let (c, _) = load(&[], &[], None).unwrap();
        assert_eq!(c.map.recent_columns, crate::recent_columns::DEFAULT_COLUMNS);
        assert!(c.map.recent_header);
    }

    #[test]
    fn recent_box_layers_replace_the_whole_list() {
        let file = "[map]\nrecent_columns = [\"country\", \"call\"]\nrecent_header = false\n";
        let (c, _) = load(&[], &[], Some(file)).unwrap();
        assert_eq!(columns(&c), ["country", "call"]);
        assert!(!c.map.recent_header);

        let (c, _) = load(&[], &[("LQM_RECENT_COLUMNS", "call, mode")], Some(file)).unwrap();
        assert_eq!(columns(&c), ["call", "mode"]);

        let (c, _) = load(
            &[
                "--recent-columns",
                "date",
                "--recent-columns",
                "time",
                "--recent-header",
                "true",
            ],
            &[
                ("LQM_RECENT_COLUMNS", "call"),
                ("LQM_RECENT_HEADER", "false"),
            ],
            Some(file),
        )
        .unwrap();
        assert_eq!(columns(&c), ["date", "time"]);
        assert!(c.map.recent_header);

        let (c, _) = load(&["--recent-columns", "band,call"], &[], None).unwrap();
        assert_eq!(columns(&c), ["band", "call"]);
        let (c, _) = load(&[], &[("LQM_RECENT_HEADER", "false")], None).unwrap();
        assert!(!c.map.recent_header);
    }

    #[test]
    fn recent_box_errors() {
        config_err(
            load(&["--recent-columns", "call,freq"], &[], None),
            "map.recent_columns",
        );
        config_err(
            load(&[], &[("LQM_RECENT_COLUMNS", "call,call")], None),
            "map.recent_columns",
        );
        config_err(
            load(&[], &[], Some("[map]\nrecent_columns = []\n")),
            "map.recent_columns",
        );
        config_err(
            load(&[], &[("LQM_RECENT_HEADER", "maybe")], None),
            "map.recent_header",
        );
    }

    // --- feature 008: export options (T010) ---

    #[test]
    fn export_options_parse_and_map_to_actions() {
        assert_eq!(cli(&[]).export_action(), None);
        assert_eq!(
            cli(&["--save-config", "a.toml"]).export_action(),
            Some(ExportAction::Save(PathBuf::from("a.toml")))
        );
        assert_eq!(
            cli(&["--dump-config", "-"]).export_action(),
            Some(ExportAction::Dump(PathBuf::from("-")))
        );
    }

    #[test]
    fn save_and_dump_conflict() {
        let err = Cli::try_parse_from(["live-qso-map", "--save-config", "a", "--dump-config", "b"])
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn export_options_have_no_environment_variable() {
        let (config, _) = load(
            &[],
            &[("LQM_SAVE_CONFIG", "/x.toml"), ("LQM_DUMP_CONFIG", "-")],
            None,
        )
        .unwrap();
        assert_eq!(config, Config::default());
    }
}
