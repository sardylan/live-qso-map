# Research: Live QSO Map

**Feature**: `001-live-qso-map` | **Date**: 2026-10-08 | **Spec**: [spec.md](spec.md)

Every decision below was either taken with the project owner on 2026-10-08 or is derived from
documentation of the external formats the spec mandates. Each entry gives the decision, why it was made
and what else was considered.

---

## R1. Runtime, HTTP and WebSocket stack

- **Decision**: `tokio` 1.x for the async runtime, the UDP socket, timers and the broadcast channel.
  `axum` 0.8 for HTTP and WebSocket, with its `ws` feature.
- **Rationale**: one runtime covers UDP, HTTP, WebSocket and outbound HTTP. axum is a thin layer over
  hyper/tower that has WebSocket support built in, so no separate WebSocket crate is needed at runtime.
- **Alternatives**: actix-web, which brings its own actor runtime and is heavier; plain hyper with
  tungstenite, which needs more hand-written glue; a synchronous std-only design, which would need
  threads for UDP, HTTP, WebSocket and lookups and has no WebSocket support in std.

## R2. Channels between stages

- **Decision**:
  - Channel 1 (ingest to enrichment) is a bounded MPMC queue from `async-channel`, so the
    configurable pool of enrichment workers (default 1) can share it.
  - Channel 2 (enrichment to WebSocket sessions) is a `tokio::sync::broadcast`. Every enrichment
    worker can send on it, and every open page's session receives every message.
- **Rationale**: this matches the owner's design direction, an MPMC work queue followed by a broadcast
  fan-out. tokio's broadcast is MPMC by design. When a slow page falls behind it gets `Lagged`, and the
  session then sends that page a fresh snapshot, so a slow browser never blocks the pipeline.
  async-channel is a small, focused crate. tokio's own `mpsc` is single-consumer.
- **Alternatives**: `flume`, which has a larger API than needed; tokio `mpsc` behind a shared
  `Mutex<Receiver>`, which works but obscures the intent and serialises the receivers; `crossbeam`,
  which is not async.

## R3. Order of duplicate detection and enrichment

- **Decision**: a **resend gate** sits right after parsing, before channel 1. It keeps one entry per
  identity: a fingerprint of the normalised record before enrichment, plus a revision number (`rev`).
  - An identical fingerprint means the message is dropped, so no lookups are made.
  - A different fingerprint means a new revision (`rev + 1`), which is enriched as a correction.
  - An unknown identity starts at `rev = 1`.

  After enrichment, the history only accepts a version whose `rev` is higher than the one it already
  holds. Pages apply the same rule.
- **Rationale**: identical resends are common, and every lookup counts against QRZ.com and HamQTH
  quotas (FR-016 forbids caching lookups). With more than one worker, a correction can finish before
  the original does. The revision number keeps the newest version in every case.
- **Alternatives**: deduplicating only in the history, after enrichment, which the owner rejected
  because every resend would cost a lookup.

## R4. History and fan-out consistency

- **Decision**: the history is shared state (`Mutex<History>`). It is ordered by a sort key
  `(QSO datetime at first sight, identity)` and indexed by identity. An enrichment worker inserts or
  updates under the lock and, if the history accepted the change, sends the `upsert` on channel 2
  **while still holding the lock**. Broadcast sends never block, so holding the lock is safe, and it
  keeps the order of history changes identical to the order of broadcasts. A new session first
  subscribes to channel 2 and then takes a snapshot under the lock. Pages apply upserts idempotently by
  `(id, rev)`, so an event that appears both in the snapshot and on the channel is harmless.
- **Eviction**: when the size exceeds `history_size` (0 means unbounded), the entry with the oldest
  sort key is removed. The resend gate is capped at the same size, evicting oldest first, so a
  correction for an evicted QSO is treated as a new QSO, as the spec's edge case requires.
- **Sort-key stability**: FR-028 says a correction keeps its original place in time order. The sort key
  is therefore fixed when the QSO is first inserted, even if a correction changes TIME_ON.

## R5. Detecting the datagram format (FR-002)

- **Decision**: the first non-whitespace bytes decide the format.
  - Starts with `<?xml` or `<`, and the root element is `contactinfo`, `contactreplace`,
    `contactdelete` or `dynamicresults`, and `<logger>`/`<soft>` starts with `QARTest`: **QARTest**.
  - Otherwise, contains `<EOR>` or `<EOH>` (case-insensitive), or starts with an ADIF tag (`<NAME:len>`):
    **ADIF**.
  - Anything else, including ADX (an XML root of `ADX`): **unrecognised**, logged at warn level and
    dropped (FR-005).
