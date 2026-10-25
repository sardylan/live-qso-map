//! HTTP server: map page, assets, WebSocket sessions and health check
//! (contracts/http-endpoints.md).

pub mod protocol;
pub mod session;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::band_filter::BandSet;
use crate::config::MapConfig;
use crate::errors::{Error, Result};
use crate::pipeline::Pipeline;
use crate::web::protocol::{ClientMessage, HelloStation, ServerMessage};
use crate::web::session::{MessageSink, drive};

const INDEX_HTML: &str = include_str!("assets/index.html");
/// The page with the release that serves it, so a stale copy can tell (feature 018, R5).
static INDEX_PAGE: LazyLock<String> =
    LazyLock::new(|| INDEX_HTML.replace("__LQM_VERSION__", env!("CARGO_PKG_VERSION")));
const APP_JS: &str = include_str!("assets/app.js");
const MAP_STATE_JS: &str = include_str!("assets/map-state.js");
const GEODESIC_JS: &str = include_str!("assets/geodesic.js");
const APP_CSS: &str = include_str!("assets/app.css");
const VIEW_JS: &str = include_str!("assets/view.js");
const LAYERS_JS: &str = include_str!("assets/layers.js");
const CONNECTION_JS: &str = include_str!("assets/connection.js");
const QTH_JS: &str = include_str!("assets/qth.js");
const AUTOZOOM_JS: &str = include_str!("assets/autozoom.js");
const AUTOZOOM_GEO_JS: &str = include_str!("assets/autozoom-geo.js");
const RECENT_JS: &str = include_str!("assets/recent.js");
const SCORE_JS: &str = include_str!("assets/score.js");
const STATION_JS: &str = include_str!("assets/station.js");
const THEMES_JS: &str = include_str!("assets/themes.js");
const SUN_JS: &str = include_str!("assets/sun.js");
const PANEL_JS: &str = include_str!("assets/panel.js");
const LEGEND_JS: &str = include_str!("assets/legend.js");
const GRAYLINE_JS: &str = include_str!("assets/grayline.js");

#[derive(Clone)]
struct AppState {
    pipeline: Pipeline,
    map: Arc<MapConfig>,
    band_filter: Arc<Option<BandSet>>,
    station: Arc<HelloStation>,
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
    station: HelloStation,
    shutdown: watch::Receiver<bool>,
) {
    let state = AppState {
        pipeline,
        map: Arc::new(map),
        band_filter: Arc::new(band_filter),
        station: Arc::new(station),
        sessions: Arc::new(AtomicUsize::new(0)),
        started: Instant::now(),
        shutdown: shutdown.clone(),
    };
    let app = Router::new()
        .route(
            "/",
            get(|| asset("text/html; charset=utf-8", INDEX_PAGE.as_str())),
        )
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
        .route("/qth.js", get(|| asset("text/javascript", QTH_JS)))
        .route(
            "/autozoom.js",
            get(|| asset("text/javascript", AUTOZOOM_JS)),
        )
        .route(
            "/autozoom-geo.js",
            get(|| asset("text/javascript", AUTOZOOM_GEO_JS)),
        )
        .route("/recent.js", get(|| asset("text/javascript", RECENT_JS)))
        .route("/score.js", get(|| asset("text/javascript", SCORE_JS)))
        .route("/station.js", get(|| asset("text/javascript", STATION_JS)))
        .route("/themes.js", get(|| asset("text/javascript", THEMES_JS)))
        .route("/sun.js", get(|| asset("text/javascript", SUN_JS)))
        .route("/panel.js", get(|| asset("text/javascript", PANEL_JS)))
        .route("/legend.js", get(|| asset("text/javascript", LEGEND_JS)))
        .route(
            "/grayline.js",
            get(|| asset("text/javascript", GRAYLINE_JS)),
        )
        .route("/ws", get(ws_handler))
        .route("/healthz", get(healthz))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
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

/// Page files are embedded, so they change only with the binary: `no-cache` makes browsers
/// revalidate them, and a rebuilt service never runs with stale page modules.
async fn asset(content_type: &'static str, body: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
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

/// An error response with the JSON body every error must have (`docs/http-api.md` API-3).
/// `message` is for people; it never carries configuration values or file paths.
pub fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = serde_json::json!({ "error": code, "message": message });
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

async fn not_found(uri: Uri) -> Response {
    json_error(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("no such endpoint: {}", uri.path()),
    )
}

async fn method_not_allowed(method: Method, uri: Uri) -> Response {
    json_error(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        &format!("method {method} not allowed on {}", uri.path()),
    )
}

async fn ws_handler(
    ws: std::result::Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    State(state): State<AppState>,
) -> Response {
    match ws {
        Ok(ws) => ws.on_upgrade(move |socket| session_task(socket, state)),
        Err(rejection) => json_error(
            rejection.status(),
            "websocket_upgrade_required",
            &rejection.body_text(),
        ),
    }
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
        &state.station,
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
