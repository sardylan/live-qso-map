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
- Rust (edition 2024) + browser ES modules; no new crates (016-map-autozoom-fixes)
- Rust (edition 2024) + browser ES modules; no new crates; status derived from the in-memory history, nothing stored (017-station-status-box)

### Recent Changes

- 017-station-status-box: station status box (band and operator of the latest QSO by QSO time; operator falls back to the station callsign) via a new `station_status` WebSocket message, protocol still 4; greyed after 15 min
- 016-map-autozoom-fixes: flat auto-zoom centred on the QTH; globe kept only while markers stay `map.globe_border_deg` (default 10°) inside the visible horizon, else flat; the globe no longer shrinks to a dot
- 015-example-config-release: `config.example.toml` (every setting commented out, `"<…>"` placeholders for credentials and station) in every release file, in `/usr/share/doc/live-qso-map/` of the .deb and image, and as a standalone release download; Debian gets a short live `config.toml`
- 014-adif-persistence: QSO history in a local ADIF file (load and re-enrich at startup, save at graceful stop, `--enrich-only [FILE]`); no server history limit, page capped at 1000

<!-- SPECKIT END -->