- **Rationale**: QARTest XML and ADIF cannot be confused. ADIF tags always carry a `:length`, while XML
  tags never do. Checking the logger name keeps future N1MM support, which uses the same root elements,
  as a separate, deliberate addition.

## R6. ADIF parsing and writing (in-house)

- **Decision**: a small in-house parser for ADIF 3.1.x.
  - Fields are `<NAME:LEN[:TYPE]>DATA`, and `LEN` counts **bytes**.
  - The header is optional and ends at `<EOH>`; text before the first `<` is ignored.
  - Records end at `<EOR>`. Field names are case-insensitive and stored uppercase.
  - A record with an invalid length or a truncated value is rejected on its own; parsing resumes at
    the next `<EOR>`, so the valid records around it still go through (SC-006).

  A writer serialises records back to ADIF, for logs and for the `adif` text in the WebSocket payload.
- **Rationale**: the grammar is tiny (about 150 lines), it must be fuzz-tested against malformed input
  (SC-006, constitution I), and the existing crates are small, sparsely maintained and do not recover
  from errors per record.
- **Alternatives**: the `adif` and `adif-rs` crates, rejected for those reasons.

## R7. QARTest XML (FR-004, FR-004a)

- **Decision**: parse with `quick-xml`, reading events into a flat map from element name to text,
  because the messages are one level deep. Apply the FR-004a table. `txfreq` is in units of 10 Hz, so
  `FREQ` in MHz is `txfreq / 100000` with 6 decimals and `BAND` comes from the band plan (R9). The
  `timestamp` is UTC and becomes `QSO_DATE` and `TIME_ON`. Application-specific fields use the ADIF
  `APP_LIVEQSOMAP_*` namespace, for example `APP_LIVEQSOMAP_LOGGER_ID` and
  `APP_LIVEQSOMAP_QARTEST_COUNTRYPREFIX`.
- **Evidence**: reference capture `reference/qartest-16.9.1-cqww-ssb.txt` (QARTest 16.9.1).
- **Alternatives**: `roxmltree`, which needs a DOM and is not needed for flat messages, and
  hand-written string scanning, which is fragile with escaping and CDATA.

## R8. Mode mapping

- **Decision**:
  - `LSB` and `USB` become `MODE=SSB` with `SUBMODE=LSB/USB`.
  - `CW`, `RTTY`, `FM`, `AM` and `FT8` are kept as they are.
  - `FT4` becomes `MFSK` with `SUBMODE=FT4`. `PSK31` becomes `PSK` with `SUBMODE=PSK31`.
  - Any other value is kept uppercased in `MODE` and logged at debug level.

  The identity key (FR-007) uses `MODE` after mapping.
- **Rationale**: these follow the ADIF 3.1 mode and submode enumeration. An unknown mode is better
  kept than dropped.

## R9. Band plan

- **Decision**: a static table of ADIF 3.1 bands (2190m to submm) with lower and upper edges in MHz.
  `BAND` comes from the frequency, and QARTest's `<band>` in metres (e.g. `40` becomes `40m`) is used
  only when the frequency is missing or zero (owner decision). A frequency outside every band leaves
  `BAND` absent.

## R10. Maidenhead locator

- **Decision**: in-house conversion of a locator with 4, 6 or 8 characters to the centre of its square.
  - Field `A–R`, square `0–9`, subsquare `a–x` (case-insensitive), extended square `0–9`.
  - Any other length or character makes the locator invalid, so it is treated as absent.
  - Precision levels: 4 characters is `Locator4`; 6 or 8 characters is `Locator6` (the spec's
    "6-character or better").

  The reverse conversion, from latitude/longitude to a 6-character locator, is used to fill
  `GRIDSQUARE` when only coordinates are known.
- **Alternatives**: the `maidenhead` crate, rejected because the logic is about 40 lines and pinned
  down by tests.

## R11. Position precision and selection (FR-012 – FR-014)

