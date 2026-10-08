---
description: "Task list for 001-live-qso-map"
---

# Tasks: Live QSO Map

**Input**: Design documents from `/specs/001-live-qso-map/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Every test task comes before the implementation
tasks it covers, and its tests MUST be run and seen to fail before implementation begins. Rust tests
run with `cargo test`. Frontend tests run with `node --test web/`, using Node from nvm
(`~/.nvm/versions/node/v26.11.1/bin/node` when nvm is not loaded in the shell).

**Conventions used by every task**:

- **Errors**: the crate `Error` enum and `Result<T>` alias from `src/errors.rs`, per
  `docs/rust-guidelines.md` §1 (binding through constitution Principle II). No `unwrap`, `expect` or
  `panic!` in runtime paths.
- **Bad input**: log it with `tracing` and skip it. It must never end the process.
- **Unit tests** live in a `#[cfg(test)] mod tests` at the bottom of the file they test. Integration
  tests live in `tests/`.
- **Spec references**: FR-/SC- numbers refer to `spec.md`, R-numbers to `research.md`, and contracts
  to `contracts/*.md`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US5)
- **[R:TIER]**: risk tier. A task without one is GREEN.

---

## Phase 1: Setup (shared infrastructure)

**Purpose**: dependencies, crate layout and lint gates.

- [X] T001 Add runtime dependencies to `Cargo.toml` per plan.md "Dependency Justification". Use the latest compatible versions and turn off default features where possible:
  - `tokio` (rt-multi-thread, net, sync, time, macros, signal, fs)
  - `axum` (http1, ws, tokio)
  - `async-channel`
  - `reqwest` (rustls TLS only, `ring` crypto provider if available, per R17; no default features)
  - `quick-xml`
  - `clap` (derive, env)
  - `serde` (derive)
  - `toml`
  - `serde_json`
  - `tracing`
  - `tracing-subscriber` (fmt, env-filter)

  Add the dev-dependency `tokio-tungstenite`. Then run `cargo build` so `Cargo.lock` updates.

  Decide the rustls crypto provider now: build the crate with the chosen reqwest features for
  `armv7-unknown-linux-musleabihf` (`cross build --target armv7-unknown-linux-musleabihf`), and
  record the result in `research.md` R17.
- [X] T002 Create the crate layout from plan.md "Source Code":
  - `src/lib.rs` with `#![forbid(unsafe_code)]` and `pub mod` declarations for `errors`, `config`,
    `adif`, `band`, `mode`, `maidenhead`, `position`, `ingest`, `enrich`, `history`, `pipeline` and
    `web`;
  - an empty module file for each (`src/adif/mod.rs`, `src/ingest/mod.rs`, `src/enrich/mod.rs`,
    `src/enrich/provider/mod.rs` and `src/web/mod.rs` as directories);
  - `src/main.rs` with `#![forbid(unsafe_code)]`, calling a placeholder `live_qso_map::run()`
    (re-exported from `src/pipeline.rs`, see T025).

  `cargo build` must pass.
- [X] T003 [P] Add `[lints.rust] unsafe_code = "forbid"` and `[lints.clippy] unwrap_used = "deny"`, `expect_used = "deny"` and `panic = "deny"` to `Cargo.toml`. Allow those three in tests with `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` in `src/lib.rs`. Because `[lints]` also applies to integration-test crates, every `tests/*.rs` file must start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. Verify that `cargo clippy --all-targets -- -D warnings` passes.
- [X] T004 [P] Create the test scaffolding:
  - `tests/fixtures/README.md`, describing the fixture files to come;
  - `web/package.json` containing `{"type":"module","private":true}`, so that `node --test web/` treats
    the `.js` files as ES modules, with no dependencies;
  - an empty `src/web/assets/` directory.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: core types used by every story: error, configuration, ADIF record, band plan, locator,
position, identity, history, the WebSocket view type and the pipeline wiring.

**⚠️ CRITICAL**: no user story work starts until this phase is complete.

### Tests first (write, run, confirm they FAIL)

- [X] T005 [P] Write unit tests in `src/errors.rs` (`docs/rust-guidelines.md` §1.6):
  - `Display` is not empty for every variant;
  - `From<std::io::Error>`, `From<toml::de::Error>`, `From<quick_xml::Error>`,
    `From<reqwest::Error>`, `From<serde_json::Error>` and `From<tokio::task::JoinError>` produce the
    matching variants;
  - `Error` implements `std::error::Error`, and `source()` returns the wrapped error for every
    variant that wraps one (`docs/rust-guidelines.md` §1.6).
- [X] T006 [P] Write unit tests in `src/adif/parse.rs` for the R6 parser:
  - one record;
  - a header before `<EOH>`, which is skipped;
  - three records in one datagram give three records (FR-003);
  - field names are case-insensitive and stored uppercase;
  - the optional type indicator `<FREQ:5:N>` is accepted;
  - the length counts UTF-8 bytes, e.g. `<NAME:6>Jürgen`;
  - text before the first `<` is ignored;
  - trailing text without `<EOR>` containing `CALL` becomes a final record;
  - a malformed record (non-numeric length, length past the end of the input) is skipped and the
    records before and after it are still returned, with errors reported per record;
  - empty input gives zero records.
- [X] T007 [P] Write unit tests in `src/adif/write.rs`:
  - writing then parsing a record returns the same fields;
  - the output length uses bytes;
  - `format_lat(39.3125)` returns `N039 18.750`, and `format_lon(-122.08)` returns `W122 04.800`;
  - `parse_lat`/`parse_lon` reverse them, and reject out-of-range or malformed text.
- [X] T008 [P] Write unit tests in `src/band.rs`:
  - `band_from_mhz(7.0)` is `40m`, `14.2` is `20m`, `144.3` is `2m`, `432.1` is `70cm`, `50.1` is `6m`;
  - band edges are inclusive (`7.3` is `40m`);
  - `band_from_mhz(5.0)` is `None` (outside every ADIF band);
  - `band_from_metres("40")` is `40m` and `"2"` is `2m`;
  - `is_adif_band("20m")` is true and `"21m"` is false.
- [X] T009 [P] Write unit tests in `src/maidenhead.rs`:
  - `JM49NH` gives a centre of about (39.3125, 9.125);
  - `FN42` gives the centre (42.5, -71.0);
  - lowercase is accepted;
  - 8 characters (`JM49NH52`) are accepted with the 8-character centre;
  - lengths 2, 3, 5 and 10 are invalid;
  - characters out of range (`SS00`, `JM4X`) are invalid;
  - `to_locator6(39.3125, 9.125)` returns `JM49NH`;
  - `precision_of("JM49")` is `Locator4`, and `"JM49NH"` or `"JM49NH52"` is `Locator6`.
- [X] T010 [P] Write unit tests in `src/position.rs`:
  - `Precision` ordering is `Country < Locator4 < Locator6 < Coordinates`;
  - `Position::new` returns `Err(Error::InvalidPosition { .. })` for a latitude outside ±90 or a
    longitude outside ±180;
  - `as_str` and serde give `coordinates`, `locator6`, `locator4` and `country`;
  - `PositionSource` strings are `logger`, `qrz`, `hamqth`, `qrzcq`, `cty` and `config`.
