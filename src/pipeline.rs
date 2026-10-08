//! Service wiring: history, broadcast (channel 2), listeners and task lifecycle
//! (plan "Summary", research R2/R4).

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::errors::Result;
use crate::history::{EnrichedQso, History, Upsert};
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

/// A change to the history, fanned out to every page session (channel 2).
#[derive(Debug, Clone)]
pub enum HistoryChange {
    Upsert(Arc<EnrichedQso>),
}

/// Shared history plus the broadcast sender.
#[derive(Clone)]
pub struct Pipeline {
    history: Arc<Mutex<History>>,
    changes: broadcast::Sender<HistoryChange>,
}

impl Pipeline {
    pub fn new(history_size: usize, broadcast_queue: usize) -> Self {
        let (changes, _) = broadcast::channel(broadcast_queue.max(1));
        Self {
            history: Arc::new(Mutex::new(History::new(history_size))),
            changes,
        }
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
}

impl RunningService {
    pub fn subscribe(&self) -> broadcast::Receiver<HistoryChange> {
        self.pipeline.subscribe()
    }

    pub fn history_len(&self) -> usize {
        self.pipeline.history_len()
    }

    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// Signals every task to stop and waits up to 2 s for them; stragglers are aborted.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown.send(true);
        let drained = tokio::time::timeout(Duration::from_secs(2), async {
            while self.tasks.join_next().await.is_some() {}
        })
        .await;
        if drained.is_err() {
            tracing::warn!("tasks did not stop within 2 s; aborting");
            self.tasks.abort_all();
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

/// Builds the production sources: every provider with credentials, then the country data
/// (downloaded or refreshed first, R13).
async fn build_sources(config: &Config) -> Result<Vec<LookupSource>> {
    let http = crate::enrich::provider::http_client(config.lookup.timeout)?;
    let mut sources: Vec<LookupSource> = crate::enrich::provider::clients(&config.lookup, &http)
        .into_iter()
        .map(provider_source)
        .collect();
    let cty = &config.country_data;
    let refresh = crate::enrich::cty::load_or_refresh(
        &cty.path,
        &cty.url,
        cty.max_age_days,
        std::time::SystemTime::now(),
        |url| crate::enrich::cty::fetch_url(http.clone(), url),
    );
    match tokio::time::timeout(config.lookup.timeout * 4, refresh).await {
        Ok(Some(data)) => sources.push(cty_source(Arc::new(data))),
        Ok(None) => {}
        Err(_) => {
            tracing::warn!("country data not ready in time; country-level positions disabled")
        }
    }
    Ok(sources)
}

/// Binds the UDP and HTTP listeners (port 0 is allowed) and spawns every task.
pub async fn start(config: Config, sources: LookupSources) -> Result<RunningService> {
    crate::install_crypto_provider();
    let sources = match sources {
        Some(sources) => sources,
        None => build_sources(&config).await?,
    };
    let lookups = crate::enrich::Lookups::new(sources, &config);
    let udp = UdpSocket::bind((config.bind_address, config.udp_port)).await?;
    let http = TcpListener::bind((config.bind_address, config.http_port)).await?;
    let udp_addr = udp.local_addr()?;
    let http_addr = http.local_addr()?;

    let pipeline = Pipeline::new(config.history_size, config.broadcast_queue);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut tasks = JoinSet::new();
    spawn_tasks(
        &mut tasks,
        Arc::new(config),
        pipeline.clone(),
        shutdown_rx,
        udp,
        http,
        lookups,
    );

    tracing::info!(%udp_addr, %http_addr, "live-qso-map started");
    Ok(RunningService {
        udp_addr,
        http_addr,
        pipeline,
        shutdown: shutdown_tx,
        tasks,
    })
}

fn spawn_tasks(
    tasks: &mut JoinSet<()>,
    config: Arc<Config>,
    pipeline: Pipeline,
    shutdown: watch::Receiver<bool>,
    udp: UdpSocket,
    http: TcpListener,
    lookups: crate::enrich::Lookups,
) {
    let (queue_tx, queue_rx) = async_channel::bounded(config.ingest_queue);
    tasks.spawn(crate::ingest::run(
        udp,
        config.station.clone(),
        queue_tx,
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
        shutdown,
    ));
}

/// Starts the service and runs it until Ctrl+C or SIGTERM.
pub async fn run(config: Config) -> Result<()> {
    let service = start(config, None).await?;
    wait_for_signal().await;
    tracing::info!("shutting down");
    service.shutdown().await;
    Ok(())
}

async fn wait_for_signal() {
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
        let pipeline = Pipeline::new(0, 16);
        let mut rx = pipeline.subscribe();
        assert!(pipeline.publish(qso("A", 2)));
        assert!(!pipeline.publish(qso("A", 1)));
        assert!(!pipeline.publish(qso("A", 2)));
        let HistoryChange::Upsert(first) = rx.try_recv().unwrap();
        assert_eq!(first.rev, 2);
        assert!(rx.try_recv().is_err());
        assert_eq!(pipeline.history_len(), 1);
    }

    #[test]
    fn publish_without_subscribers_still_updates_history() {
        let pipeline = Pipeline::new(0, 16);
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
}