- **Decision**: `Precision` is ordered `Country < Locator4 < Locator6 < Coordinates`.
  - **QSO already has a position**: if the record carries `LAT`/`LON` (coordinates) or `GRIDSQUARE`,
    that position is used and no lookups are made.
  - **Otherwise**: all configured online providers and the country data are queried **at the same
    time**, each bounded by `lookup_timeout`. The candidate with the highest precision wins, and a tie
    goes to the provider listed first in `provider_order`. The country data always comes last on ties.
  - **QSO is placed at**: the winning position's latitude and longitude, recorded in
    `APP_LIVEQSOMAP_POS_SOURCE` and `APP_LIVEQSOMAP_POS_PRECISION`.
- **QRZ `geoloc` mapping** (owner decision):

  | `geoloc` value | Precision |
  |----------------|-----------|
  | `user`, `geocode`, `zip` | Coordinates |
  | `grid` | Locator4 or Locator6, from the length of `grid` |
  | `state`, `dxcc` | Country |
  | `none` | ignored |

- **HamQTH and QRZCQ**: `latitude`/`longitude` count as Coordinates. `grid`/`locator` count as a
  locator, at the precision its length gives.
- **Worst-case latency**: one `lookup_timeout`, because providers are queried in parallel, not one
  after another. With the default 5 s this meets SC-002.

## R12. Online providers

| Provider | Login | Lookup | Session lifetime | Position fields |
|----------|-------|--------|------------------|-----------------|
| QRZ.com | `GET https://xmldata.qrz.com/xml/current/?username=U;password=P;agent=A` gives `<Session><Key>` | `?s=KEY;callsign=C` | up to 24 h; log in again when `<Key>` is missing | `lat`, `lon` (signed decimal), `grid`, `geoloc`, `dxcc`, `land`, `cqzone`, `ituzone` |
| HamQTH | `GET https://www.hamqth.com/xml.php?u=U&p=P` gives `<session><session_id>` | `?id=SID&callsign=C&prg=liveqsomap` | 1 h; log in again on "Session does not exist or expired" | `latitude`, `longitude`, `grid`, `adif` (DXCC), `country`, `cq`, `itu`, `continent` |
| QRZCQ | `GET https://ssl.qrzcq.com/xml?username=U&password=P&agent=A` gives `<Session><Key>` | `?s=KEY&callsign=C&agent=A` | 3 days; log in again on `Session Timeout` | `latitude`, `longitude`, `locator`, `dxcc`, `country`, `cq`, `itu`, `continent` |

- **Session keys are cached** in memory and refreshed when they expire. This does not break FR-016,
  which forbids caching *lookup results*, not login sessions. A concurrent login is single-flighted
  per provider.
- **"Not found"** (`Not found: X` or `Callsign not found`) means no candidate, logged at debug level.
  Login failures and HTTP errors are logged at warn level, and the provider is skipped for that QSO.
- **Fields filled** (FR-017), only when absent from the record: `GRIDSQUARE`, `LAT`, `LON`
  (ADIF `XDDD MM.MMM` format), `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `NAME`, `QTH`, `STATE`,
  `CNTY`, `IOTA`. Values come from the winning provider first, then from the others in
  `provider_order`.
- **Sources**: [QRZ XML spec](https://www.qrz.com/docs/xml/current_spec.html),
  [HamQTH developers](https://www.hamqth.com/developers.php),
  [QRZCQ XML API](https://www.qrzcq.com/docs/api/xml/).

## R13. Country data (AD1C `cty.dat`)

- **Decision**: download `https://www.country-files.com/cty/cty.dat` at startup when the configured
  file is missing or older than `country_data.max_age_days` (default 30), then save it to the
  configured path. If the download fails, any existing file, even an old one, is used. If there is no
  file at all, the country level is disabled and a warning is logged. The file is never embedded in
  the binary.
- **Format**: entity header lines with fields separated by `:`: name, CQ zone, ITU zone, continent,
  latitude (+N), **longitude (+W, so the sign must be inverted)**, UTC offset, primary prefix (`*` for
  WAE-only entities). These are followed by alias lines separated by `,` and ending in `;`.
  - `=CALL` marks an exact callsign.
  - Overrides: `(cq)`, `[itu]`, `<lat/lon>`, `{cont}`, `~utc~`.
- **Lookup**: an exact `=` match first, then the longest prefix match. Callsigns with a `/` follow the
  common rules:
  - Ignore `/P`, `/M`, `/QRP`, `/A`.
  - `/MM` and `/AM` have no entity, so there is no candidate.
  - A one-digit suffix replaces the call-area digit.
  - Otherwise use the part that is a known prefix (usually the shorter one).

  Sardinia (`IS0`/`IM0`) and Sicily (`IT9`) are separate entities in cty.dat, which satisfies SC-007.