- [X] T011 [P] Write unit tests in `src/ingest/identity.rs` (data-model "Identity"):
  - with a logger id, the canonical form is `L|IS0MZK|3809734F4`;
  - without one, the composite form is `C|IS0MZK|DL5QFN|40M|SSB|20261008|1504` (time truncated to
    `HHMM`);
  - components are trimmed and uppercased, so `" dl5qfn "` and `DL5QFN` are equal;
  - `station_call` falls back from `STATION_CALLSIGN` to the configured default, then to empty;
    a different `OPERATOR` value does not change the identity;
  - a missing `BAND` or `MODE` becomes an empty component.
- [X] T012 [P] Write unit tests in `src/history.rs` (data-model "History"):
  - `upsert` of a new id is `Accepted`;
  - the same id with a higher `rev` is `Accepted`, and the stored `sort_key` is unchanged even when
    TIME_ON changed (FR-028);
  - the same or a lower `rev` is `Stale`;
  - `latest(Some(n))` returns the last n in ascending `sort_key` order, and `latest(None)` returns all;
  - with capacity 3, a 4th insert evicts the oldest `sort_key`;
  - capacity 0 never evicts;
  - `len()` works.
- [X] T013 [P] Write unit tests in `src/config.rs` for precedence and validation (contracts/configuration.md):
  - the defaults match the table;
  - a TOML value is used when neither the CLI nor the environment sets it;
  - an environment value overrides TOML;
  - a CLI value overrides the environment;
  - `--band-color 20m=#112233` overrides only `20m`, and `LQM_BAND_COLORS` parses comma-separated
    entries;
  - validation errors for port 0, `workers=0`, an invalid locator, only one of latitude/longitude, a
    bad colour, an unknown band and an unknown provider name;
  - a provider with a username but no password is disabled with a warning;
  - `Debug` of the config never contains a password (redacted newtype).

  Use `Cli::try_parse_from` and an injected env map, not the process environment.
- [X] T014 [P] Write unit tests in `src/web/protocol.rs` (contracts/websocket-protocol.md):
  - `hello` serialises with `type`, `protocol=1`, `limits=[10,20,50,100,200,500,null]`,
    `default_limit=20`, `bands`, `fallback_color` and `tiles`;
  - `snapshot` and `upsert` match the contract shape;
  - `QsoView::from(&EnrichedQso)` fills `id`, `rev`, `sort_key` (RFC 3339, UTC), `call`, `band`,
    `mode`, `submode`, `datetime`, `station_call`, `remote`/`station` (or `null`) and `adif`;
  - `ClientMessage` parses `{"type":"set_limit","limit":100}` and `{"type":"set_limit","limit":null}`;
  - an unknown `type` gives `ClientMessage::Unknown` rather than an error.

### Implementation

- [X] T015 Implement `src/errors.rs` following `docs/rust-guidelines.md` §1 (no error-handling crates). Define an `enum Error`:
  - wrapping variants that keep the original error for `source()`: `Io(std::io::Error)`,
    `Toml(toml::de::Error)`, `Xml(quick_xml::Error)`, `Http(reqwest::Error)`,
    `Json(serde_json::Error)`, `Task(tokio::task::JoinError)`;
  - structured domain variants (§1.4): `Config { setting: String, message: String }`,
    `Adif { record: usize, message: String }`, `QarTest { element: &'static str, message: String }`,
    `Provider { provider: &'static str, message: String }`, `InvalidPosition { lat: f64, lon: f64 }`.

  Hand-write `impl Display`, `impl std::error::Error` with `source()`, and a `From` impl per wrapping variant. Declare `pub type Result<T> = std::result::Result<T, Error>;`. Every module imports `use crate::errors::{Error, Result};`. T005 must pass.
- [X] T016 Implement `AdifRecord` in `src/adif/mod.rs`: an ordered map from uppercase name to value, with `get`, `set_if_absent` (FR-017: never overwrite), `set`, `iter` and `fields_sorted`. Then implement `parse_datagram(&[u8]) -> (Vec<AdifRecord>, Vec<Error>)` in `src/adif/parse.rs`, recovering per record. T006 must pass.
- [X] T017 [P] Implement `src/adif/write.rs`: `to_adif_string(&AdifRecord)`, `format_lat`/`format_lon` (ADIF `XDDD MM.MMM`) and `parse_lat`/`parse_lon`. T007 must pass.
- [X] T018 [P] Implement `src/band.rs`: the static ADIF 3.1 band table (2190m to submm) with lower and upper limits in MHz, `band_from_mhz`, `band_from_metres` and `is_adif_band`. T008 must pass.
- [X] T019 [P] Implement `src/maidenhead.rs`: `centre(&str) -> Option<(f64, f64)>` for 4, 6 and 8 characters, `precision_of`, `to_locator6` and `is_valid` (R10). T009 must pass.
- [X] T020 [P] Implement `src/position.rs`: `Precision` (Ord, Serialize as lowercase), `PositionSource`, and `Position { lat, lon, precision, source }` with a validating constructor. Also define `LookupCandidate { source: PositionSource, position: Option<Position>, fields: AdifRecord }` here (data-model "LookupCandidate"), because both the Foundational pipeline (T025) and US2 use it. T010 must pass.
- [X] T021 [P] Implement `src/ingest/identity.rs`: `enum Identity { Logger{..}, Composite{..} }`, `Identity::from_record(&AdifRecord, logger_id: Option<&str>, default_station: Option<&str>)` and `canonical() -> String`. Add `pub mod identity;` to `src/ingest/mod.rs`. T011 must pass.
- [X] T022 Implement `EnrichedQso` and `History` in `src/history.rs`, with rev-checked `upsert`, a stable `sort_key`, eviction, `latest` and `len` (data-model "History"). The `sort_key` timestamp is parsed from `QSO_DATE` and `TIME_ON` into a UTC unix-seconds `i64` without extra crates. T012 must pass.
- [X] T023 Implement `src/config.rs`:
  - a clap `Cli` struct whose flags and env vars follow the contracts/configuration.md table, all
    `Option`;
  - a serde `FileConfig` struct;
  - `Config::load(cli, env_lookup, file_text) -> Result<Config>`, which merges CLI, then env, then
    file, then defaults, and validates, failing with `Error::Config { setting, message }`;
  - a `Secret` newtype with a redacted `Debug`;
  - the built-in band palette (`160m #8c564b`, `80m #9467bd`, `60m #bcbd22`, `40m #d62728`,
    `30m #17becf`, `20m #1f77b4`, `17m #ff7f0e`, `15m #2ca02c`, `12m #e377c2`, `10m #393b79`,
    `6m #637939`, `2m #843c39`, `70cm #7b4173`).

  T013 must pass.
