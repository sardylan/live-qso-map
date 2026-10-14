//! Online callsign lookup providers (research R12). Login sessions are cached; lookup results
//! never are (FR-016).

pub mod hamqth;
pub mod qrz;
pub mod qrzcq;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::adif::AdifRecord;
use crate::adif::write::{format_lat, format_lon};
use crate::config::{Credentials, ProviderKind};
use crate::errors::{Error, Result};
use crate::maidenhead;
use crate::position::{LookupCandidate, Position, PositionSource, Precision};

/// What one lookup response means. Not an error type: real failures are `Err(Error::Provider)`.
#[derive(Debug, Clone, PartialEq)]
pub enum LookupOutcome {
    Found(LookupCandidate),
    NotFound,
    NeedsLogin,
}

pub(crate) const AGENT: &str = concat!("live-qso-map-", env!("CARGO_PKG_VERSION"));

/// Percent-encodes a query value (RFC 3986 unreserved characters pass through).
pub(crate) fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Builds a candidate from a provider's coordinates and locator.
/// `coordinates_precision` is what lat/lon is worth (QRZ maps it from `geoloc`).
pub(crate) fn candidate(
    source: PositionSource,
    lat_lon: Option<(f64, f64)>,
    coordinates_precision: Option<Precision>,
    grid: Option<&str>,
    mut fields: AdifRecord,
) -> LookupCandidate {
    let grid = grid.filter(|g| maidenhead::is_valid(g));
    let position = match (lat_lon, coordinates_precision) {
        (Some((lat, lon)), Some(Precision::Coordinates)) => {
            fields.set("LAT", format_lat(lat));
            fields.set("LON", format_lon(lon));
            Position::new(lat, lon, Precision::Coordinates, source).ok()
        }
        (Some((lat, lon)), Some(Precision::Country)) => {
            Position::new(lat, lon, Precision::Country, source).ok()
        }
        _ => grid.and_then(|g| {
            let (lat, lon) = maidenhead::centre(g)?;
            Position::new(lat, lon, maidenhead::precision_of(g)?, source).ok()
        }),
    };
    if let Some(g) = grid
        && coordinates_precision != Some(Precision::Country)
    {
        fields.set("GRIDSQUARE", g);
    }
    LookupCandidate {
        source,
        position,
        fields,
    }
}

pub(crate) fn parse_f64(value: Option<&str>) -> Option<f64> {
    value?.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Copies `(adif field, provider value)` pairs that are present and not blank.
pub(crate) fn fields_from(pairs: &[(&str, Option<&str>)]) -> AdifRecord {
    let mut record = AdifRecord::new();
    for (name, value) in pairs {
        if let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) {
            record.set(name, v);
        }
    }
    record
}

pub(crate) fn provider_error(provider: &'static str, message: impl Into<String>) -> Error {
    Error::Provider {
        provider,
        message: message.into(),
    }
}

/// A logged-in client for one provider.
pub struct ProviderClient {
    kind: ProviderKind,
    http: reqwest::Client,
    credentials: Credentials,
    session: Mutex<Option<String>>,
}

impl std::fmt::Debug for ProviderClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderClient")
            .field("kind", &self.kind)
            .field("username", &self.credentials.username)
            .field("password", &self.credentials.password)
            .field("session", &"***")
            .finish()
    }
}

impl ProviderClient {
    pub fn new(kind: ProviderKind, credentials: Credentials, http: reqwest::Client) -> Self {
        Self {
            kind,
            http,
            credentials,
            session: Mutex::new(None),
        }
    }

    pub fn source(&self) -> PositionSource {
        match self.kind {
            ProviderKind::Qrz => PositionSource::Qrz,
            ProviderKind::HamQth => PositionSource::HamQth,
            ProviderKind::QrzCq => PositionSource::QrzCq,
        }
    }