- **Source**: [CTY file format](https://www.country-files.com/cty-dat-format/), maintained by AD1C.

## R14. Configuration (FR-032, FR-033)

- **Decision**: `clap` 4 with `derive` and `env`. Every option has a long flag and a `LQM_*` environment
  variable, and clap's own precedence gives command line over environment. Values from the TOML file
  (`--config`/`LQM_CONFIG`, read with `serde` and `toml`) fill whatever both left unset. Built-in
  defaults come last. Lists use repeatable flags or comma-separated environment values, for example
  `--band-color 20m=#1f77b4` or `LQM_BAND_COLORS="20m=#1f77b4,40m=#ff7f0e"`.
- **Alternatives**: `figment` or `config-rs`, which bring more layers and dependencies than needed.

## R15. Frontend (owner decision)

- **Decision**:
  - **Files**: plain HTML, CSS and ES-module JS, embedded in the binary with `include_str!`.
  - **Map library**: Leaflet **1.9.4** loaded from cdnjs with SRI hashes. Tiles default to
    OpenStreetMap and can be configured.
  - **Geodesic lines**: computed in our own JS by spherical interpolation (about 64 points per line).
    Longitudes are unwrapped so lines that cross the antimeridian draw continuously.
  - **Markers**: `L.marker` with a coloured `divIcon`, so that `setZIndexOffset` can stack markers by
    QSO time (FR-028). This is not possible with SVG circle markers.
  - **Logic**: all decisions live in pure modules. `map-state.js` handles the window of the last N,
    grouping by position, the colour of the most recent QSO, the revision rule and `diffLayers`.
    `connection.js` handles the reconnect backoff, the post-connect handshake and the remembered N.
    `app.js` is DOM and Leaflet glue only.
- **Testing**: `map-state.js`, `connection.js` and `geodesic.js` are tested with Node's built-in runner (`node --test`), with **no npm
  packages**. Node is a developer-only tool: it is not needed to build or run the service.
- **Alternatives**: embedding Leaflet in the binary, which is offline-friendly but unnecessary because
  tiles need the Internet anyway; a TypeScript bundler, rejected by the owner; the
  `Leaflet.Geodesic` plugin, an extra CDN dependency for about 40 lines of math.

## R16. Errors and logging

- **Decision** (binding through `docs/rust-guidelines.md` §1): a crate-wide `enum Error` in
  `src/errors.rs` with `impl std::fmt::Display`, `impl std::error::Error` and
  `From` conversions for the underlying errors (io, toml, xml, http, json). A crate alias,
  `type Result<T> = std::result::Result<T, Error>;`, is used everywhere. No `thiserror` or `anyhow`
  (owner decision). Logging uses `tracing` with `tracing-subscriber`; the level is configurable
  (`--log-level`, `LQM_LOG_LEVEL`, default `info`).

## R17. Outbound HTTP

- **Decision** (settled in T001, 2026-10-08): `reqwest` 0.13 with `default-features = false` and
  `rustls-no-provider`, plus a direct `rustls` 0.23 dependency with only `ring`, `std` and `tls12`.
  `main` installs `rustls::crypto::ring::default_provider()` once at startup. A shared `Client` has
  connect and read timeouts. Server certificates are verified with `rustls-platform-verifier`, so the
  container image must ship CA certificates (T090).
- **Why ring**: reqwest's default `rustls` feature uses `aws-lc-rs`, which needs cmake and a C
  toolchain to cross-compile to musl/armv7. `ring` builds with just a C compiler, which `cross`
  images provide. `cross` was not installed when T001 ran, so the armv7 build check moves to T091.
- **Alternatives**: `ureq`, which is blocking and would need `spawn_blocking` for every lookup, and
  hyper-util's client, which needs more boilerplate.

## R18. Build and deployment (owner decision)

- **Decision**:
  - **Binaries**: static musl binaries for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`
    (Raspberry Pi 4/5) and `armv7-unknown-linux-musleabihf` (32-bit Raspberry Pi OS), built with
    `cross`.
  - **Container**: a multi-stage `Dockerfile` that runs the static binary on a minimal non-root base
    image and declares `EXPOSE 12060/udp 8080/tcp` and a config volume.
  - **systemd**: an example unit and config in `deploy/`.
  - **Health check**: `GET /healthz` for container and cloud health checks.

---

## Open items

None. Every item marked NEEDS CLARIFICATION in Technical Context is resolved above.