- [X] T024 Implement `src/web/protocol.rs`: the `ServerMessage` enum (`Hello`, `Snapshot`, `Upsert`) and `ClientMessage` (`SetLimit`, `Unknown`), with serde `tag = "type"` and `rename_all = "snake_case"`, plus `QsoView` and `From<&EnrichedQso>`. T014 must pass.
- [X] T025 Implement the skeleton of `src/pipeline.rs`:
  - `pub struct Pipeline` holding `Arc<Mutex<History>>`, a `broadcast::Sender<HistoryChange>` (capacity
    `broadcast_queue`) and an `async_channel` for `QsoEvent` (capacity `ingest_queue`);
  - `HistoryChange::Upsert(Arc<EnrichedQso>)`;
  - `Pipeline::publish(EnrichedQso)`, which upserts into the history and, only when the upsert is
    `Accepted`, sends on the broadcast **while holding the lock** (R4);
  - `pub async fn start(config: Config, sources: LookupSources) -> Result<RunningService>` in
    `src/pipeline.rs`. It binds UDP and HTTP (port 0 is allowed), spawns every task, and returns
    `RunningService { udp_addr, http_addr, subscribe() -> broadcast::Receiver<HistoryChange>,
    history_len(), shutdown().await }`;
  - define in `src/pipeline.rs`, using std types only (no `futures` crate):
    `pub type LookupSource = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<Option<LookupCandidate>>> + Send>> + Send + Sync>;`
    and `#[doc(hidden)] pub type LookupSources = Option<Vec<LookupSource>>;`. `None` means "build
    the sources from config";
  - `pub async fn run(config) -> Result<()>` in `src/pipeline.rs` = `start(config, None)`, then wait
    for Ctrl+C/SIGTERM, then `shutdown()`. `src/lib.rs` re-exports both
    (`pub use pipeline::{run, start};`);
  - `src/main.rs`: parse the CLI, read the `--config` file, init `tracing-subscriber` with
    `log_level`, call `run`, and exit non-zero on error.

  Write these unit tests in `src/pipeline.rs` first: `publish` of a stale rev sends nothing; `start`
  with ports 0 returns non-zero addresses; `shutdown` completes within 1 s.
- [X] T026 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` over the crate (`Cargo.toml`, `src/`). All must pass before Phase 3.

**Checkpoint**: the foundation is ready and the user story phases can begin.

---

## Phase 3: User Story 1 - Logged QSO appears live on every open map (Priority: P1) 🎯 MVP

**Goal**: an ADIF QSO that already has its positions, sent over UDP, appears within 2 s on every open
page: a band-coloured marker, merged with any marker at the same point, with a great-circle line from
the station.

**Independent Test**: start the binary with `--station-locator JM49NH`, open two browsers on `/`, and
send the quickstart §2 datagram. Both show one 40m marker at JO62QM with a line from JM49NH.
`tests/adif_udp.rs` and `tests/ws_protocol.rs` (fan-out part) pass.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T027 [P] [US1] Write unit tests in `src/ingest/detect.rs` for the ADIF branch of R5: a datagram with `<CALL:4>` or `<EOR>` is `Adif`; `<EOH>` alone is `Adif`; random text, an empty datagram and `<ADX>` XML are `Unrecognised`; leading whitespace and a UTF-8 BOM are skipped.
- [X] T028 [P] [US1] Write unit tests in `src/enrich/station.rs` (FR-011):
  - record values win over config;
  - config fills `MY_GRIDSQUARE` when absent (`STATION_CALLSIGN` is filled at ingest, T036);
  - the station `Position` is resolved in this order: `MY_LAT`/`MY_LON` (source `logger`), then
    `MY_GRIDSQUARE` (`logger`), then config lat/lon (`config`), then config locator (`config`);
  - with none of these, it is `None`;
  - `APP_LIVEQSOMAP_STATION_POS_SOURCE` is set.
- [X] T029 [P] [US1] Write unit tests in `src/enrich/resolve.rs` for the logger-position rule (FR-012):
  - `LAT`/`LON` present gives a `Coordinates` position from source `logger`;
  - only `GRIDSQUARE` gives a locator precision from `logger`;
  - an invalid `GRIDSQUARE` (`ZZ99`) gives no logger position;
  - when a logger position exists, the function reports that no lookup is needed;
  - `APP_LIVEQSOMAP_POS_SOURCE` and `APP_LIVEQSOMAP_POS_PRECISION` are written (FR-018).
- [X] T030 [P] [US1] Write unit tests in `src/web/session.rs` for the session state machine:
  - on connect it emits `hello` then `snapshot(latest(20))`;
  - a `HistoryChange::Upsert` emits `upsert`;
  - an unknown client message is ignored.

  Drive the state machine with plain structs (no socket). Also test the async driver, which is
  generic over an in-crate `MessageSink` trait (`async fn send(&mut self, msg: ServerMessage) ->
  Result<()>`): with a test sink that never completes, `broadcast_queue=4` and 10 changes, the driver
  observes `Lagged`; once the sink is released it sends exactly one `snapshot`.
- [X] T031 [P] [US1] Write `web/geodesic.test.js` for `src/web/assets/geodesic.js`:
  - `greatCircle([39.2,9.2],[52.5,13.4],64)` returns 65 points, starting and ending at the inputs;
  - every intermediate point lies on the great circle (cross-track distance below 1 m);
  - a route crossing the antimeridian (Sardinia to Hawaii or New Zealand) has continuous, unwrapped
    longitudes with no jump above 180°;
  - identical endpoints return 2 points.
- [X] T032 [P] [US1] Write `web/map-state.test.js` for `src/web/assets/map-state.js` (data-model "Frontend state"):
  - `applyUpsert` adds a QSO;
  - QSOs with exactly equal `remote` lat/lon share one marker key (FR-024);
  - the marker colour comes from the band of the most recent QSO at that marker, using `bands`, with
    `fallback_color` for an unknown or missing band (FR-025);
  - a marker's z-order rank follows `sort_key`, newest highest (FR-028);
  - the line comes from the `station` of the marker's most recent QSO (FR-027), and there is no line
    when `station` is null;
  - a QSO with `remote: null` counts as visible but creates no marker (FR-019);
  - `diffLayers(prevMarkers, nextMarkers)` returns `{add, update, remove}` keyed by marker key: a new
    key is `add`; a changed colour, z-rank, popup QSO list or station is `update`; a missing key is
    `remove`; identical inputs give three empty lists.
- [X] T033 [P] [US1] Write integration test `tests/adif_udp.rs`:
  - start the service with `pipeline::start(test_config, None)` on port 0, with the station locator
    `JM49NH` and no providers or cty data;
  - subscribe with `RunningService::subscribe()`;
  - send the quickstart §2 datagram from a `UdpSocket`;
  - assert one `Upsert` within 2 s with `remote` precision `Locator6` at the JO62QM centre, source
    `logger`, and `station` from `config`;
  - send the quickstart §3 three-record datagram and assert exactly two more upserts; the malformed
    third record is skipped and the pipeline keeps running.
- [X] T034 [P] [US1] Write integration test `tests/ws_protocol.rs` (fan-out):
  - start the service with `pipeline::start(test_config, None)` on port 0;
  - connect two `tokio-tungstenite` clients to `/ws`;
  - assert each receives `hello` then `snapshot` (empty);
  - send one ADIF QSO over UDP and assert both receive the same `upsert` within 2 s (SC-001, FR-020);
  - assert that `GET /healthz` returns `{"status":"ok","history":1,...}` (contracts/http-endpoints.md).

### Implementation for User Story 1

- [X] T035 [US1] Implement `src/ingest/detect.rs`: `enum SourceFormat { Adif, QarTest, Unrecognised }` and `detect(&[u8]) -> SourceFormat`, with the ADIF and Unrecognised rules of R5. Leave the QARTest branch returning `Unrecognised` until US5. T027 must pass.
- [X] T036 [US1] Implement `QsoEvent` (data-model) and the UDP receive loop in `src/ingest/mod.rs`:
  - bind `bind_address:udp_port`;
  - `recv_from` into a 65,535-byte buffer;
  - `detect`; for `Adif`, `parse_datagram`, log each per-record error at warn, reject records without
    `CALL`/`QSO_DATE`/`TIME_ON` (warn), fill `STATION_CALLSIGN` from config when absent (the only place
    that does it, because the identity needs it), and compute the `Identity`;
  - send each `QsoEvent{kind: New}` on channel 1;
  - log `Unrecognised` at warn with the source address and length (FR-005).
- [X] T037 [US1] Implement `src/enrich/station.rs` (FR-011): `apply_station(&mut AdifRecord, &StationConfig) -> Option<Position>`, which handles only the station
  position fields. T028 must pass.
- [X] T038 [US1] Implement the logger-position part of `src/enrich/resolve.rs`: `logger_position(&AdifRecord) -> Option<Position>` and `write_position_fields(&mut AdifRecord, &Position)`, which fills `GRIDSQUARE` (when absent, from `to_locator6`), `LAT`/`LON` (when absent), `APP_LIVEQSOMAP_POS_SOURCE` and `APP_LIVEQSOMAP_POS_PRECISION`. T029 must pass.
- [X] T039 [US1] Implement the enrichment worker loop in `src/enrich/mod.rs`:
  - `spawn_workers(n, rx: async_channel::Receiver<QsoEvent>, pipeline)`;
  - per event: `apply_station`, then `logger_position`; a QSO without a position is still published
    with `remote: None`, and a warn is logged (FR-019);
  - build an `EnrichedQso` with `rev = 1` (the resend gate replaces this in US4);
  - call `Pipeline::publish`.

  Spawn the workers and the UDP loop from `pipeline::start`.
- [X] T040 [US1] Implement `src/web/session.rs` as a pure state machine: `Session::on_connect(&History) -> Vec<ServerMessage>` and `on_change(&HistoryChange) -> Vec<ServerMessage>`. Then implement the async driver in `src/web/mod.rs`: on upgrade, **subscribe to the broadcast first**, then take the snapshot under the history lock, then loop over `tokio::select!` of socket messages and broadcast receives. The driver sends through the `MessageSink` trait; the WebSocket adapter implements it. A send error ends the session quietly. T030 must pass.
- [X] T041 [US1] Implement the axum router in `src/web/mod.rs`:
  - `GET /`, `/app.js`, `/map-state.js`, `/geodesic.js` and `/app.css`, served from `include_str!`
    (`/connection.js` is added in T068, when the file exists)
    with the right content types;
  - `GET /ws`;
  - `GET /healthz` returning JSON with the history length, the active session count (an `AtomicUsize`)
    and the uptime;
  - a 404 fallback;
  - bind `bind_address:http_port`.

  Start it from `pipeline::start`.
- [X] T042 [P] [US1] Implement `src/web/assets/geodesic.js`: `export function greatCircle(a, b, segments)` using spherical linear interpolation, with longitudes unwrapped for continuity. T031 must pass.
- [X] T043 [US1] Implement `src/web/assets/map-state.js` as a pure ES module with no DOM or Leaflet:
  - `createState({bands, fallbackColor, limit})`;
  - `applySnapshot(state, qsos)`;
  - `applyUpsert(state, qso)`;
  - `visible(state)`, the last `limit` by `sort_key`;
  - `markers(state)`, returning a `Map` from `"lat,lon"` to `{qsos, color, zRank, station}`;
  - `diffLayers(prev, next)`, returning `{add, update, remove}`.

  T032 must pass.
- [X] T044 [US1] Implement the page:
  - `src/web/assets/index.html`: loads Leaflet 1.9.4 from
    `https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/leaflet.min.css` and `.../leaflet.min.js`,
    with SRI hashes taken from `https://api.cdnjs.com/libraries/leaflet/1.9.4?fields=sri`; a
    full-screen map `div`, a legend container, and `app.js` as a module;
  - `src/web/assets/app.css`: full-viewport map, legend box, popup table, coloured `divIcon` dot;
  - `src/web/assets/app.js`: glue only, with no decisions of its own. Build the `ws:`/`wss:` URL from
    `location`; on `hello`, create the map with `tiles.url`/`tiles.attribution` and the legend from
    `bands`; feed `hello`, `snapshot` and `upsert` into `map-state`, call `diffLayers`, and apply each
    `add`/`update`/`remove` to Leaflet (an `L.marker` with a coloured `divIcon` and
    `setZIndexOffset(zRank*1000)`, a popup listing call, band, mode, UTC time and position
    source/precision (FR-026), and an `L.polyline(greatCircle(...))` per marker). In US1, `app.js`
    opens a single WebSocket with no retry; US3 (T070) replaces this with `connection.js`.
