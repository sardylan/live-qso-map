//! Per-page session: a pure state machine plus an async driver (data-model "MapSession").

use std::future::Future;

use tokio::sync::{broadcast, watch};

use crate::band_filter::BandSet;
use crate::config::MapConfig;
use crate::errors::Result;
use crate::history::History;
use crate::pipeline::{HistoryChange, Pipeline, ScoreState};
use crate::web::protocol::{
    ClientMessage, DEFAULT_LIMIT, HelloStation, QsoView, ServerMessage, is_allowed_limit,
};

/// Keeps the valid band names, in canonical form; unknown names are dropped (logged at debug
/// level), and a list left empty means every band (contracts/websocket-protocol.md).
fn clean_bands(bands: Option<Vec<String>>) -> Option<Vec<String>> {
    let bands = bands?;
    let valid: Vec<String> = bands
        .iter()
        .filter_map(|b| {
            let canonical = crate::band::canonical_band(b);
            if canonical.is_none() {
                tracing::debug!(band = %b, "ignoring an unknown band in set_limit");
            }
            canonical.map(str::to_string)
        })
        .collect();
    (!valid.is_empty()).then_some(valid)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    limit: Option<usize>,
    /// The bands the page shows, canonical names; `None` for every band (feature 011).
    bands: Option<Vec<String>>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            limit: Some(DEFAULT_LIMIT),
            bands: None,
        }
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    /// `hello` and the window, then the latest score when one is held (feature 007).
    pub fn on_connect(
        &self,
        history: &History,
        score: Option<&ScoreState>,
        map: &MapConfig,
        filter: Option<&BandSet>,
        station: &HelloStation,
    ) -> Vec<ServerMessage> {
        let mut messages = vec![
            ServerMessage::hello(map, filter, station),
            self.snapshot(history),
        ];
        messages.extend(score.map(ServerMessage::score));
        messages
    }

    pub fn on_change(&self, change: &HistoryChange) -> Vec<ServerMessage> {
        match change {
            HistoryChange::Upsert(qso) => vec![ServerMessage::Upsert {
                qso: QsoView::from(qso.as_ref()),
            }],
            HistoryChange::Remove { id } => vec![ServerMessage::Remove { id: id.clone() }],
            HistoryChange::Score(state) => vec![ServerMessage::score(state)],
        }
    }

    pub fn on_client(&mut self, message: ClientMessage, history: &History) -> Vec<ServerMessage> {
        match message {
            ClientMessage::SetLimit { limit, bands } if is_allowed_limit(limit) => {
                self.limit = limit;
                self.bands = clean_bands(bands);
                vec![self.snapshot(history)]
            }
            ClientMessage::SetLimit { limit, .. } => {
                tracing::debug!(
                    ?limit,
                    "ignoring set_limit with a value outside the allowed set"
                );
                Vec::new()
            }
            ClientMessage::Unknown => Vec::new(),
        }
    }

    /// The page missed broadcasts: resend its whole window and the latest score.
    pub fn on_lagged(&self, history: &History, score: Option<&ScoreState>) -> Vec<ServerMessage> {
        let mut messages = vec![self.snapshot(history)];
        messages.extend(score.map(ServerMessage::score));
        messages
    }

    fn snapshot(&self, history: &History) -> ServerMessage {
        ServerMessage::Snapshot {
            limit: self.limit,
            qsos: history
                .latest_matching(self.limit, self.bands.as_deref())
                .iter()
                .map(|q| QsoView::from(q.as_ref()))
                .collect(),
        }
    }
}

/// The page connection as seen by the driver. The WebSocket adapter implements it; tests use
/// in-memory fakes.
pub trait MessageSink {
    fn send(&mut self, msg: ServerMessage) -> impl Future<Output = Result<()>> + Send;
    /// The next client message, or `None` once the page has gone away.
    fn recv(&mut self) -> impl Future<Output = Option<ClientMessage>> + Send;
}

/// Runs one page session until the page disconnects, a send fails or the service shuts down.
/// `changes` must be subscribed before calling, so no change between subscription and the
/// initial snapshot is lost (R4).
pub async fn drive<S: MessageSink>(
    mut sink: S,
    mut changes: broadcast::Receiver<HistoryChange>,
    pipeline: &Pipeline,
    map: &MapConfig,
    filter: Option<&BandSet>,
    station: &HelloStation,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut session = Session::new();
    let initial = {
        let score = pipeline.latest_score();
        let history = pipeline.lock_history();
        session.on_connect(&history, score.as_deref(), map, filter, station)
    };
    if send_all(&mut sink, initial).await.is_err() {
        return;
    }
    loop {
        let event = tokio::select! {
            _ = shutdown.wait_for(|stop| *stop) => return,
            incoming = sink.recv() => match incoming {
                Some(message) => Event::Client(message),
                None => return,
            },
            change = changes.recv() => match change {
                Ok(change) => Event::Change(change),
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::debug!(missed, "page fell behind; sending a snapshot");
                    Event::Lagged
                }
                Err(broadcast::error::RecvError::Closed) => return,
            },
        };
        let outgoing = match event {
            Event::Client(message) => {
                let history = pipeline.lock_history();
                session.on_client(message, &history)
            }
            Event::Change(change) => session.on_change(&change),
            Event::Lagged => {
                let score = pipeline.latest_score();
                let history = pipeline.lock_history();
                session.on_lagged(&history, score.as_deref())
            }
        };
        if send_all(&mut sink, outgoing).await.is_err() {
            return;
        }
    }
}

