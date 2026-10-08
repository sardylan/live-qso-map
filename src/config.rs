//! Configuration: command line, then `LQM_*` environment, then TOML file, then defaults
//! (FR-032, FR-033, contracts/configuration.md).

use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::band::canonical_band;
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
    /// Map tile URL template [env: LQM_TILE_URL]
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
}

/// The background map sent to pages as `hello.basemap` (feature 002, data-model).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BaseMap {
    Style { url: String, attribution: String },
    Raster { url: String, attribution: String },
}

impl MapConfig {
    /// The configured style, or the OpenFreeMap default with its credit.
    pub fn basemap(&self) -> BaseMap {
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
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    bind_address: Option<IpAddr>,
    udp_port: Option<u32>,
    http_port: Option<u32>,
    history_size: Option<usize>,
    workers: Option<usize>,
    ingest_queue: Option<usize>,
    broadcast_queue: Option<usize>,
    log_level: Option<String>,
    #[serde(default)]
    station: FileStation,
    #[serde(default)]
    lookup: FileLookup,
    #[serde(default)]
    country_data: FileCountryData,
    #[serde(default)]
    map: FileMap,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileStation {
    callsign: Option<String>,
    locator: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLookup {
    timeout_secs: Option<u64>,
    provider_order: Option<Vec<String>>,
    #[serde(default)]
    qrz: FileCredentials,
    #[serde(default)]
    hamqth: FileCredentials,
    #[serde(default)]
    qrzcq: FileCredentials,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileCredentials {
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileCountryData {
    path: Option<PathBuf>,
    url: Option<String>,
    max_age_days: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileMap {
    tile_url: Option<String>,
    tile_attribution: Option<String>,
    fallback_color: Option<String>,
    #[serde(default)]
    bands: BTreeMap<String, String>,
}

/// Looks up an environment variable; injected so tests never touch the process environment.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The configuration file path: `--config`, then `LQM_CONFIG`.
pub fn config_path(cli: &Cli, env: EnvLookup) -> Option<PathBuf> {
    cli.config
        .clone()
        .or_else(|| env("LQM_CONFIG").map(PathBuf::from))
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
            style_url: None,
            style_attribution: None,
            tile_url: layer(
                "map.tile_url",
                cli.tile_url.clone(),
                env,
                "LQM_TILE_URL",
                file.map.tile_url,
            )?,
            tile_attribution: layer(
                "map.tile_attribution",
                cli.tile_attribution.clone(),
                env,
                "LQM_TILE_ATTRIBUTION",
                file.map.tile_attribution,
            )?,
            fallback_color,
            bands,
        };

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
    fn config_file_path_comes_from_cli_or_env() {
        assert_eq!(
            config_path(&cli(&["--config", "/a.toml"]), &|_| None),
            Some(PathBuf::from("/a.toml"))
        );
        let env = env(&[("LQM_CONFIG", "/b.toml")]);
        assert_eq!(
            config_path(&cli(&[]), &|k| env.get(k).cloned()),
            Some(PathBuf::from("/b.toml"))
        );
        assert_eq!(config_path(&cli(&[]), &|_| None), None);
    }
}