- [ ] T045 [US1] Run the quickstart §2 and §3 manually in two browsers and confirm the Independent Test. Run `cargo test --test adif_udp --test ws_protocol` and `node --test web/`; all must pass.

**Checkpoint**: User Story 1 works on its own. This is the MVP.

---

## Phase 4: User Story 2 - Remote position resolved from lookup services (Priority: P2)

**Goal**: a QSO without a usable remote position is placed at the best precision available from QRZ.com,
HamQTH and QRZCQ (queried in parallel with a timeout) and the AD1C cty.dat country data.

**Independent Test**: configure the cty.dat fixture and fake providers in `tests/lookup_resolution.rs`.
A callsign-only QSO gets the most precise candidate. `IS0`, `IT9` and `I1` resolve to Sardinia, Sicily
and mainland Italy (SC-007). Quickstart §5 works with real credentials.

### Fixtures

- [ ] T046 [P] [US2] Create `tests/fixtures/cty-excerpt.dat` by downloading the current `https://www.country-files.com/cty/cty.dat` and copying exactly the entity blocks for Germany (DL), China (BY), United States (K), Switzerland (HB), Italy (I), Sardinia (IS), Sicily (*IT9, WAE-only), plus one entity that has `=EXACT` callsigns and per-alias `(cq)[itu]<lat/lon>` overrides. Record the source date in `tests/fixtures/README.md`.
- [ ] T047 [P] [US2] Create provider response fixtures from the structures documented in research.md R12. Replace any example personal data with fictitious values, and note in `tests/fixtures/README.md` that the files are reconstructed from documentation.
  - QRZ.com, in `tests/fixtures/qrz/`: `login_ok.xml` (`<Session><Key>`), `login_bad_password.xml`,
    `lookup_user.xml` (lat/lon with `geoloc=user`), `lookup_grid.xml` (`geoloc=grid`, 4-character grid),
    `lookup_dxcc.xml` (`geoloc=dxcc`), `not_found.xml`, `session_timeout.xml` (no `<Key>`).
  - HamQTH, in `tests/fixtures/hamqth/`: `login_ok.xml`, `lookup_full.xml` (namespace
    `https://www.hamqth.com`, latitude, longitude and grid), `lookup_grid_only.xml`, `not_found.xml`,
    `session_expired.xml`.
  - QRZCQ, in `tests/fixtures/qrzcq/`: `login_ok.xml`, `lookup_full.xml`, `premium_required.xml`,
    `not_found.xml`, `session_timeout.xml`.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T048 [P] [US2] Write unit tests in `src/enrich/cty.rs` (R13) using `tests/fixtures/cty-excerpt.dat`:
  - parsing gives the expected entity count;
  - longitudes are converted from +W to +E (Germany is about 10°E, not 10°W);
  - the longest prefix wins (`IS0ABC` is Sardinia, `IT9ABC` is Sicily, `I1ABC` is Italy) (SC-007);
  - an `=EXACT` callsign wins over its prefix;
  - per-alias `<lat/lon>`, `(cq)` and `[itu]` overrides apply;
  - `DL1ABC/P`, `/M`, `/QRP` and `/A` resolve like `DL1ABC`;
  - `/MM` and `/AM` give `None`;
  - `IS0/DL1ABC` resolves to Sardinia;
  - `K1ABC/6` keeps K;
  - an unknown callsign gives `None`;
  - malformed lines are skipped without error.

  The result `LookupCandidate` has precision `Country`, source `cty`, and fields `COUNTRY`, `CQZ`,
  `ITUZ` and `CONT`. (`DXCC` is not in cty.dat, so it is absent.)
