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
- Rust (edition 2024) tests + POSIX sh CI checks; no new crates (015-example-config-release)

### Recent Changes

- 015-example-config-release: `config.example.toml` (every setting commented out, `"<…>"` placeholders for credentials and station) in every release file, in `/usr/share/doc/live-qso-map/` of the .deb and image, and as a standalone release download; Debian gets a short live `config.toml`
- 014-adif-persistence: QSO history in a local ADIF file (load and re-enrich at startup, save at graceful stop, `--enrich-only [FILE]`); no server history limit, page capped at 1000
- 013-atmosphere-follows-globe: the globe's glow is a backlit rim brightest towards the Sun, recomputed on every move (never a haze over the map); grayline as page-drawn raster tiles (smooth night fade, 0° to −12°) on both views; optional Sun symbol; release browser steps `atmosphere` and `grayline` on a test clock

<!-- SPECKIT END -->
