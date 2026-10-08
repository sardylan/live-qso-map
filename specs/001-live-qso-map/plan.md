# Implementation Plan: Live QSO Map

**Branch**: `001-live-qso-map` | **Date**: 2026-10-08 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-live-qso-map/spec.md`

## Summary

A single Rust binary with these stages:

1. It receives QSO datagrams on one UDP port and detects plain ADIF or QARTest XML from the content.
2. It normalises every QSO to an ADIF record.
3. A resend gate drops identical resends **before** enrichment, so they cost no lookups.
4. Enrichment workers fill the station information, then resolve the remote position. They use the
   logger's position when present. Otherwise they query QRZ.com, HamQTH and QRZCQ in parallel, plus the
   AD1C cty.dat country data, and keep the most precise result.
5. The enriched QSO is inserted into an in-memory history and broadcast to every connected map page
   over a WebSocket.

The page is plain JS with Leaflet from a CDN. It shows the last N QSOs (chosen per page) as
band-coloured markers, merged by position and stacked by QSO time, with great-circle lines from the
station.

```text
UDP ──► ingest (detect → parse → normalise → station defaults → identity → resend gate)
         │  async-channel (MPMC, bounded)              ← channel 1
         ▼
      enrichment workers ×W (default 1): remote position (logger | providers ∥ cty) → ADIF fields
         │  History upsert (rev-checked), under lock
         │  tokio::broadcast (MPMC fan-out)            ← channel 2
         ▼
      WebSocket session per page: hello → snapshot(N) → upserts; set_limit → snapshot; lagged → snapshot
```

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98)
**Primary Dependencies**: tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber
**Storage**: in-memory history (bounded, lost on restart); cty.dat file cached on disk
**Testing**: `cargo test` (unit, integration, replay of the reference capture); `node --test` for the frontend state module (dev only, no npm packages)
**Target Platform**: Linux x86_64, aarch64 and armv7 (static musl), in a container or under systemd; macOS for development
**Project Type**: single binary (backend plus embedded static frontend)
**Performance Goals**: marker on every page within 2 s for QSOs that already have a position (SC-001); within the lookup timeout plus 2 s otherwise (SC-002); 10 QSO/s with 50 pages on a Raspberry Pi 4 (SC-004)
**Constraints**: only one UDP and one TCP port; no lookup caching (FR-016); history in RAM only; open access (FR-035)
**Scale/Scope**: history of 10,000 QSOs by default; tens of concurrent viewers; 3 online providers plus cty.dat

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - Every module has its tests listed in the structure below, and `tasks.md` will put each test task
    before the implementation it covers.
  - The parsing and conversion modules (ADIF, QARTest, Maidenhead, band plan, mode mapping, cty.dat,
    provider XML, configuration merge) get edge-case and malformed-input tests, including the
    SC-006 fuzz-style corpus.
  - The QARTest reference capture is replayed as an integration test (SC-009).
- [x] **II. Rust Safety & Quality**:
  - `#![forbid(unsafe_code)]` in `lib.rs` and `main.rs`.
  - Errors use the crate `enum Error` and `Result<T>` alias in `src/errors.rs`, per
    `docs/rust-guidelines.md` §1 (R16). `unwrap`/`expect` are allowed only in
    tests and for documented invariants.
  - A bad datagram, record, provider response or WebSocket message is logged and skipped; none can
    end the process.
  - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are task gates.
- [x] **III. Simplicity & Minimal Dependencies**:
  - Each crate is justified below.
  - Logic that crates could provide but that is small and safety-relevant is written in-house: the
    ADIF parser, Maidenhead conversion, the band plan and the cty.dat parser.
  - Still a single self-contained binary, with frontend assets embedded.
  - No speculative abstractions: providers are a closed `enum`, not a plugin trait object.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| `tokio` (rt-multi-thread, net, sync, time, macros, signal, fs) | async runtime, UDP socket, timers, `broadcast` channel (channel 2), shutdown signal | std threads: no async I/O, no WebSocket; one runtime is needed by axum and reqwest anyway |