- [ ] T049 [P] [US2] Write unit tests in `src/enrich/cty.rs` for refresh logic using a temp dir and an injected clock/fetcher. Assert the outcome for each case:
  - file missing, fetch succeeds: the file is written, then loaded;
  - file younger than `max_age_days`: no fetch;
  - file older than `max_age_days`, fetch fails: the old file is used and a warn is logged;
  - no file and the fetch fails: `None` and a warn;
  - `max_age_days = 0` and the file exists: no fetch.
- [ ] T050 [P] [US2] [R:YELLOW] Write unit tests in `src/enrich/provider/qrz.rs` using `tests/fixtures/qrz/`:
  - the login response gives the key;
  - a bad password gives `Err(Error::Provider { provider: "qrz", .. })`;
  - the `geoloc` mapping of R11: `user`/`geocode`/`zip` are `Coordinates`; `grid` takes its precision
    from the grid length; `state`/`dxcc` are `Country`; `none` gives no position;
  - the fields mapped are `GRIDSQUARE`, `LAT`, `LON`, `DXCC`, `COUNTRY` (from `land`), `CQZ`, `ITUZ`,
    `NAME` (`fname` + `name`), `QTH` (`addr2`), `STATE`, `CNTY` and `IOTA`;
  - not found gives `Ok(LookupOutcome::NotFound)`;
  - a missing `<Key>` gives `Ok(LookupOutcome::NeedsLogin)`;
  - unknown elements are ignored.
- [ ] T051 [P] [US2] [R:YELLOW] Write unit tests in `src/enrich/provider/hamqth.rs` using `tests/fixtures/hamqth/`:
  - the namespaced XML parses;
  - latitude/longitude give `Coordinates`;
  - grid only gives a locator precision;
  - `adif` gives `DXCC`, and `cq`/`itu`/`continent`/`country` give `CQZ`/`ITUZ`/`CONT`/`COUNTRY`;
  - "Callsign not found" gives `Ok(LookupOutcome::NotFound)`;
  - "Session does not exist or expired" gives `Ok(LookupOutcome::NeedsLogin)`.
- [ ] T052 [P] [US2] [R:YELLOW] Write unit tests in `src/enrich/provider/qrzcq.rs` using `tests/fixtures/qrzcq/`:
  - latitude/longitude give `Coordinates`;
  - `locator` gives a locator precision;
  - `dxcc`, `cq`, `itu`, `continent` and `country` are mapped;
  - "Not found: X" gives `Ok(LookupOutcome::NotFound)`;
  - "Session Timeout" gives `Ok(LookupOutcome::NeedsLogin)`;
  - "Premium Required" gives `Err(Error::Provider { provider: "qrzcq", .. })`.
- [ ] T053 [P] [US2] Write unit tests in `src/enrich/resolve.rs` for selection (R11). Inject candidate futures:
  - coordinates, locator6 and country together give coordinates (US2 scenario 1);
  - locator4 and country give locator4 (scenario 2);
  - a tie at the same precision is won by the first in `provider_order`, and `cty` loses ties;
  - a source that sleeps beyond the timeout is dropped, the result arrives within `timeout + 100 ms`
    (US2 scenario 4, SC-002), and a warn is logged;
  - a source returning `Err` is skipped;
  - non-position fields are filled from the winner first, then the others in order, never overwriting
    logger values (FR-017);
  - all sources empty gives `None`.
- [ ] T054 [P] [US2] Write integration test `tests/lookup_resolution.rs`:
  - start the service with `pipeline::start(config, Some(sources))`, using the cty fixture and two
    fake providers built as `pipeline::LookupSource` closures (T025). Production builds its sources
    from the `Provider` enum and the cty data;
  - send callsign-only ADIF QSOs and assert the broadcast `remote` precision and source for each
    scenario of US2;
  - assert that a QSO with `GRIDSQUARE` triggers no provider call (US2 scenario 5);
  - assert that `IS0XYZ`, `IT9XYZ` and `I1XYZ` land in Sardinia, Sicily and Italy (SC-007).

### Implementation for User Story 2

- [ ] T055 [US2] Implement `src/enrich/cty.rs` parsing and lookup: `CountryData::parse(&str)`, `lookup(call) -> Option<LookupCandidate>` (R13 rules), using `LookupCandidate` from `src/position.rs` (T020). T048 must pass.
- [ ] T056 [US2] Implement the cty.dat refresh in `src/enrich/cty.rs`: `load_or_refresh(path, url, max_age_days, fetch)`, using `reqwest` for the real fetcher and writing to a temp file followed by a rename. Call it from `pipeline::start` before the UDP listener starts, bounded by `timeout_secs × 4` (plan "Implementation Notes"). T049 must pass.
- [ ] T057 [US2] Implement `src/enrich/provider/mod.rs`:
  - `enum LookupOutcome { Found(LookupCandidate), NotFound, NeedsLogin }`, the return type of each
    provider's response parser as `Result<LookupOutcome>`. It is an outcome, not an error type, so it
    complies with `docs/rust-guidelines.md` §1.2;
  - `enum Provider { Qrz(QrzClient), HamQth(HamQthClient), QrzCq(QrzCqClient) }` with
    `async fn lookup(&self, call: &str) -> Result<Option<LookupCandidate>>`;
  - a session cache (`tokio::sync::Mutex<Option<Session>>`) with single-flight login;
  - on `NeedsLogin`, log in again once and retry;
  - a shared `reqwest::Client` with a timeout of `timeout_secs`;
  - a `name()` used for `provider_order` and `PositionSource`;
  - credential URLs are never logged.
