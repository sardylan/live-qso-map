# live-qso-map Development Guidelines

Project principles: `.specify/memory/constitution.md`.
Rust implementation rules (binding): `docs/rust-guidelines.md`.
Ham radio domain rules (binding, §1) and glossary (§2): `docs/ham-radio.md`.

<!-- SPECKIT START -->
<!-- Managed by .specify/scripts/bash/update-agent-context.sh; edit outside these markers. -->

### Active Technologies

- Rust (edition 2024, stable toolchain; developed on 1.98) + tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber (001-live-qso-map)
- in-memory history (bounded, lost on restart); cty.dat file cached on disk (001-live-qso-map)

### Recent Changes

- 001-live-qso-map: Added Rust (edition 2024, stable toolchain; developed on 1.98) + tokio, axum, async-channel, reqwest (rustls), quick-xml, clap, serde, toml, serde_json, tracing, tracing-subscriber

<!-- SPECKIT END -->
