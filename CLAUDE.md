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

### Recent Changes

- 009-fix-ci-pipeline: Added CircleCI config 2.1 and POSIX `sh` CI scripts + no new crates; CI-only tools cross 0.2.5, cargo-deb 3.8.0, cargo-deny 0.20.2, gh 2.102.0, crane 0.22.1 (pinned, SHA-256 verified); maintainer-only cargo-release 1.1.6
- 008-platform-packaging: Added Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2) + none new (clap gains `wrap_help` and `unicode`; existing `toml` writes the export); CI-only release tools cargo-deb 3.8.0, cross, docker buildx, zip
- 007-contest-score-layout: Added Rust (edition 2024, latest stable toolchain); browser ES modules + none new (existing `quick-xml` reads the score message)

<!-- SPECKIT END -->
