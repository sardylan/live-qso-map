# live-qso-map Development Guidelines

Project principles: `.specify/memory/constitution.md`.
Rust implementation rules (binding): `docs/rust-guidelines.md`.
Ham radio domain rules (binding, §1) and glossary (§2): `docs/ham-radio.md`.
HTTP/WebSocket interface rules (binding): `docs/http-api.md`.

Questions to the user (spec-kit clarifications included): ask them one at a time and wait for
each answer before asking the next. Never batch several questions in one message.

<!-- SPECKIT START -->
<!-- Managed by .specify/scripts/bash/update-agent-context.sh; edit outside these markers. -->

### Active Technologies

- Rust (edition 2024) + browser ES modules; no new crates (014-adif-persistence)
- one ADIF history file (`history_file`, next to cty.dat), replaced atomically at a graceful stop (014-adif-persistence)

### Recent Changes

- 014-adif-persistence: QSO history in a local ADIF file (load and re-enrich at startup, save at graceful stop, `--enrich-only [FILE]`); no server history limit, page capped at 1000
- 013-atmosphere-follows-globe: the globe's glow is a backlit rim brightest towards the Sun, recomputed on every move (never a haze over the map); grayline as page-drawn raster tiles (smooth night fade, 0° to −12°) on both views; optional Sun symbol; release browser steps `atmosphere` and `grayline` on a test clock
- 012-page-fixes-browser-tests: page fixes (served modules, toggle outline, opaque panel, no duplicate band groups); release-only real-browser tests over the Chrome DevTools Protocol, Node built-ins only

<!-- SPECKIT END -->