| `axum` (http1, ws, tokio) | HTTP server for the page, assets and `/healthz`; WebSocket upgrade | plain hyper plus tungstenite (more glue code); actix-web (second runtime model) |
| `async-channel` | bounded **MPMC** queue for channel 1, shared by W workers | tokio `mpsc` is single-consumer; `Mutex<Receiver>` works around it and obscures intent; `flume` has a larger API |
| `reqwest` (rustls only, no default features) | HTTPS client for QRZ.com, HamQTH, QRZCQ and the cty.dat download, with timeouts | `ureq` is blocking (`spawn_blocking` per lookup); hyper-util client needs much more code; std has no HTTP |
| `quick-xml` | parse QARTest messages and provider XML responses | hand-written scanning is fragile (entities, CDATA); `roxmltree` builds a DOM that is not needed |
| `clap` (derive, env) | command line plus `LQM_*` environment with built-in precedence (FR-032) | hand-rolled argv/env parsing: error-prone, no help text |
| `serde` (derive) | deserialise the TOML file; serialise the WebSocket JSON | none practical |
| `toml` | configuration file format (FR-032) | none in std |
| `serde_json` | WebSocket protocol encoding | hand-written JSON: error-prone |
| `tracing`, `tracing-subscriber` (fmt, env-filter) | levelled, structured logs (warn/debug rules in FR-005, FR-005a) | `log` plus `env_logger` is comparable; tracing fits async tasks better and is already used by axum and tower |
| dev: `tokio-tungstenite` | WebSocket client in integration tests (fan-out, set_limit, lag) | no WebSocket client in std or in the runtime dependencies |
| dev tool: Node.js (`node --test`) | unit tests for `map-state.js` and `geodesic.js`; no npm packages | not testing frontend logic, which would break Principle I; a headless browser harness is heavier |

## Project Structure

### Documentation (this feature)