    /// The provider's name, as in `APP_LIVEQSOMAP_POS_SOURCE` (`qrz`, `hamqth`, `qrzcq`).
    pub fn name(&self) -> &'static str {
        self.kind.as_str()
    }

    fn login_url(&self) -> String {
        let (user, pass) = (
            &self.credentials.username,
            self.credentials.password.expose(),
        );
        match self.kind {
            ProviderKind::Qrz => qrz::login_url(user, pass),
            ProviderKind::HamQth => hamqth::login_url(user, pass),
            ProviderKind::QrzCq => qrzcq::login_url(user, pass),
        }
    }

    fn lookup_url(&self, key: &str, call: &str) -> String {
        match self.kind {
            ProviderKind::Qrz => qrz::lookup_url(key, call),
            ProviderKind::HamQth => hamqth::lookup_url(key, call),
            ProviderKind::QrzCq => qrzcq::lookup_url(key, call),
        }
    }

    fn parse_login(&self, xml: &str) -> Result<String> {
        match self.kind {
            ProviderKind::Qrz => qrz::parse_login(xml),
            ProviderKind::HamQth => hamqth::parse_login(xml),
            ProviderKind::QrzCq => qrzcq::parse_login(xml),
        }
    }

    fn parse_lookup(&self, xml: &str) -> Result<LookupOutcome> {
        match self.kind {
            ProviderKind::Qrz => qrz::parse_lookup(xml),
            ProviderKind::HamQth => hamqth::parse_lookup(xml),
            ProviderKind::QrzCq => qrzcq::parse_lookup(xml),
        }
    }

    /// GETs `url`. Errors never carry the URL, which may hold credentials or a session key.
    async fn get(&self, url: &str) -> Result<String> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Http(e.without_url()))?;
        response
            .text()
            .await
            .map_err(|e| Error::Http(e.without_url()))
    }

    /// The cached session key, logging in first when there is none. Holding the mutex during
    /// login makes concurrent lookups share one login (single-flight).
    async fn session_key(&self, force_login: bool) -> Result<String> {
        let mut session = self.session.lock().await;
        if !force_login && let Some(key) = session.as_ref() {
            return Ok(key.clone());
        }
        *session = None;
        let key = self.parse_login(&self.get(&self.login_url()).await?)?;
        tracing::debug!(provider = self.name(), "logged in");
        *session = Some(key.clone());
        Ok(key)
    }

    pub async fn lookup(&self, call: &str) -> Result<Option<LookupCandidate>> {
        let mut force_login = false;
        for _ in 0..2 {
            let key = self.session_key(force_login).await?;
            match self.parse_lookup(&self.get(&self.lookup_url(&key, call)).await?)? {
                LookupOutcome::Found(candidate) => return Ok(Some(candidate)),
                LookupOutcome::NotFound => {
                    tracing::debug!(provider = self.name(), call, "callsign not found");
                    return Ok(None);
                }
                LookupOutcome::NeedsLogin => force_login = true,
            }
        }
        Err(provider_error(
            self.name(),
            "session rejected after a fresh login",
        ))
    }
}

/// The shared HTTP client for providers and the cty.dat download.
pub fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    crate::install_crypto_provider();
    Ok(reqwest::Client::builder()
        .user_agent(AGENT)
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()?)
}

/// Builds a client for every provider that has credentials.
pub fn clients(
    lookup: &crate::config::LookupConfig,
    http: &reqwest::Client,
) -> Vec<Arc<ProviderClient>> {
    [
        (ProviderKind::Qrz, &lookup.qrz),
        (ProviderKind::HamQth, &lookup.hamqth),
        (ProviderKind::QrzCq, &lookup.qrzcq),
    ]
    .into_iter()
    .filter_map(|(kind, credentials)| {
        credentials
            .clone()
            .map(|c| Arc::new(ProviderClient::new(kind, c, http.clone())))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Secret;

    #[test]
    fn encode_escapes_reserved_characters() {
        assert_eq!(encode("IS0/DL1ABC"), "IS0%2FDL1ABC");
        assert_eq!(encode("a b&c=d;é"), "a%20b%26c%3Dd%3B%C3%A9");
        assert_eq!(encode("Az09-._~"), "Az09-._~");
    }

    #[test]
    fn debug_never_shows_secrets() {
        crate::install_crypto_provider();
        let client = ProviderClient::new(
            ProviderKind::Qrz,
            Credentials {
                username: "IS0MZK".into(),
                password: Secret::new("hunter2-secret"),
            },
            reqwest::Client::new(),
        );
        let text = format!("{client:?}");
        assert!(text.contains("IS0MZK"));
        assert!(!text.contains("hunter2-secret"));
    }

    #[tokio::test]
    async fn http_errors_never_contain_the_url() {
        let client = ProviderClient::new(
            ProviderKind::Qrz,
            Credentials {
                username: "IS0MZK".into(),
                password: Secret::new("hunter2-secret"),
            },
            http_client(Duration::from_secs(1)).unwrap(),
        );
        // Port 9 (discard) is closed: the request fails at connect time.
        let err = client
            .get("http://127.0.0.1:9/xml?username=IS0MZK&password=hunter2-secret&s=SESSIONKEY")
            .await
            .unwrap_err();
        for text in [err.to_string(), format!("{err:?}")] {
            assert!(!text.contains("hunter2-secret"), "{text}");
            assert!(!text.contains("SESSIONKEY"), "{text}");
        }
    }
}
