# live-qso-map Development Guidelines

Project principles: `.specify/memory/constitution.md`.
Rust implementation rules (binding): `docs/rust-guidelines.md`.
Ham radio domain rules (binding, §1) and glossary (§2): `docs/ham-radio.md`.

<!-- SPECKIT START -->
<!-- Managed by .specify/scripts/bash/update-agent-context.sh; edit outside these markers. -->

### Active Technologies

- Rust (edition 2024, stable toolchain; developed on 1.98) + tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber (001-live-qso-map)
- in-memory history (bounded, lost on restart); cty.dat file cached on disk (001-live-qso-map)
- Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules + no new crates; the frontend uses MapLibre GL JS 5.24.0 (cdnjs, SRI) (002-maplibre)
- browser `localStorage` for the projection preference; no server storage (002-maplibre)
- Rust (edition 2024, stable toolchain; developed on 1.98) + none new (existing tokio, axum, serde, clap, toml) (003-band-filter)
- none new; tombstones are kept in memory next to the history (003-band-filter)

### Recent Changes

- 003-band-filter: Added Rust (edition 2024, stable toolchain; developed on 1.98) + none new (existing tokio, axum, serde, clap, toml)
- 002-maplibre: Added Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules + no new crates; the frontend uses MapLibre GL JS 5.24.0 (cdnjs, SRI)
- 001-live-qso-map: Added Rust (edition 2024, stable toolchain; developed on 1.98) + tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber

<!-- SPECKIT END -->