enum Event {
    Client(ClientMessage),
    Change(HistoryChange),
    Lagged,
}

async fn send_all<S: MessageSink>(sink: &mut S, messages: Vec<ServerMessage>) -> Result<()> {
    for message in messages {
        sink.send(message).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adif::AdifRecord;
    use crate::config::Config;
    use crate::history::EnrichedQso;
    use crate::ingest::score::ContestScore;
    use crate::pipeline::ScoreState;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::{Notify, mpsc, watch};

    fn qso(i: i64) -> EnrichedQso {
        let mut record = AdifRecord::new();
        record.set("CALL", format!("K{i}AA"));
        EnrichedQso {
            id: format!("Q{i:04}"),
            rev: 1,
            sort_ts: i,
            record,
            remote: None,
            station: None,
            entity: None,
        }
    }

    fn pipeline_with(n: i64, queue: usize) -> Pipeline {
        let p = Pipeline::new(0, queue);
        for i in 0..n {
            p.publish(qso(i));
        }
        p
    }

    fn kind(m: &ServerMessage) -> &'static str {
        match m {
            ServerMessage::Hello { .. } => "hello",
            ServerMessage::Snapshot { .. } => "snapshot",
            ServerMessage::Upsert { .. } => "upsert",
            ServerMessage::Remove { .. } => "remove",
            ServerMessage::Score { .. } => "score",
        }
    }

    #[test]
    fn connect_emits_hello_then_snapshot_of_20() {
        let p = pipeline_with(25, 16);
        let session = Session::new();
        let msgs = session.on_connect(
            &p.lock_history(),
            None,
            &Config::default().map,
            None,
            &HelloStation::default(),
        );
        assert_eq!(
            msgs.iter().map(kind).collect::<Vec<_>>(),
            ["hello", "snapshot"]
        );
        let ServerMessage::Snapshot { limit, qsos } = &msgs[1] else {
            panic!("expected snapshot");
        };
        assert_eq!(*limit, Some(20));
        assert_eq!(qsos.len(), 20);
        assert_eq!(qsos[0].id, "Q0005");
        assert_eq!(qsos[19].id, "Q0024");
    }

    #[test]
    fn change_emits_upsert() {
        let p = pipeline_with(0, 16);
        let mut rx = p.subscribe();
        p.publish(qso(1));
        let change = rx.try_recv().unwrap();
        let msgs = Session::new().on_change(&change);
        assert_eq!(msgs.len(), 1);
        let ServerMessage::Upsert { qso } = &msgs[0] else {
            panic!("expected upsert");
        };
        assert_eq!(qso.id, "Q0001");
    }

    #[test]
    fn unknown_client_message_is_ignored() {
        let p = pipeline_with(3, 16);
        let mut session = Session::new();
        assert!(
            session
                .on_client(ClientMessage::Unknown, &p.lock_history())
                .is_empty()
        );
    }

    fn snapshot_len(m: &ServerMessage) -> (Option<usize>, usize) {
        match m {
            ServerMessage::Snapshot { limit, qsos } => (*limit, qsos.len()),
            other => panic!("expected snapshot, got {}", kind(other)),
        }
    }

    #[test]
    fn set_limit_resends_the_window() {
        let p = pipeline_with(150, 16);
        let mut session = Session::new();
        let msgs = session.on_client(
            ClientMessage::SetLimit {
                limit: Some(100),
                bands: None,
            },
            &p.lock_history(),
        );
        assert_eq!(msgs.len(), 1);
        assert_eq!(snapshot_len(&msgs[0]), (Some(100), 100));
        assert_eq!(session.limit(), Some(100));

        let msgs = session.on_client(
            ClientMessage::SetLimit {
                limit: None,
                bands: None,
            },
            &p.lock_history(),
        );
        assert_eq!(snapshot_len(&msgs[0]), (None, 150));
    }

    #[test]
    fn disallowed_limit_is_ignored() {
        let p = pipeline_with(30, 16);
        let mut session = Session::new();
        assert!(
            session
                .on_client(
                    ClientMessage::SetLimit {
                        limit: Some(7),
                        bands: None
                    },
                    &p.lock_history()
                )
                .is_empty()
        );
        assert_eq!(session.limit(), Some(20));
    }

    #[test]
    fn lag_resends_the_current_window() {
        let p = pipeline_with(60, 16);
        let mut session = Session::new();
        session.on_client(
            ClientMessage::SetLimit {
                limit: Some(50),
                bands: None,
            },
            &p.lock_history(),
        );
        let msgs = session.on_lagged(&p.lock_history(), None);
        assert_eq!(msgs.len(), 1);
        assert_eq!(snapshot_len(&msgs[0]), (Some(50), 50));
    }

    /// A sink that blocks every send until the gate opens, and replays scripted client input.
    struct GatedSink {
        gate: Arc<Notify>,
        open: Arc<std::sync::atomic::AtomicBool>,
        sent: mpsc::UnboundedSender<ServerMessage>,
        incoming: VecDeque<ClientMessage>,
    }

    impl MessageSink for GatedSink {
        async fn send(&mut self, msg: ServerMessage) -> crate::errors::Result<()> {
            while !self.open.load(std::sync::atomic::Ordering::SeqCst) {
                self.gate.notified().await;
            }
            let _ = self.sent.send(msg);
            Ok(())
        }

        async fn recv(&mut self) -> Option<ClientMessage> {
            match self.incoming.pop_front() {
                Some(m) => Some(m),
                None => std::future::pending().await,
            }
        }
    }

    #[tokio::test]
    async fn driver_recovers_from_lag_with_one_snapshot() {
        let pipeline = Pipeline::new(0, 4);
        let gate = Arc::new(Notify::new());
        let open = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (sent_tx, mut sent_rx) = mpsc::unbounded_channel();
        let sink = GatedSink {
            gate: gate.clone(),
            open: open.clone(),
            sent: sent_tx,
            incoming: VecDeque::new(),
        };
        let (_stop_tx, stop_rx) = watch::channel(false);
        let map = Config::default().map;
        let rx = pipeline.subscribe();
        let snapshot_source = pipeline.clone();
        let driver = tokio::spawn(async move {
            drive(
                sink,
                rx,
                &snapshot_source,
                &map,
                None,
                &HelloStation::default(),
                stop_rx,
            )
            .await;
        });

        // The driver is blocked sending `hello`; overflow the 4-slot broadcast queue.
        tokio::task::yield_now().await;
        for i in 0..10 {
            pipeline.publish(qso(i));
        }
        open.store(true, std::sync::atomic::Ordering::SeqCst);
        gate.notify_waiters();

        let mut kinds = Vec::new();
        while let Ok(Some(m)) =
            tokio::time::timeout(std::time::Duration::from_millis(300), sent_rx.recv()).await
        {
            kinds.push(kind(&m));
        }
        driver.abort();

        assert_eq!(&kinds[..2], ["hello", "snapshot"]);
        let after: Vec<_> = kinds[2..].to_vec();
        assert_eq!(
            after.iter().filter(|k| **k == "snapshot").count(),
            1,
            "{kinds:?}"
        );
        assert_eq!(after.first(), Some(&"snapshot"), "{kinds:?}");
    }

    #[test]
    fn removal_change_emits_remove() {
        let msgs = Session::new().on_change(&HistoryChange::Remove { id: "Q0001".into() });
        assert_eq!(msgs, vec![ServerMessage::Remove { id: "Q0001".into() }]);
    }

    // --- contest score (feature 007, T007) ----------------------------------------------------

    fn held_score() -> ScoreState {
        ScoreState::now(ContestScore {
            score: 132,
            qsos: 7,
            mults: 11,
        })
    }

    #[test]
    fn connect_sends_the_score_after_the_snapshot_when_one_is_held() {
        let p = pipeline_with(3, 16);
        let map = Config::default().map;
        let station = HelloStation::default();
        let none = Session::new().on_connect(&p.lock_history(), None, &map, None, &station);
        assert_eq!(
            none.iter().map(kind).collect::<Vec<_>>(),
            ["hello", "snapshot"]
        );
        let score = held_score();
        let some = Session::new().on_connect(&p.lock_history(), Some(&score), &map, None, &station);
        assert_eq!(
            some.iter().map(kind).collect::<Vec<_>>(),
            ["hello", "snapshot", "score"]
        );
        assert_eq!(some[2], ServerMessage::score(&score));
    }

    #[test]
    fn a_score_change_emits_one_score() {
        let score = Arc::new(held_score());
        let msgs = Session::new().on_change(&HistoryChange::Score(Arc::clone(&score)));
        assert_eq!(msgs.iter().map(kind).collect::<Vec<_>>(), ["score"]);
    }

    #[test]
    fn lag_resends_the_score_after_the_snapshot() {
        let p = pipeline_with(5, 16);
        let score = held_score();
        let msgs = Session::new().on_lagged(&p.lock_history(), Some(&score));
        assert_eq!(
            msgs.iter().map(kind).collect::<Vec<_>>(),
            ["snapshot", "score"]
        );
    }

    #[tokio::test]
    async fn driver_forwards_a_score_published_after_connect() {
        let pipeline = Pipeline::new(0, 16);
        let gate = Arc::new(Notify::new());
        let open = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (sent_tx, mut sent_rx) = mpsc::unbounded_channel();
        let sink = GatedSink {
            gate,
            open,
            sent: sent_tx,
            incoming: VecDeque::new(),
        };
        let (_stop_tx, stop_rx) = watch::channel(false);
        let map = Config::default().map;
        let rx = pipeline.subscribe();
        let source = pipeline.clone();
        let driver = tokio::spawn(async move {
            drive(
                sink,
                rx,
                &source,
                &map,
                None,
                &HelloStation::default(),
                stop_rx,
            )
            .await;
        });
        let mut kinds = Vec::new();
        for _ in 0..2 {
            let m = tokio::time::timeout(std::time::Duration::from_secs(1), sent_rx.recv())
                .await
                .unwrap()
                .unwrap();
            kinds.push(kind(&m));
        }
        pipeline.publish_score(ContestScore {
            score: 90,
            qsos: 6,
            mults: 10,
        });
        let m = tokio::time::timeout(std::time::Duration::from_secs(1), sent_rx.recv())
            .await
            .unwrap()
            .unwrap();
        driver.abort();
        assert_eq!(kinds, ["hello", "snapshot"]);
        let ServerMessage::Score {
            score, qsos, mults, ..
        } = m
        else {
            panic!("expected a score, got {}", kind(&m));
        };
        assert_eq!((score, qsos, mults), (90, 6, 10));
    }

    // --- band filter (feature 011, T021) ----------------------------------------------------

    fn qso_on(i: i64, band: &str) -> EnrichedQso {
        let mut q = qso(i);
        q.record.set("BAND", band);
        q
    }

    fn snapshot_ids(msgs: &[ServerMessage]) -> Vec<String> {
        let [ServerMessage::Snapshot { qsos, .. }] = msgs else {
            panic!("expected one snapshot, got {msgs:?}");
        };
        qsos.iter().map(|q| q.id.clone()).collect()
    }

    fn mixed_pipeline() -> Pipeline {
        let p = Pipeline::new(0, 16);
        for i in 0..30 {
            p.publish(qso_on(i, if i % 3 == 0 { "40m" } else { "30m" }));
        }
        p
    }

    #[test]
    fn set_limit_with_bands_snapshots_only_those_bands() {
        let p = mixed_pipeline();
        let mut session = Session::new();
        let msgs = session.on_client(
            ClientMessage::SetLimit {
                limit: Some(10),
                bands: Some(vec!["40m".into()]),
            },
            &p.lock_history(),
        );
        let ids = snapshot_ids(&msgs);
        assert_eq!(
            ids.len(),
            10,
            "ten 40 m QSOs exist: last N counts only shown ones"
        );
        assert_eq!(ids.first().map(String::as_str), Some("Q0000"));
        assert_eq!(ids.last().map(String::as_str), Some("Q0027"));
    }

    #[test]
    fn unknown_bands_are_dropped_and_an_empty_list_means_every_band() {
        let p = mixed_pipeline();
        let mut session = Session::new();
        let msgs = session.on_client(
            ClientMessage::SetLimit {
                limit: Some(10),
                bands: Some(vec!["nope".into(), "40M".into()]),
            },
            &p.lock_history(),
        );
        assert_eq!(snapshot_ids(&msgs).len(), 10, "40M kept, nope dropped");
        let msgs = session.on_client(
            ClientMessage::SetLimit {
                limit: Some(10),
                bands: Some(vec!["nope".into()]),
            },
            &p.lock_history(),
        );
        let ids = snapshot_ids(&msgs);
        assert_eq!(
            ids.last().map(String::as_str),
            Some("Q0029"),
            "nothing valid left: every band"
        );
    }

    #[test]
    fn upserts_reach_the_page_whatever_its_bands() {
        let mut session = Session::new();
        let p = mixed_pipeline();
        session.on_client(
            ClientMessage::SetLimit {
                limit: Some(10),
                bands: Some(vec!["40m".into()]),
            },
            &p.lock_history(),
        );
        let change = HistoryChange::Upsert(Arc::new(qso_on(99, "30m")));
        assert_eq!(
            session.on_change(&change).len(),
            1,
            "the page filters what it draws"
        );
    }
}
