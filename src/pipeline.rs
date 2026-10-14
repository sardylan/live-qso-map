//! Service wiring: history, broadcast (channel 2), listeners and task lifecycle
//! (plan "Summary", research R2/R4).

use crate::adif::AdifRecord;
use crate::enrich::cty::CountryData;
use crate::entity_extents::ExtentTable;
use crate::web::protocol::{EntityView, HelloStation};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::errors::Result;
use crate::history::{EnrichedQso, History, Removal, Upsert, format_rfc3339};
use crate::ingest::score::ContestScore;
use crate::position::LookupCandidate;

/// One lookup source: a callsign in, a candidate (or nothing) out. Production builds these from
/// the provider enum and the country data; tests inject fakes through [`start`].
pub type LookupSource = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<Option<LookupCandidate>>> + Send>>
        + Send
        + Sync,
>;

/// Test override for the lookup sources; `None` builds them from the configuration.
#[doc(hidden)]
pub type LookupSources = Option<Vec<LookupSource>>;

/// A change pages must hear about, fanned out to every page session (channel 2).
#[derive(Debug, Clone)]
pub enum HistoryChange {
    Upsert(Arc<EnrichedQso>),
    /// The QSO no longer exists (feature 003: corrected onto an ignored band).
    Remove {
        id: String,
    },
    /// A new contest score (feature 007).
    Score(Arc<ScoreState>),
}

/// The latest contest score and when the service received it (feature 007, research R3).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreState {
    pub score: ContestScore,
    /// Monotonic receive time, for the message's `age_s`.
    pub received: Instant,
    /// UTC receive time, `YYYY-MM-DDTHH:MM:SSZ` (HR-1).
    pub received_at: String,
}

impl ScoreState {
    pub fn now(score: ContestScore) -> Self {
        let seconds = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(elapsed) => i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX),
            Err(_) => {
                tracing::warn!("the system clock is before 1970; the score time shows as 1970");
                0
            }
        };
        Self {
            score,
            received: Instant::now(),
            received_at: format_rfc3339(seconds),
        }
    }
}

/// Shared history and latest score, plus the broadcast sender.
#[derive(Clone)]
pub struct Pipeline {
    history: Arc<Mutex<History>>,
    score: Arc<Mutex<Option<Arc<ScoreState>>>>,
    changes: broadcast::Sender<HistoryChange>,
    /// QSOs handed to the workers and not yet published: id → revision and record to save if
    /// the service stops first (feature 014, R9).
    pending: Arc<Mutex<HashMap<String, (u32, AdifRecord)>>>,
}