- [ ] T058 [P] [US2] [R:YELLOW] Implement `src/enrich/provider/qrz.rs`: login `https://xmldata.qrz.com/xml/current/?username=;password=;agent=live-qso-map-<version>` (URL-encode the credentials), lookup `?s=KEY;callsign=C`, and parse with `quick-xml` into a `LookupCandidate` using the geoloc mapping. T050 must pass.
- [ ] T059 [P] [US2] [R:YELLOW] Implement `src/enrich/provider/hamqth.rs`: login `https://www.hamqth.com/xml.php?u=&p=`, lookup `?id=SID&callsign=C&prg=liveqsomap`, and namespace-aware parsing. T051 must pass.
- [ ] T060 [P] [US2] [R:YELLOW] Implement `src/enrich/provider/qrzcq.rs`: login `https://ssl.qrzcq.com/xml?username=&password=&agent=`, lookup `?s=KEY&callsign=C&agent=`, and parsing. T052 must pass.
- [ ] T061 [US2] Implement the selection in `src/enrich/resolve.rs`: `resolve_remote(call, sources: &[LookupSource], order, timeout) -> (Option<Position>, AdifRecord fields)`, running every source with `tokio::time::timeout` concurrently (`futures`-free: spawn each source and collect `JoinHandle`s), picking by precision then order, and merging fields. T053 must pass.
- [ ] T062 [US2] Wire `resolve_remote` into the worker in `src/enrich/mod.rs`: when `logger_position` is `None`, call it with the enabled providers (from config) plus cty, and write the fields with `set_if_absent` and the position fields. Build the providers and cty data in `pipeline::start`. T054 must pass.
- [ ] T063 [US2] Run quickstart §5 with real QRZ/HamQTH credentials (if available) and verify the popup source and precision. Record any differences from the fixtures in `tests/fixtures/README.md`.

**Checkpoint**: User Stories 1 and 2 both work on their own.

---

## Phase 5: User Story 3 - Viewer selects how many recent QSOs to show (Priority: P2)

**Goal**: each page has its own last-N selector (10, 20, 50, 100, 200, 500 or All; default 20). Pages
refill from the server history on open, on a change of N, on lag and after reconnecting.

**Independent Test**: `tests/ws_protocol.rs` (limit part). With 150 QSOs, a new page shows 20; at 100
it shows 100; at All it shows 150; a second page at 20 is unaffected. Quickstart §6.

### Tests for User Story 3 (write first, confirm they FAIL)

- [ ] T064 [P] [US3] Extend the unit tests in `src/web/session.rs`:
  - `set_limit(100)` emits `snapshot` with the last 100;
  - `set_limit(null)` emits all;
  - a limit not in `hello.limits` (e.g. 7) is ignored with a debug log and no message;
  - `on_lagged` emits `snapshot(latest(current_limit))`.
- [ ] T065 [P] [US3] Extend `web/map-state.test.js`:
  - with `limit=20` and 25 upserts, only the last 20 by `sort_key` are visible;
  - a marker is removed when none of its QSOs is visible (FR-030);
  - `setLimit(null)` makes everything visible;
  - an out-of-order upsert (an older `sort_key` arriving later) is placed by `sort_key`, not by
    arrival time.

  Also write `web/connection.test.js` for `src/web/assets/connection.js`:
  - `backoffDelay(attempt, rand)` gives 1, 2, 4, 8, 16, 30, 30 s (±20% jitter, controlled through an
    injected `rand`);
  - `onOpen` resets the attempt count;
  - `onHello(state)` returns `[{"type":"set_limit","limit":<current>}]`;
  - `loadLimit(storage)` returns 20 when the storage is empty, throws, or holds an invalid value,
    and otherwise the stored value;
  - `saveLimit(storage, v)` swallows storage errors.
- [ ] T066 [P] [US3] Extend `tests/ws_protocol.rs` (service started with `pipeline::start` on port 0):
  - insert 150 QSOs, then connect: the snapshot has 20, in ascending order (SC-003, within 3 s);
  - send `set_limit 100` and get a snapshot of 100;
  - `set_limit null` gives 150;
  - a second client still gets only upserts and no extra snapshot;
  - with `history_size=1000`, after 1001 inserts `/healthz` reports 1000 (US3 scenario 4).

  Lag recovery is tested deterministically in T030/T064, not here.

### Implementation for User Story 3

- [ ] T067 [US3] Implement `set_limit` and lag handling in `src/web/session.rs`, and in the `MessageSink`-based driver in `src/web/mod.rs` handle `broadcast::error::RecvError::Lagged` by sending the snapshot. Validate the limit against the allowed set. T064 must pass.
- [ ] T068 [US3] Implement `setLimit` and visible-window recomputation in `src/web/assets/map-state.js`, and implement `src/web/assets/connection.js` (`backoffDelay`, `onOpen`, `onHello`, `loadLimit`, `saveLimit`). Add the `GET /connection.js` route (`text/javascript`, `include_str!`) to the router in `src/web/mod.rs`. T065 must pass.
- [ ] T069 [US3] Add the selector to the page:
  - `index.html`/`app.css`: a `<select>` with 10, 20, 50, 100, 200, 500 and All (default 20) in a
    corner control;
  - `app.js`: on change, send `{"type":"set_limit","limit":...}` and persist the choice with
    `connection.saveLimit(sessionStorage, v)`; initialise the selector with
    `connection.loadLimit(sessionStorage)`. DOM wiring only.
- [ ] T070 [US3] Wire reconnection in `src/web/assets/app.js` (FR-031) using `connection.js`: on close, schedule a retry after `backoffDelay(attempt, Math.random)`; on open, call `onOpen`; on `hello`, send the messages returned by `onHello`; show a small "reconnecting…" indicator while disconnected. DOM wiring only. T066 must pass, and quickstart §6 must be checked manually.

**Checkpoint**: User Stories 1, 2 and 3 work on their own.

---

## Phase 6: User Story 4 - Corrections and resends are handled without duplicates (Priority: P3)

**Goal**: identical resends are dropped before enrichment, so they cost no lookups. Corrections update
the existing entry everywhere, in place. A newer revision always wins, even with several workers.

**Independent Test**: `tests/resend_gate.rs`. Sending the same ADIF QSO twice gives one upsert; a
modified resend gives `rev=2`, an in-place update and an unchanged count. Same remote call and band but
a different mode gives two QSOs.

### Tests for User Story 4 (write first, confirm they FAIL)

- [ ] T071 [P] [US4] Write unit tests in `src/ingest/gate.rs` (data-model "ResendGate"):
  - an unknown identity gives `Forward{rev:1}`;
  - the same identity and fingerprint gives `Drop` (FR-008);
  - the same identity with a different fingerprint gives `Forward{rev:2}` (FR-009);
  - a fingerprint ignores field order and is computed before enrichment;
  - a correction without a logger id whose old composite identity (`oldcall` + `oldtimestamp`) is known
    is re-keyed to the new identity with `rev + 1`;
  - a correction with an unknown identity gives `Forward{rev:1}` (edge case);
  - with capacity 2 and 3 identities, the oldest is evicted, so a later resend of it is new;
  - with capacity 0, nothing is evicted;
  - two QSOs with the same call and band but different `MODE` are two identities (US4 scenario 4).
