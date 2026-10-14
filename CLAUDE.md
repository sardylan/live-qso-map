# live-qso-map Development Guidelines

Project principles: `.specify/memory/constitution.md`.
Rust implementation rules (binding): `docs/rust-guidelines.md`.
Ham radio domain rules (binding, §1) and glossary (§2): `docs/ham-radio.md`.
HTTP/WebSocket interface rules (binding): `docs/http-api.md`.

<!-- SPECKIT START -->
<!-- Managed by .specify/scripts/bash/update-agent-context.sh; edit outside these markers. -->

### Active Technologies

- Rust (edition 2024, latest stable toolchain) + tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber (001-live-qso-map)
- in-memory history (bounded, lost on restart); cty.dat file cached on disk (001-live-qso-map)
- Rust (edition 2024, latest stable toolchain); browser ES modules + no new crates; the frontend uses MapLibre GL JS 5.24.0 (cdnjs, SRI) (002-maplibre)
- browser `localStorage` for the projection preference; no server storage (002-maplibre)
- Rust (edition 2024, latest stable toolchain) + none new (existing tokio, axum, serde, clap, toml) (003-band-filter)
- none new; tombstones are kept in memory next to the history (003-band-filter)
- Rust (edition 2024, latest stable toolchain); browser ES modules; Python 3 stdlib (dev-only generator) + no new crates or frontend libraries (004-qth-autozoom)
- browser `localStorage` (`lqm.autozoom`, `lqm.recent`); embedded `src/web/entity_extents.json` generated from Natural Earth (004-qth-autozoom)
- Rust (edition 2024, latest stable toolchain); browser ES modules + none new (005-recent-box-dxcc)
- none new; the entity is stored on the enriched QSO in the in-memory history (005-recent-box-dxcc)
- Rust (edition 2024, latest stable toolchain); browser ES modules + none new (006-recent-box-config)
- browser `localStorage` for `lqm.limit` (moved from session storage), `lqm.recent.size`, `lqm.recent.order` (006-recent-box-config)
- Rust (edition 2024, latest stable toolchain); browser ES modules + none new (existing `quick-xml` reads the score message) (007-contest-score-layout)
- in memory, the latest score only, lost on restart; no new browser keys (007-contest-score-layout)
- Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2) + none new (clap gains `wrap_help` and `unicode`; existing `toml` writes the export); CI-only release tools cargo-deb 3.8.0, cross, docker buildx, zip (008-platform-packaging)
- none new; Debian `/etc/live-qso-map/` and `/var/lib/live-qso-map/`, the same paths in the container (volume), the program's folder on Windows and macOS (008-platform-packaging)
- CircleCI config 2.1, POSIX `sh` CI scripts, one Rust integration test + no new crates; CI-only tools pinned and SHA-256 verified; maintainer-only cargo-release 1.1.6 (009-fix-ci-pipeline)
- `CHANGELOG.md`, `release.toml`, `deny.toml`, `deploy/github/main-ruleset.json`; CircleCI context `live-qso-map-release` (009-fix-ci-pipeline)
- Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2) + none new; reuses the bundled `entity_extents.json` rectangles (010-portable-callsigns)
- none new; one optional ADIF field `APP_LIVEQSOMAP_POS_REJECTED` in the in-memory record (010-portable-callsigns)
- Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2); browser ES modules + none new (MapLibre GL JS 5.24.0 `setSky`; OpenFreeMap `fiord`/`dark` styles) (011-theme-settings-panel)
- browser `localStorage`: `lqm.theme`, `lqm.atmosphere`, `lqm.legend`; protocol 4 (`hello.band_groups`, `set_limit.bands`) (011-theme-settings-panel)
- Rust (edition 2024, latest stable toolchain); browser ES modules; Node 22+ built-ins for the browser test runner + no new crates, no npm packages; CI-only cimg/node:24.21.0-browsers and Chrome for Testing 154.0.8037.92 (pinned, SHA-256 verified) (012-page-fixes-browser-tests)
- none; browser test artifacts in `target/browser-test/` (012-page-fixes-browser-tests)
- browser ES modules; one `/grayline.js` route in Rust + none new (MapLibre GL JS 5.24.0 `setLight` per move, `addProtocol` raster tiles, `symbol` layer) (013-atmosphere-follows-globe)
- browser `localStorage`: `lqm.grayline`, `lqm.sun`. No server state. (013-atmosphere-follows-globe)

### Recent Changes

- 013-atmosphere-follows-globe: the globe's glow is a backlit rim brightest towards the Sun, recomputed on every move (never a haze over the map); grayline as page-drawn raster tiles (smooth night fade, 0° to −12°) on both views; optional Sun symbol; release browser steps `atmosphere` and `grayline` on a test clock
- 012-page-fixes-browser-tests: page fixes (served modules, toggle outline, opaque panel, no duplicate band groups); release-only real-browser tests over the Chrome DevTools Protocol, Node built-ins only
- 011-theme-settings-panel: Added Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2); browser ES modules + none new; themes (light/dim/dark), sun-based auto theme, atmosphere, settings panel, band-group choice, default bands hf-contest

<!-- SPECKIT END -->