impl Pipeline {
    pub fn new(broadcast_queue: usize) -> Self {
        let (changes, _) = broadcast::channel(broadcast_queue.max(1));
        Self {
            history: Arc::new(Mutex::new(History::new())),
            score: Arc::new(Mutex::new(None)),
            changes,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lock_pending(&self) -> MutexGuard<'_, HashMap<String, (u32, AdifRecord)>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Remembers a queued QSO until a worker is done with it. An older revision never replaces
    /// a newer one.
    pub fn pending_insert(&self, id: &str, rev: u32, record: AdifRecord) {
        let mut pending = self.lock_pending();
        if pending.get(id).is_none_or(|(held, _)| *held <= rev) {
            pending.insert(id.to_string(), (rev, record));
        }
    }

    /// Forgets a QSO once a worker published it or dropped it as stale, unless a newer revision
    /// is waiting.
    pub fn pending_remove(&self, id: &str, rev: u32) {
        let mut pending = self.lock_pending();
        if pending.get(id).is_some_and(|(held, _)| *held == rev) {
            pending.remove(id);
        }
    }

    /// Every record to write to the history file, in QSO time order: the history, a pending
    /// version when it is newer or the QSO is not in the history yet, and the parked records
    /// whose id is in neither (feature 014, R8, R9).
    pub fn records_to_save(&self, parked: &[(String, AdifRecord)]) -> Vec<AdifRecord> {
        let history = self.lock_history();
        let pending = self.lock_pending();
        let mut out: Vec<AdifRecord> = history
            .iter_in_order()
            .map(|q| match pending.get(&q.id) {
                Some((rev, record)) if *rev > q.rev => record.clone(),
                _ => q.record.clone(),
            })
            .collect();
        out.extend(
            pending
                .iter()
                .filter(|(id, _)| history.get(id).is_none())
                .map(|(_, (_, record))| record.clone()),
        );
        out.extend(
            parked
                .iter()
                .filter(|(id, _)| history.get(id).is_none() && !pending.contains_key(id))
                .map(|(_, record)| record.clone()),
        );
        out.sort_by_key(|r| {
            r.get("QSO_DATE")
                .zip(r.get("TIME_ON"))
                .and_then(|(d, t)| crate::history::qso_timestamp(d, t))
                .unwrap_or(i64::MAX)
        });
        out
    }

    fn lock_score(&self) -> MutexGuard<'_, Option<Arc<ScoreState>>> {
        self.score
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Keeps `score` as the latest and broadcasts it while holding the lock, so pages see scores
    /// in the order they arrived. Identical repeats are kept too: each refreshes the time (R2).
    pub fn publish_score(&self, score: ContestScore) {
        let mut held = self.lock_score();
        let state = Arc::new(ScoreState::now(score));
        *held = Some(Arc::clone(&state));
        // An error only means no page is connected.
        let _ = self.changes.send(HistoryChange::Score(state));
    }

    pub fn latest_score(&self) -> Option<Arc<ScoreState>> {
        self.lock_score().clone()
    }

    /// Locks the history. A poisoned lock is recovered: the history stays usable.
    pub fn lock_history(&self) -> MutexGuard<'_, History> {
        self.history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Upserts into the history and, only when accepted, broadcasts the change while still
    /// holding the lock, so broadcast order equals history order (R4). Returns whether the
    /// change was accepted.
    pub fn publish(&self, qso: EnrichedQso) -> bool {
        let mut history = self.lock_history();
        match history.upsert(qso) {
            Upsert::Accepted(stored) => {
                // An error only means no page is connected.
                let _ = self.changes.send(HistoryChange::Upsert(stored));
                true
            }
            Upsert::Stale => false,
        }
    }

    /// Removes a QSO at revision `rev`; broadcasts only when an entry was actually removed,
    /// under the lock like `publish` (research R3).
    pub fn remove(&self, id: &str, rev: u32) -> bool {
        let mut history = self.lock_history();
        match history.remove(id, rev) {
            Removal::Removed => {
                let _ = self
                    .changes
                    .send(HistoryChange::Remove { id: id.to_string() });
                true
            }
            Removal::NotPresent => false,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HistoryChange> {
        self.changes.subscribe()
    }

    pub fn history_len(&self) -> usize {
        self.lock_history().len()
    }
}

/// A started service: bound addresses plus control over its tasks.
pub struct RunningService {
    pub udp_addr: SocketAddr,
    pub http_addr: SocketAddr,
    pipeline: Pipeline,
    shutdown: watch::Sender<bool>,
    tasks: JoinSet<()>,
    /// Where the history is saved at the stop (feature 014); `None` when there is no file.
    history_file: Option<PathBuf>,
    /// File records on bands not considered, written back verbatim (R8).
    parked: Vec<(String, AdifRecord)>,
}

impl RunningService {
    pub fn subscribe(&self) -> broadcast::Receiver<HistoryChange> {
        self.pipeline.subscribe()
    }

    pub fn history_len(&self) -> usize {
        self.pipeline.history_len()
    }

    pub fn latest_score(&self) -> Option<Arc<ScoreState>> {
        self.pipeline.latest_score()
    }

    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// Signals every task to stop and waits up to 2 s for them; stragglers are aborted. Then
    /// saves the history file, if any (feature 014, FR-012): always, even if the file was
    /// deleted meanwhile (FR-003). A failed save is logged; the stop still completes.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown.send(true);
        let drained = tokio::time::timeout(Duration::from_secs(2), async {
            while self.tasks.join_next().await.is_some() {}
        })
        .await;
        if drained.is_err() {
            tracing::warn!("tasks did not stop within 2 s; aborting");
            self.tasks.abort_all();
            while self.tasks.join_next().await.is_some() {}
        }
        if let Some(path) = &self.history_file {
            save_history(path, &self.pipeline.records_to_save(&self.parked));
        }
    }
}

/// Unix seconds now; a clock before 1970 counts as 1970.
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

/// Writes the history file and logs the outcome (contracts/history-file.md).
fn save_history(path: &Path, records: &[AdifRecord]) -> bool {
    match crate::history_file::write::save(path, records, now_unix()) {
        Ok(()) => {
            tracing::info!(
                "history saved to {}: {} QSOs",
                path.display(),
                records.len()
            );
            true
        }
        Err(e) => {
            tracing::error!("cannot save the history: {e}");
            false
        }
    }
}

/// Reads the history file into events (feature 014, R2, R11). A missing file is an empty
/// history. An unreadable one is moved aside in the service, and is an error in enrich-only
/// mode or when it cannot be moved.
fn load_history(
    path: &Path,
    config: &Config,
    mode: crate::history_file::restore::Mode,
) -> Result<crate::ingest::Restored> {
    use crate::history_file::read::{Read, move_aside, read};
    use crate::history_file::restore::Mode;
    let error = |message: String| crate::errors::Error::HistoryFile {
        path: path.to_path_buf(),
        message,
    };
    let (records, malformed) = match (read(path), mode) {
        (Read::Missing, Mode::Service) => {
            tracing::info!("history file {}: none yet, starting empty", path.display());
            return Ok(crate::ingest::Restored::default());
        }
        (Read::Missing, Mode::EnrichOnly) => return Err(error("no such file".to_string())),
        (Read::Unreadable(reason), Mode::EnrichOnly) => return Err(error(reason)),
        (Read::Unreadable(reason), Mode::Service) => match move_aside(path, now_unix()) {
            Ok(moved) => {
                tracing::warn!(
                    "history file {} is unreadable ({reason}); moved to {}",
                    path.display(),
                    moved.display()
                );
                return Ok(crate::ingest::Restored::default());
            }
            Err(e) => return Err(error(format!("{reason}; {e}"))),
        },
        (Read::Records { records, malformed }, _) => (records, malformed),
    };
    let filter = match mode {
        Mode::Service => config.band_filter.as_ref(),
        Mode::EnrichOnly => None,
    };
    let source = path.display().to_string();
    let mut restored =
        crate::ingest::restored_events(records, &config.station, mode, filter, &source);
    for reason in &malformed {
        tracing::warn!("history file {source}: skipping a malformed record: {reason}");
    }
    restored.report.skipped += malformed.len();
    let r = &restored.report;
    tracing::info!(
        "history file {source}: loaded {} QSOs ({} reused, {} to enrich, {} on other bands), \
         skipped {} malformed",
        r.loaded,
        r.reused,
        r.to_enrich,
        r.parked,
        r.skipped
    );
    Ok(restored)
}

/// Passes restored events through `gate` in file order (R2): forwarded events are remembered as
/// pending with the record as loaded; parked ones are returned with their history id.
fn gate_restored(
    restored: crate::ingest::Restored,
    gate: &mut crate::ingest::gate::ResendGate,
    pipeline: &Pipeline,
) -> (Vec<crate::ingest::QsoEvent>, Vec<(String, AdifRecord)>) {
    use crate::ingest::gate::GateDecision;
    let mut forwarded = Vec::with_capacity(restored.events.len());
    for (mut event, loaded) in restored.events {
        if gate.check(&mut event) == GateDecision::Drop {
            continue;
        }
        pipeline.pending_insert(&event.id, event.rev, loaded);
        forwarded.push(event);
    }
    let mut parked = Vec::with_capacity(restored.parked.len());
    for mut event in restored.parked {
        if gate.check(&mut event) == GateDecision::Drop {
            continue;
        }
        if let Some(record) = event.saved {
            parked.push((event.id, record));
        }
    }
    (forwarded, parked)
}

/// Sends restored events to the restore queue, stopping at shutdown.
async fn feed(
    events: Vec<crate::ingest::QsoEvent>,
    queue: async_channel::Sender<crate::ingest::QsoEvent>,
    mut shutdown: watch::Receiver<bool>,
) {
    for event in events {
        tokio::select! {
            _ = shutdown.wait_for(|stop| *stop) => return,
            sent = queue.send(event) => if sent.is_err() { return },
        }
    }
}

/// A lookup source answering from the country data.
pub fn cty_source(data: Arc<crate::enrich::cty::CountryData>) -> LookupSource {
    Arc::new(move |call: String| {
        let candidate = data.lookup(&call);
        Box::pin(async move { Ok(candidate) })
    })
}

/// A lookup source answering from an online provider.
pub fn provider_source(client: Arc<crate::enrich::provider::ProviderClient>) -> LookupSource {
    Arc::new(move |call: String| {
        let client = Arc::clone(&client);
        Box::pin(async move { client.lookup(&call).await })
    })
}

/// The production sources, their names (feature 014, R7) and the country data.
type BuiltSources = (
    Vec<LookupSource>,
    Vec<Option<&'static str>>,
    Option<Arc<CountryData>>,
);

/// Builds the production sources: every provider with credentials, then the country data
/// (downloaded or refreshed first, R13). The country data is also returned, to resolve the
/// station's entity (feature 004).
async fn build_sources(config: &Config) -> Result<BuiltSources> {
    let http = crate::enrich::provider::http_client(config.lookup.timeout)?;
    let clients = crate::enrich::provider::clients(&config.lookup, &http);
    let mut names: Vec<Option<&'static str>> = clients.iter().map(|c| Some(c.name())).collect();
    let mut sources: Vec<LookupSource> = clients.into_iter().map(provider_source).collect();
    let cty = &config.country_data;
    let refresh = crate::enrich::cty::load_or_refresh(
        &cty.path,
        &cty.url,
        cty.max_age_days,
        std::time::SystemTime::now(),
        |url| crate::enrich::cty::fetch_url(http.clone(), url),
    );
    let mut country = None;
    match tokio::time::timeout(config.lookup.timeout * 4, refresh).await {
        Ok(Some(data)) => {
            let data = Arc::new(data);
            sources.push(cty_source(Arc::clone(&data)));
            names.push(Some("cty"));
            country = Some(data);
        }
        Ok(None) => {}
        Err(_) => {
            tracing::warn!("country data not ready in time; country-level positions disabled")
        }
    }
    Ok((sources, names, country))
}

/// The station's entity and its extent for `hello` (feature 004, FR-010, FR-025). `None` when
/// any piece is missing; each case is logged at debug level.
pub fn station_entity(
    station: &crate::config::StationConfig,
    country: Option<&CountryData>,
    extents: &ExtentTable,
) -> Option<EntityView> {
    let Some(call) = station.callsign.as_deref() else {
        tracing::debug!("no station callsign: auto-zoom frames a radius around the QTH");
        return None;
    };
    let Some(country) = country else {
        tracing::debug!("no country data: the station entity is unknown");
        return None;
    };
    let Some(info) = country.entity_info(call) else {
        tracing::debug!(call, "station callsign matches no entity");
        return None;
    };
    let (prefix, name) = (info.prefix, info.name);
    let Some(bounds) = extents.get(prefix) else {
        tracing::debug!(prefix, "no bundled extent for the station entity");
        return None;
    };
    Some(EntityView {
        prefix: prefix.to_string(),
        name: name.to_string(),
        bounds,
    })
}

/// The startup line naming the considered bands, in frequency order (feature 003, FR-006).
pub fn bands_log_line(filter: Option<&crate::band_filter::BandSet>) -> String {
    match filter {
        Some(set) => format!("bands considered: {set}"),
        None => "bands considered: all".to_string(),
    }
}

/// Binds the UDP and HTTP listeners (port 0 is allowed) and spawns every task. With injected
/// `sources`, no country data is used; tests that need it call [`start_with_country`].
pub async fn start(config: Config, sources: LookupSources) -> Result<RunningService> {
    start_with_country(config, sources, None).await
}

/// Like [`start`], with country data injected next to the sources (tests, feature 005 R2).
/// When `sources` is `None`, both are built from the configuration and `country` is ignored.
pub async fn start_with_country(
    config: Config,
    sources: LookupSources,
    country: Option<Arc<CountryData>>,
) -> Result<RunningService> {
    crate::install_crypto_provider();
    let extents = ExtentTable::embedded()?;
    let (sources, names, country) = match sources {
        Some(sources) => (sources, Vec::new(), country),
        None => build_sources(&config).await?,
    };
    let entity = station_entity(&config.station, country.as_deref(), &extents);
    let hello_station = HelloStation::new(&config.station, entity);
    let lookups =
        crate::enrich::Lookups::new(sources, country, Arc::new(extents), &config).with_names(names);
    tracing::info!("{}", bands_log_line(config.band_filter.as_ref()));
    let udp = UdpSocket::bind((config.bind_address, config.udp_port)).await?;
    let http = TcpListener::bind((config.bind_address, config.http_port)).await?;
    let udp_addr = udp.local_addr()?;
    let http_addr = http.local_addr()?;

    let pipeline = Pipeline::new(config.broadcast_queue);
    let history_file = config.history_file.clone();
    let restored = match &history_file {
        Some(path) => load_history(path, &config, crate::history_file::restore::Mode::Service)?,
        None => crate::ingest::Restored::default(),
    };
    let mut gate = crate::ingest::gate::ResendGate::new();
    let (restored, parked) = gate_restored(restored, &mut gate, &pipeline);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut tasks = JoinSet::new();
    let config = Arc::new(config);
    spawn_restore(
        &mut tasks,
        Arc::clone(&config),
        pipeline.clone(),
        shutdown_rx.clone(),
        lookups.clone(),
        restored,
    );
    spawn_tasks(
        &mut tasks,
        config,
        pipeline.clone(),
        shutdown_rx,
        (udp, http),
        lookups,
        (hello_station, gate),
    );

    tracing::info!(%udp_addr, %http_addr, "live-qso-map started");
    Ok(RunningService {
        udp_addr,
        http_addr,
        pipeline,
        shutdown: shutdown_tx,
        tasks,
        history_file,
        parked,
    })
}

/// Restored QSOs have their own queue and worker, so live QSOs never wait for them (R2).
fn spawn_restore(
    tasks: &mut JoinSet<()>,
    config: Arc<Config>,
    pipeline: Pipeline,
    shutdown: watch::Receiver<bool>,
    lookups: crate::enrich::Lookups,
    restored: Vec<crate::ingest::QsoEvent>,
) {
    if restored.is_empty() {
        return;
    }
    let (restore_tx, restore_rx) = async_channel::bounded(config.ingest_queue);
    tasks.spawn(feed(restored, restore_tx, shutdown.clone()));
    tasks.spawn(crate::enrich::run_worker(
        restore_rx, pipeline, config, lookups, shutdown,
    ));
}

fn spawn_tasks(
    tasks: &mut JoinSet<()>,
    config: Arc<Config>,
    pipeline: Pipeline,
    shutdown: watch::Receiver<bool>,
    (udp, http): (UdpSocket, TcpListener),
    lookups: crate::enrich::Lookups,
    (hello_station, gate): (HelloStation, crate::ingest::gate::ResendGate),
) {
    let (queue_tx, queue_rx) = async_channel::bounded(config.ingest_queue);
    tasks.spawn(crate::ingest::run(
        udp,
        config.station.clone(),
        queue_tx,
        gate,
        config.band_filter.clone(),
        pipeline.clone(),
        shutdown.clone(),
    ));
    for _ in 0..config.workers {
        tasks.spawn(crate::enrich::run_worker(
            queue_rx.clone(),
            pipeline.clone(),
            Arc::clone(&config),
            lookups.clone(),
            shutdown.clone(),
        ));
    }
    tasks.spawn(crate::web::serve(
        http,
        pipeline,
        config.map.clone(),
        config.band_filter.clone(),
        hello_station,
        shutdown,
    ));
}

/// What an enrich-only run did (feature 014, FR-020).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnrichReport {
    pub load: crate::history_file::LoadReport,
    /// Records whose position came from the record itself (`logger`).
    pub placed_by_record: usize,
    /// Records placed by a lookup (an online provider or the country data).
    pub placed_by_lookup: usize,
    /// Records with `APP_LIVEQSOMAP_LOOKUP_FAILED`.
    pub lookup_failed: usize,
    pub without_position: usize,
}

impl EnrichReport {
    fn count(load: crate::history_file::LoadReport, records: &[AdifRecord]) -> Self {
        let mut report = Self {
            load,
            ..Self::default()
        };
        for record in records {
            if record
                .get_non_empty(crate::history_file::LOOKUP_FAILED_FIELD)
                .is_some()
            {
                report.lookup_failed += 1;
                continue;
            }
            match record.get_non_empty("APP_LIVEQSOMAP_POS_SOURCE") {
                None => report.without_position += 1,
                Some("logger") => report.placed_by_record += 1,
                Some(_) => report.placed_by_lookup += 1,
            }
        }
        report
    }
}

/// `--enrich-only`: loads `path`, completes every record on every band, writes it back and
/// returns (feature 014, R12). No listener is opened. A missing or unreadable file is an error
/// and nothing is written. When `stop` resolves first, what was completed is saved.
pub async fn enrich_only(
    config: Config,
    path: &Path,
    sources: LookupSources,
    country: Option<Arc<CountryData>>,
    stop: impl Future<Output = ()>,
) -> Result<EnrichReport> {
    crate::install_crypto_provider();
    let restored = load_history(
        path,
        &config,
        crate::history_file::restore::Mode::EnrichOnly,
    )?;
    let load = restored.report.clone();
    let extents = ExtentTable::embedded()?;
    let (sources, names, country) = match sources {
        Some(sources) => (sources, Vec::new(), country),
        None => build_sources(&config).await?,
    };
    let lookups =
        crate::enrich::Lookups::new(sources, country, Arc::new(extents), &config).with_names(names);
    let pipeline = Pipeline::new(config.broadcast_queue);
    let mut gate = crate::ingest::gate::ResendGate::new();
    let (events, _) = gate_restored(restored, &mut gate, &pipeline);

    let config = Arc::new(config);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (queue_tx, queue_rx) = async_channel::bounded(config.ingest_queue);
    let mut tasks = JoinSet::new();
    tasks.spawn(feed(events, queue_tx, shutdown_rx.clone()));
    for _ in 0..config.workers {
        tasks.spawn(crate::enrich::run_worker(
            queue_rx.clone(),
            pipeline.clone(),
            Arc::clone(&config),
            lookups.clone(),
            shutdown_rx.clone(),
        ));
    }
    drop(queue_rx);
    tokio::pin!(stop);
    let finished = tokio::select! {
        _ = async { while tasks.join_next().await.is_some() {} } => true,
        _ = &mut stop => false,
    };
    if !finished {
        tracing::info!("interrupted; saving what was completed");
        let _ = shutdown_tx.send(true);
        let drained = tokio::time::timeout(Duration::from_secs(2), async {
            while tasks.join_next().await.is_some() {}
        })
        .await;
        if drained.is_err() {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        }
    }
    let records = pipeline.records_to_save(&[]);
    if !save_history(path, &records) {
        return Err(crate::errors::Error::HistoryFile {
            path: path.to_path_buf(),
            message: "cannot save the completed file".to_string(),
        });
    }
    let report = EnrichReport::count(load, &records);
    tracing::info!(
        "enriched {}: {} QSOs, {} placed from the record, {} by lookup, {} with a failed \
         lookup, {} without a position",
        path.display(),
        records.len(),
        report.placed_by_record,
        report.placed_by_lookup,
        report.lookup_failed,
        report.without_position
    );
    Ok(report)
}

/// Starts the service and runs it until Ctrl+C or SIGTERM.
pub async fn run(config: Config) -> Result<()> {
    let service = start(config, None).await?;
    wait_for_signal().await;
    tracing::info!("shutting down");
    service.shutdown().await;
    Ok(())
}

/// Resolves on Ctrl+C or, on Unix, SIGTERM.
pub async fn wait_for_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = ctrl_c => {}
                    _ = term.recv() => {}
                }
            }
            Err(e) => {
                tracing::warn!("cannot listen for SIGTERM: {e}");
                let _ = ctrl_c.await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::score::ContestScore;

    #[test]
    fn station_entity_needs_callsign_country_data_and_extent() {
        let country = crate::enrich::cty::CountryData::parse(include_str!(
            "../tests/fixtures/cty-excerpt.dat"
        ));
        let extents =
            ExtentTable::parse(r#"{"source":"t","extents":{"IS":[8.13,38.86,9.83,41.31]}}"#)
                .unwrap();
        let mut station = crate::config::StationConfig {
            callsign: Some("is0mzk".into()),
            ..Default::default()
        };
        assert_eq!(
            station_entity(&station, Some(&country), &extents),
            Some(EntityView {
                prefix: "IS".into(),
                name: "Sardinia".into(),
                bounds: [8.13, 38.86, 9.83, 41.31],
            })
        );
        assert_eq!(station_entity(&station, None, &extents), None);
        station.callsign = Some("I2XYZ".into()); // entity known, no extent
        assert_eq!(station_entity(&station, Some(&country), &extents), None);
        station.callsign = Some("QQ9QQ".into());
        assert_eq!(station_entity(&station, Some(&country), &extents), None);
        station.callsign = None;
        assert_eq!(station_entity(&station, Some(&country), &extents), None);
    }
    use crate::adif::AdifRecord;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::{Duration, Instant};

    fn qso(id: &str, rev: u32) -> EnrichedQso {
        EnrichedQso {
            id: id.to_string(),
            rev,
            sort_ts: 0,
            record: AdifRecord::new(),
            remote: None,
            station: None,
            entity: None,
        }
    }

    fn test_config() -> Config {
        let mut config = Config {
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            udp_port: 0,
            http_port: 0,
            ..Config::default()
        };
        config.country_data.path = "/nonexistent/cty.dat".into();
        config.country_data.url = "http://127.0.0.1:9/cty.dat".into();
        config
    }

    #[test]
    fn publish_of_stale_rev_sends_nothing() {
        let pipeline = Pipeline::new(16);
        let mut rx = pipeline.subscribe();
        assert!(pipeline.publish(qso("A", 2)));
        assert!(!pipeline.publish(qso("A", 1)));
        assert!(!pipeline.publish(qso("A", 2)));
        let Ok(HistoryChange::Upsert(first)) = rx.try_recv() else {
            panic!("expected an upsert");
        };
        assert_eq!(first.rev, 2);
        assert!(rx.try_recv().is_err());
        assert_eq!(pipeline.history_len(), 1);
    }

    #[test]
    fn publish_without_subscribers_still_updates_history() {
        let pipeline = Pipeline::new(16);
        assert!(pipeline.publish(qso("A", 1)));
        assert_eq!(pipeline.history_len(), 1);
    }

    #[tokio::test]
    async fn start_binds_ephemeral_ports() {
        let service = start(test_config(), None).await.unwrap();
        assert_ne!(service.udp_addr.port(), 0);
        assert_ne!(service.http_addr.port(), 0);
        assert_eq!(service.history_len(), 0);
        service.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_completes_quickly() {
        let service = start(test_config(), None).await.unwrap();
        let begin = Instant::now();
        service.shutdown().await;
        assert!(begin.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn bands_log_line_lists_the_effective_set() {
        assert_eq!(bands_log_line(None), "bands considered: all");
        assert_eq!(
            bands_log_line(crate::config::Config::default().band_filter.as_ref()),
            "bands considered: 160m 80m 40m 20m 15m 10m"
        );
        let set = crate::band_filter::resolve(&["hf", "2m", "-60m"]).unwrap();
        assert_eq!(
            bands_log_line(Some(&set)),
            "bands considered: 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m"
        );
    }

    #[test]
    fn remove_broadcasts_only_when_something_was_removed() {
        let pipeline = Pipeline::new(16);
        pipeline.publish(qso("A", 1));
        let mut rx = pipeline.subscribe();
        assert!(!pipeline.remove("ZZ", 2));
        assert!(rx.try_recv().is_err());
        assert!(pipeline.remove("A", 2));
        match rx.try_recv() {
            Ok(HistoryChange::Remove { id }) => assert_eq!(id, "A"),
            other => panic!("expected a removal, got {other:?}"),
        }
        assert_eq!(pipeline.history_len(), 0);
        assert!(!pipeline.publish(qso("A", 2)), "stale revision stays out");
        assert!(rx.try_recv().is_err());
    }

    // --- contest score (feature 007, T005) ----------------------------------------------------

    fn score(score: u64) -> ContestScore {
        ContestScore {
            score,
            qsos: 7,
            mults: 11,
        }
    }

    #[test]
    fn a_new_pipeline_holds_no_score() {
        assert!(Pipeline::new(16).latest_score().is_none());
    }

    #[test]
    fn publish_score_stores_and_broadcasts() {
        let pipeline = Pipeline::new(16);
        let mut rx = pipeline.subscribe();
        pipeline.publish_score(score(132));
        let held = pipeline.latest_score().unwrap();
        assert_eq!(held.score, score(132));
        let at = held.received_at.as_bytes();
        assert_eq!(at.len(), 20, "{}", held.received_at);
        assert!(
            held.received_at.ends_with('Z') && at[10] == b'T' && at[4] == b'-',
            "{}",
            held.received_at
        );
        match rx.try_recv() {
            Ok(HistoryChange::Score(sent)) => assert_eq!(sent.score, score(132)),
            other => panic!("expected a score, got {other:?}"),
        }
        assert_eq!(pipeline.history_len(), 0, "a score is not a QSO (FR-004)");
    }

    #[test]
    fn an_identical_score_is_broadcast_again_and_refreshed() {
        let pipeline = Pipeline::new(16);
        let mut rx = pipeline.subscribe();
        pipeline.publish_score(score(90));
        let first = pipeline.latest_score().unwrap().received;
        std::thread::sleep(Duration::from_millis(5));
        pipeline.publish_score(score(90));
        assert!(pipeline.latest_score().unwrap().received > first);
        assert!(matches!(rx.try_recv(), Ok(HistoryChange::Score(_))));
        assert!(matches!(rx.try_recv(), Ok(HistoryChange::Score(_))));
    }

    // --- pending QSOs and what is saved (feature 014, T025, R9) -----------------------------

    fn rec(call: &str, time: &str) -> AdifRecord {
        let mut r = AdifRecord::new();
        r.set("CALL", call);
        r.set("QSO_DATE", "20261010");
        r.set("TIME_ON", time);
        r
    }

    fn placed(id: &str, rev: u32, call: &str, time: &str) -> EnrichedQso {
        let record = rec(call, time);
        EnrichedQso {
            id: id.to_string(),
            rev,
            sort_ts: crate::history::qso_timestamp("20261010", time).unwrap(),
            record,
            remote: None,
            station: None,
            entity: None,
        }
    }

    fn calls(records: &[AdifRecord]) -> Vec<&str> {
        records.iter().map(|r| r.get("CALL").unwrap()).collect()
    }

    #[test]
    fn records_to_save_merge_history_pending_and_parked_in_qso_time_order() {
        let pipeline = Pipeline::new(16);
        pipeline.publish(placed("B", 1, "B", "1202"));
        pipeline.publish(placed("D", 1, "D", "1204"));
        pipeline.pending_insert("A", 1, rec("A", "1201"));
        let parked = vec![("C".to_string(), rec("C", "1203"))];
        assert_eq!(
            calls(&pipeline.records_to_save(&parked)),
            ["A", "B", "C", "D"]
        );
    }

    #[test]
    fn a_newer_pending_version_beats_the_history() {
        let pipeline = Pipeline::new(16);
        pipeline.publish(placed("A", 1, "OLD", "1200"));
        pipeline.pending_insert("A", 2, rec("NEW", "1200"));
        assert_eq!(calls(&pipeline.records_to_save(&[])), ["NEW"]);
    }

    #[test]
    fn an_older_pending_version_loses_and_published_ones_are_forgotten() {
        let pipeline = Pipeline::new(16);
        pipeline.pending_insert("A", 1, rec("PENDING", "1200"));
        pipeline.publish(placed("A", 2, "PLACED", "1200"));
        assert_eq!(calls(&pipeline.records_to_save(&[])), ["PLACED"]);
        pipeline.pending_remove("A", 1);
        pipeline.pending_insert("B", 1, rec("B", "1201"));
        pipeline.pending_insert("B", 2, rec("B2", "1201"));
        pipeline.pending_remove("B", 1);
        assert_eq!(
            calls(&pipeline.records_to_save(&[])),
            ["PLACED", "B2"],
            "removing rev 1 keeps the pending rev 2"
        );
        pipeline.pending_remove("B", 2);
        assert_eq!(calls(&pipeline.records_to_save(&[])), ["PLACED"]);
    }

    #[test]
    fn a_parked_record_loses_to_a_qso_with_its_id() {
        let pipeline = Pipeline::new(16);
        pipeline.publish(placed("A", 2, "MOVED", "1200"));
        let parked = vec![("A".to_string(), rec("PARKED", "1200"))];
        assert_eq!(calls(&pipeline.records_to_save(&parked)), ["MOVED"]);
    }
}