- [ ] T072 [P] [US4] Extend `web/map-state.test.js`:
  - an upsert with a lower or equal `rev` for a known id is ignored;
  - a higher `rev` replaces the QSO in place: the visible count is unchanged and `sort_key` is
    unchanged (FR-028);
  - when the corrected QSO's `remote` changes, it moves from the old marker to the new one, and an
    empty old marker is removed.
- [ ] T073 [P] [US4] Write integration test `tests/resend_gate.rs`:
  - start the service with `pipeline::start(config, Some(sources))` and a counting fake provider;
  - send a callsign-only QSO twice: one upsert and **one** provider call (the dedupe point of R3);
  - send it again with a changed `SRX_STRING`: an upsert with `rev=2` and the history length is still
    1 (SC-005);
  - with `workers=4` and a fake provider that delays `rev=1` by 300 ms but answers `rev=2` immediately,
    the final history entry and the last broadcast have `rev=2`; the stale `rev=1` is not broadcast
    (R3/R4);
  - send two *different* QSOs (different TIME_ON) for the same callsign: the counting provider is
    called twice, because lookups are not cached (FR-016).

### Implementation for User Story 4

- [ ] T074 [US4] Implement `src/ingest/gate.rs`: `ResendGate { entries, order, capacity }`, `fingerprint(&AdifRecord) -> u64` (std `DefaultHasher` over the sorted fields) and `check(&QsoEvent) -> GateDecision`, with the rules in the data-model table. T071 must pass.
- [ ] T075 [US4] Insert the gate into the UDP loop in `src/ingest/mod.rs`, between identity and channel 1: dropped events are logged at debug, and forwarded events carry their `rev`. Change `src/enrich/mod.rs` to use the event's `rev` instead of the constant `1`. Size the gate to `history_size`. T073 must pass.
- [ ] T076 [US4] Implement the revision rule and in-place replacement in `src/web/assets/map-state.js` `applyUpsert`. T072 must pass.

**Checkpoint**: User Stories 1–4 work on their own.

---

## Phase 7: User Story 5 - QSOs from QARTest are shown on the map (Priority: P3)

**Goal**: QARTest XML on the same UDP port is detected, converted to ADIF following FR-004a, and
processed like any other QSO. Score and delete messages are ignored at debug level.

**Independent Test**: `tests/qartest_replay.rs` reproduces SC-009 from
`specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt`. The full replay depends on US2 (cty
fixture, so no "position not found" warnings) and US4 (corrections). The conversion and detection unit
tests depend only on the Foundational phase.

### Tests for User Story 5 (write first, confirm they FAIL)

- [ ] T077 [P] [US5] Write unit tests in `src/mode.rs` (R8):
  - `LSB` gives `SSB` with `LSB`, and `USB` gives `SSB` with `USB`;
  - `CW`, `RTTY`, `FM`, `AM` and `FT8` pass through;
  - `FT4` gives `MFSK` with `FT4`;
  - `PSK31` gives `PSK` with `PSK31`;
  - lowercase input is accepted;
  - an unknown `XYZ` gives `XYZ` with no submode.
- [ ] T078 [P] [US5] Extend the unit tests in `src/ingest/detect.rs` for the QARTest branch of R5:
  - each of the 10 reference packets is detected as `QarTest`;
  - `<contactinfo>` XML whose `<logger>` is `N1MM Logger+` is `Unrecognised`;
  - `<ADX>` is `Unrecognised`;
  - malformed XML is `Unrecognised`.
- [ ] T079 [P] [US5] Write unit tests in `src/ingest/qartest.rs`, loading packets from the reference capture with a small helper that splits on the `BEGIN PACKET`/`END PACKET` markers. Assert:
  - **packet #1 (DL5QFN)** converts exactly to the contracts/udp-input.md expected record:
    `CALL`, `STATION_CALLSIGN=IS0MZK`, `QSO_DATE=20261008`, `TIME_ON=150404`, `FREQ=7.000000`,
    `BAND=40m`, `MODE=SSB`, `SUBMODE=LSB`, `RST_SENT=59`, `RST_RCVD=59`, `SRX_STRING=14`, `PFX=DL5`,
    `CONTEST_ID=CQ-WW-SSB`, `PROGRAMID=QARTest`, `PROGRAMVERSION=16.9.1`,
    `APP_LIVEQSOMAP_LOGGER_ID=3807B771C`, `APP_LIVEQSOMAP_QARTEST_*`; `OPERATOR` and `STATIONNAME`
    are absent;
  - **packets #6–#9** are `Correction` with `logger_id` set and `previous` set from
    `oldcall`/`oldtimestamp`;
  - **packets #4 and #10** (`dynamicresults`) give no event, logged at debug;
  - **a synthetic `contactdelete`** gives no event, logged at debug (FR-005a);
  - **`txfreq` 0 or missing** takes `BAND` from `<band>40</band>`, giving `40m`;
  - **all three `exch1/2/3` set** are joined with single spaces;
  - **XML entities** such as `&amp;` are decoded;
  - **a missing `call` or `timestamp`** gives an error and the datagram is skipped.
- [ ] T080 [P] [US5] Write integration test `tests/qartest_replay.rs` (SC-009):
  - start the service with `pipeline::start` on port 0, with `tests/fixtures/cty-excerpt.dat` and no
    online providers;
  - capture warnings with a counting `tracing` layer installed through
    `tracing::subscriber::set_global_default` at the start of the single `#[tokio::test]` in this
    file (one test per binary, so the global install is safe). It counts events at WARN or above
    from any thread;
  - replay the 10 reference packets over UDP in order;
  - assert exactly 5 history entries (DL5QFN, B5Q, K6QA, K2WQX, HB9FQZ);
  - K6QA has `rev=3` and `SRX_STRING=3`;
  - HB9FQZ has `rev=2` and `SRX_STRING=14`;
  - all five have `remote` precision `Country` from `cty` (DL, BY, K, K, HB);
  - zero warn+ events were captured.

### Implementation for User Story 5

- [ ] T081 [P] [US5] Implement `src/mode.rs`: `map_mode(&str) -> (String, Option<String>)`. T077 must pass.
- [ ] T082 [US5] Implement the QARTest branch of `detect` in `src/ingest/detect.rs` using `quick-xml`: read the root element name and the `logger` or `soft` text without building a DOM. T078 must pass.
- [ ] T083 [US5] Implement `src/ingest/qartest.rs`: `parse(&[u8]) -> Result<Option<QsoEvent>>` (`None` for score and delete), reading events into a flat map and applying the FR-004a table using `band`, `mode` and `adif` helpers. Add `pub mod qartest;` to `src/ingest/mod.rs`. T079 must pass.
- [ ] T084 [US5] Route `SourceFormat::QarTest` in the UDP loop in `src/ingest/mod.rs` through `qartest::parse`, then the identity (logger id), then the gate. A `Correction` without a logger id passes `previous` to the gate. T080 must pass.

