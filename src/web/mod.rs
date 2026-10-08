//! HTTP server: map page, assets, WebSocket sessions and health check
//! (contracts/http-endpoints.md).

pub mod protocol;
pub mod session;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::band_filter::BandSet;
use crate::config::MapConfig;
use crate::errors::{Error, Result};
use crate::pipeline::Pipeline;
use crate::web::protocol::{ClientMessage, ServerMessage};
use crate::web::session::{MessageSink, drive};

const INDEX_HTML: &str = include_str!("assets/index.html");
const APP_JS: &str = include_str!("assets/app.js");
const MAP_STATE_JS: &str = include_str!("assets/map-state.js");
const GEODESIC_JS: &str = include_str!("assets/geodesic.js");
const APP_CSS: &str = include_str!("assets/app.css");
const VIEW_JS: &str = include_str!("assets/view.js");
const LAYERS_JS: &str = include_str!("assets/layers.js");
const CONNECTION_JS: &str = include_str!("assets/connection.js");

#[derive(Clone)]
struct AppState {
    pipeline: Pipeline,
    map: Arc<MapConfig>,
    band_filter: Arc<Option<BandSet>>,
    sessions: Arc<AtomicUsize>,
    started: Instant,
    shutdown: watch::Receiver<bool>,
}

/// Serves HTTP until shutdown is signalled.
pub async fn serve(
    listener: TcpListener,
    pipeline: Pipeline,
    map: MapConfig,
    band_filter: Option<BandSet>,
    shutdown: watch::Receiver<bool>,
) {
    let state = AppState {
        pipeline,
        map: Arc::new(map),
        band_filter: Arc::new(band_filter),
        sessions: Arc::new(AtomicUsize::new(0)),
        started: Instant::now(),
        shutdown: shutdown.clone(),
    };
    let app = Router::new()
        .route("/", get(|| asset("text/html; charset=utf-8", INDEX_HTML)))
        .route("/app.js", get(|| asset("text/javascript", APP_JS)))
        .route(
            "/map-state.js",
            get(|| asset("text/javascript", MAP_STATE_JS)),
        )
        .route(
            "/geodesic.js",
            get(|| asset("text/javascript", GEODESIC_JS)),
        )
        .route("/app.css", get(|| asset("text/css", APP_CSS)))
        .route("/view.js", get(|| asset("text/javascript", VIEW_JS)))
        .route("/layers.js", get(|| asset("text/javascript", LAYERS_JS)))
        .route(
            "/connection.js",
            get(|| asset("text/javascript", CONNECTION_JS)),
        )
        .route("/ws", get(ws_handler))
        .route("/healthz", get(healthz))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(state);

    let mut stop = shutdown;
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stop| *stop).await;
        })
        .await;
    if let Err(e) = result {
        tracing::error!("http server stopped: {e}");
    }
}

async fn asset(content_type: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

async fn healthz(State(state): State<AppState>) -> Response {
    let body = serde_json::json!({
        "status": "ok",
        "history": state.pipeline.history_len(),
        "sessions": state.sessions.load(Ordering::SeqCst),
        "uptime_s": state.started.elapsed().as_secs(),
    });
    (
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| session_task(socket, state))
}

async fn session_task(socket: WebSocket, state: AppState) {
    // Subscribe before the snapshot is taken (R4).
    let changes = state.pipeline.subscribe();
    state.sessions.fetch_add(1, Ordering::SeqCst);
    tracing::debug!("page connected");
    drive(
        WsSink { socket },
        changes,
        &state.pipeline,
        &state.map,
        state.band_filter.as_ref().as_ref(),
        state.shutdown.clone(),
    )
    .await;
    state.sessions.fetch_sub(1, Ordering::SeqCst);
    tracing::debug!("page disconnected");
}

struct WsSink {
    socket: WebSocket,
}

impl MessageSink for WsSink {
    async fn send(&mut self, msg: ServerMessage) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        self.socket
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))
    }

    async fn recv(&mut self) -> Option<ClientMessage> {
        loop {
            match self.socket.recv().await? {
                Ok(Message::Text(text)) => return Some(ClientMessage::parse(text.as_str())),
                Ok(Message::Close(_)) | Err(_) => return None,
                Ok(_) => continue,
            }
        }
    }
}