```text
specs/001-live-qso-map/
├── plan.md              # this file
├── research.md          # Phase 0: decisions R1–R18
├── data-model.md        # Phase 1: entities, gate, history, session, frontend state
├── quickstart.md        # Phase 1: run and validation walkthrough
├── contracts/
│   ├── udp-input.md
│   ├── websocket-protocol.md
│   ├── http-endpoints.md
│   └── configuration.md
├── reference/
│   └── qartest-16.9.1-cqww-ssb.txt
├── checklists/requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLI entry: load config, init tracing, run pipeline, graceful shutdown
├── lib.rs               # #![forbid(unsafe_code)]; module tree; pub API used by integration tests
├── errors.rs            # enum Error, impl Display + std::error::Error + From<…>; type Result<T> (docs/rust-guidelines.md §1)
├── config.rs            # clap args + TOML file → merged Config; validation        [tests: precedence, validation]
├── adif/
│   ├── mod.rs           # AdifRecord (ordered uppercase map)
│   ├── parse.rs         # datagram → records, recovering per record                [tests: header, multi, malformed corpus]
│   └── write.rs         # record → ADIF text; LAT/LON XDDD MM.MMM formatting         [tests: round-trip]
├── band.rs              # ADIF band plan; freq → band; metres → band               [tests: edges, VHF/UHF]
├── mode.rs              # logger mode → ADIF MODE/SUBMODE                           [tests]
├── maidenhead.rs        # locator ↔ lat/lon (4/6/8 chars), validation               [tests]
├── ingest/
│   ├── mod.rs           # UDP receive loop → QsoEvent → gate → channel 1
│   ├── detect.rs        # SourceFormat detection                                    [tests]
│   ├── qartest.rs       # QARTest XML → QsoEvent (FR-004a)                          [tests: every reference packet]
│   └── gate.rs          # Identity, fingerprint, ResendGate                         [tests: identical/update/old*/eviction]
├── enrich/
│   ├── mod.rs           # worker loop: station fill → resolve → fields → history upsert + broadcast
│   ├── station.rs       # FR-011 defaults                                           [tests]
│   ├── resolve.rs       # candidate selection by precision/order (R11)              [tests: ties, timeouts via fake source]
│   ├── cty.rs           # cty.dat parse, lookup, download/refresh                   [tests: overrides, =exact, /suffixes, IS0/IT9]
│   └── provider/
│       ├── mod.rs       # enum Provider { Qrz, HamQth, QrzCq }; session cache; timeout
│       ├── qrz.rs       # XML → LookupCandidate incl. geoloc mapping                [tests: fixture XML, errors]
│       ├── hamqth.rs    #                                                           [tests: fixture XML, namespace, errors]
│       └── qrzcq.rs     #                                                           [tests: fixture XML, errors]
├── history.rs           # History (rev-checked upsert, stable sort key, eviction)   [tests]
├── pipeline.rs          # wires channels, workers, history, broadcast, servers
└── web/
    ├── mod.rs           # axum router: /, assets, /ws, /healthz
    ├── protocol.rs      # hello/snapshot/upsert/set_limit, QsoView                  [tests: JSON shape]
    ├── session.rs       # per-page session state machine                            [tests]
    └── assets/          # embedded via include_str!
        ├── index.html
        ├── app.css
        ├── app.js       # DOM + Leaflet glue, WebSocket with reconnect
        ├── map-state.js # pure state: window, grouping, colour, z-order, rev rule
        └── geodesic.js  # great-circle interpolation, antimeridian unwrapping

web/                     # frontend tests (node --test), import ../src/web/assets/*.js
├── map-state.test.js
└── geodesic.test.js

tests/
├── adif_udp.rs          # real UDP socket → history (US1, FR-003)
├── qartest_replay.rs    # reference capture → SC-009 assertions (US4, US5)
├── lookup_resolution.rs # fake providers + cty fixture → precision/order/timeout (US2)
├── ws_protocol.rs       # tokio-tungstenite clients: fan-out, set_limit, lag → snapshot (US3)
├── fixtures/            # provider XML responses, cty.dat excerpt, malformed ADIF corpus
└── load_ws.rs           # #[ignore] SC-004 load test, run manually on a Raspberry Pi

deploy/
├── live-qso-map.service # systemd unit example
└── config.example.toml
Dockerfile               # multi-stage → static musl binary on a minimal non-root image
Cross.toml               # cross targets: x86_64/aarch64 musl, armv7 musleabihf
```

**Structure Decision**: a single crate with a `lib.rs` and `main.rs` split, so that integration tests
can drive the pipeline without a process boundary. Frontend assets live inside `src/web/assets` and are
embedded at compile time. Their pure-logic modules are unit-tested with Node's built-in runner from
`web/`.

## Implementation Notes

- **Provider testability**: `enum Provider` holds an HTTP client and a session. Provider tests run on
  recorded XML fixtures and need no network access. `resolve.rs` takes a list of candidate futures, so
  that tests can inject fake sources with controlled delays and failures. That covers timeouts and ties
  without a mock HTTP server.
- **Secrets**: passwords are wrapped in a newtype with a redacted `Debug`. Provider URLs that carry
  credentials are never logged.
- **Graceful shutdown**: on SIGINT or SIGTERM, stop the UDP receive loop, close channel 1, let the
  workers drain within the timeout, and close the WebSockets.
- **cty.dat startup**: download or refresh before the UDP listener starts, bounded by
  `lookup.timeout_secs × 4`. A failure never blocks startup (R13).
- **Station position line**: the line runs from the `station` position of each marker's most recent
  QSO. That is `MY_LAT`/`MY_LON`, then `MY_GRIDSQUARE`, then the configured latitude/longitude, then
  the configured locator.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] I: every contract has a matching test file. The reference capture and the malformed corpus are
  fixtures.
- [x] II: there is no `unsafe`. Every external input (UDP, provider XML, cty.dat, WebSocket JSON,
  configuration) goes through a fallible parser that is logged and skipped.
- [x] III: 10 runtime crates and 1 dev crate, all justified. No new runtime services: providers and
  cty.dat are optional and degrade gracefully.