**Checkpoint**: all user stories work on their own.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T085 [P] Add the malformed-input corpus `tests/fixtures/malformed/` (truncated ADIF, length overflow, negative or huge lengths, binary noise, empty datagram, half XML, 64 KB of `<`, invalid UTF-8) and the integration test `tests/malformed_input.rs`. It sends every file plus a valid QSO in the same and in separate datagrams, against a service started with `pipeline::start` on port 0, and asserts that the service is still alive (`/healthz` 200) and that every valid record was delivered (SC-006).
- [ ] T086 [P] Add a property-style test loop in `src/adif/parse.rs`: 10,000 pseudo-random byte strings from a seeded xorshift (no extra crate) must never panic.
- [ ] T087 [P] Write `tests/load_ws.rs` as an `#[ignore]` test (SC-004): 50 WebSocket clients and 10 QSO/s for 60 s; assert every client received 600 upserts. Also record each QSO's send-to-receive latency, and assert that the 95th percentile is at most 2 s (SC-001). Document how to run it on a Raspberry Pi in `quickstart.md` §7.
- [ ] T088 [P] Add a `scripts/send_burst.sh N` developer helper that sends N synthetic ADIF QSOs with spread grid squares and increasing times via `nc -u` (quickstart §6).
- [ ] T089 [P] Create `deploy/config.example.toml`, mirroring contracts/configuration.md with every key commented, and `deploy/live-qso-map.service` (systemd: `DynamicUser=yes`, `StateDirectory=live-qso-map`, `LQM_CTY_PATH=/var/lib/live-qso-map/cty.dat`, `--config /etc/live-qso-map/config.toml`, `Restart=on-failure`, hardening options `NoNewPrivileges`, `ProtectSystem=strict` and `ProtectHome`).
- [ ] T090 [P] [R:YELLOW] Create a multi-stage `Dockerfile`: a builder stage on `rust:1.98` with the musl target, building a release `--target x86_64-unknown-linux-musl`, then a runtime stage on a minimal non-root image with CA certificates for the rustls lookups. Add `EXPOSE 12060/udp 8080/tcp`, `VOLUME /data`, `ENV LQM_CTY_PATH=/data/cty.dat` and `ENTRYPOINT ["/live-qso-map"]`. Add a `.dockerignore` excluding `target/`, `.git/` and `specs/`.
- [ ] T091 [P] Create `Cross.toml` for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` and `armv7-unknown-linux-musleabihf`, and verify that all three still build with `cross build --release --target <t>`, using the crypto provider chosen in T001 (R17).
- [ ] T092 [P] Write `README.md`: what the service does, quick start, configuration table (link to the contract), supported inputs (ADIF UDP, QARTest), provider notes (quotas; no lookup caching by design, FR-016), the open-access warning (FR-035) and deployment options.
- [ ] T093 Verify that secrets never reach the logs: run with `--log-level trace` and fake credentials against the fixtures and assert that no password or session key appears in the output. Add an assertion test in `src/enrich/provider/mod.rs` that `Debug` of every provider config is redacted.
- [ ] T094 Run the complete quality gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`. Then walk through quickstart §1–§8 end to end and tick off SC-001 to SC-009 in a short verification note at `specs/001-live-qso-map/checklists/verification.md`.

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup and blocks every story.
- **US1 (Phase 3, P1)**: depends on Foundational. This is the MVP.
- **US2 (Phase 4, P2)**: depends on Foundational. It extends the US1 worker in `src/enrich/mod.rs`, so
  run it after US1, or merge carefully on that one file.
- **US3 (Phase 5, P2)**: depends on Foundational and on the US1 session and page files
  (`src/web/session.rs`, `map-state.js`, `app.js`).
- **US4 (Phase 6, P3)**: depends on Foundational and on the US1 ingest loop (`src/ingest/mod.rs`) and
  worker.
- **US5 (Phase 7, P3)**:
  - T077–T079 and T081–T083 depend only on Foundational;
  - T084 depends on US1 (the ingest loop) and US4 (the gate handles corrections);
  - T080 depends on US2 (cty fixture) and US4.
- **Polish (Phase 8)**: after the desired stories are done. T085 needs US1; T093 needs US2.

### Story completion order

```text
Setup → Foundational → US1 (MVP) → US2 ─┐
                               ├→ US3   ├→ US5 (T080/T084) → Polish
                               └→ US4 ──┘
```

### Within each story

Tests come first and must fail, then the pure logic modules, then the async wiring, then the page. Each
`Txxx must pass` step closes its test task.

---

## Parallel Opportunities

- **Setup**: T003 and T004 run alongside T002 once T001 is done.
- **Foundational tests**: T005–T014 touch different files, so all run in parallel.
- **Foundational implementation**: T017–T021 run in parallel after T016. T022, T023 and T024 depend
  on T015 and T020.
- **US1**: T027–T034 run in parallel. T042 runs alongside T040 and T041.
- **US2**: T046–T054 run in parallel. T058, T059 and T060 run in parallel after T057.
- **US3**: T064–T066 run in parallel.
- **US4**: T071–T073 run in parallel.
- **US5**: T077–T080 run in parallel, and T081 alongside T082.
- **Polish**: T085–T092 run in parallel.
- **Across stories**: once US1 is done, US2, US3 and US4 can go to different people. They touch
  different modules except the shared worker file (`src/enrich/mod.rs`, US2 and US4) and the ingest
  loop (`src/ingest/mod.rs`, US4 and US5).

### Parallel example: User Story 2

```text
Together:  T046 cty fixture · T047 provider fixtures
Together:  T048 cty tests · T049 refresh tests · T050 qrz tests · T051 hamqth tests · T052 qrzcq tests · T053 resolve tests · T054 integration test
Then:      T055 cty → T056 refresh;  T057 provider core → (T058 ∥ T059 ∥ T060);  T061 resolve
Finally:   T062 wiring → T063 manual check
```

### Parallel example: User Story 5

```text
Together:  T077 mode tests · T078 detect tests · T079 qartest tests · T080 replay test
Together:  T081 mode · T082 detect
Then:      T083 qartest → T084 routing (needs US4) → T080 green (needs US2)
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1 Setup, then Phase 2 Foundational (all gates green).
2. Phase 3 US1: ADIF over UDP with positions in the record, shown live on every page.
3. **Stop and validate**: the US1 Independent Test and quickstart §2–§3. Demo-able with any logger
   that sends ADIF with locators.

### Incremental delivery

1. US1 is the MVP: a live map from ADIF with locators.
2. US2 adds placement of locator-less contest QSOs (lookups and cty.dat).
3. US3 adds late joiners, the history and the N selector.
4. US4 makes it robust to resends and edits.
5. US5 adds QARTest support. The SC-009 replay is the acceptance gate.
6. Polish covers deployment artifacts, robustness corpus, load test and docs.

Each story ends at a checkpoint with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` and `node --test web/` all green before the next starts.

---

## Notes

- **[P]** means different files with no dependency on an unfinished task.
- **[R:YELLOW]** marks the code that handles credentials, outbound HTTP to third parties and the
  container image. Review it with secrets and supply chain in mind.
- Verify that each test fails before implementing it (constitution I).
- Commit after each task or logical group, with no co-author trailers (project preference).
- Every new dependency beyond plan.md's list needs a justification in plan.md first (constitution III).
