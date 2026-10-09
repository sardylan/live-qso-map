# live-qso-map

A live map of every QSO an amateur radio station makes during a contest.

Your logger sends each QSO over UDP. live-qso-map converts it to ADIF, works out where the other
station is, and pushes it to every open map page. Each QSO shows up as a band-coloured marker
with a great-circle line from your station. Each viewer chooses how many recent QSOs to show.
The map opens as a 3D globe; each viewer can switch to a flat map.

It is contest-agnostic. It runs as one self-contained binary on a Raspberry Pi or mini PC, on a
VM, or in a container, and needs only one UDP port and one TCP port.

## Quick start

```bash
cargo build --release
./target/release/live-qso-map --station-callsign IS0MZK --station-locator JM49NH
```

Open <http://localhost:8080>, then point your logger at UDP port **12060**. To try it without a
logger:

```bash
printf '<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <GRIDSQUARE:6>JO62QM <EOR>' \
  | nc -u -w1 127.0.0.1 12060
scripts/send_burst.sh 150          # 150 synthetic QSOs
```

## Inputs

The format is detected from each datagram's content, so no per-sender setup is needed.

| Source | What is accepted |
|--------|------------------|
| **ADIF over UDP** | One or more records per datagram, optional header. Each record is one QSO. Malformed records are skipped; the others still go through. |
| **QARTest** | `contactinfo` (new QSO) and `contactreplace` (correction). Score (`dynamicresults`) and delete (`contactdelete`) messages are ignored. |

Anything else, including ADX, is logged and dropped.

Identical resends are ignored before any lookup. A changed resend, such as an edited exchange,
updates the existing map entry in place. QARTest's QSO id even follows callsign edits.

## Where markers go

1. **The logger's own position**: `LAT`/`LON`, or `GRIDSQUARE` at the centre of the square. No
   online lookup is made.
2. **Otherwise**, every configured source is queried at the same time, and the most precise answer
   wins: exact coordinates, then a 6-character locator, then a 4-character one, then the
   ham-radio country centre.
   - Online sources: **QRZ.com**, **HamQTH** and **QRZCQ**.
   - Country data: the AD1C [cty.dat](https://www.country-files.com/). Sardinia, Sicily and Italy
     are separate entities.
3. **Nothing found**: the QSO is still kept, but it isn't drawn.

Notes:

- Every popup shows where the position came from and how precise it is.
- **Provider quotas**: lookups are deliberately **not cached** (FR-016), so every QSO that needs a
  lookup counts against your QRZ.com/HamQTH/QRZCQ quota. QRZ.com needs a subscription for full
  data; HamQTH needs a free account.
- **Country data**: cty.dat is downloaded at startup and refreshed after 30 days. If the download
  fails, the cached copy is used.

## The map page

The page renders with [MapLibre GL JS](https://maplibre.org/) 5.24.0, loaded from cdnjs and
checked with Subresource Integrity.

- **Globe or flat**: pages open as a globe. The **Flat/Globe** button switches the view, and each
  browser remembers its own choice.
- **Base map**: by default, the free [OpenFreeMap](https://openfreemap.org) vector style, with no
  API key. Any MapLibre style URL can be configured, or raster tiles instead (see Configuration).
  If the base map can't be reached, QSOs are still drawn on an empty background and a banner
  says so.
- **WebGL**: the page needs it. Browsers without WebGL get a message suggesting to enable hardware
  acceleration.
- **QTH**: a house marks the station. Its position is the configured `[station]` position;
  without one, it is the station position of the newest QSO on the page. Every QSO is joined to
  the house by a black great-circle line.
- **Auto-zoom**: the **Auto-zoom** button, on by default, keeps the house and every QSO in view.
  - It refits after every change, at most once a second, and keeps clear of the legend, the box
    and the controls.
  - It pauses while a popup is open; a popup nobody touches closes after 30 s.
  - Any drag, zoom or rotation by hand switches it off.
  - With no QSOs yet, it frames the station's DXCC entity, found from `station.callsign`, or 500 km
    around the house when the entity is unknown.
  - On the globe, it switches to the flat map only when the QSOs cannot all fit on one side of the
    Earth, and returns to the globe when they fit again. Your saved choice is never changed.
- **Last QSOs**: a box at the bottom left lists the newest QSOs (by QSO time).
  - **Columns** are chosen by the operator (see Configuration). The default is callsign, band,
    UTC time, CQ zone and the DXCC entity from cty.dat as `prefix · name`. Long names are cut,
    and hovering shows them in full. A small header row labels the columns, unless the operator
    turns it off.
  - **Size and order** are chosen by each viewer in the top-right panel: "Box: last" 3 to 20
    (default 5), and "Newest" at the bottom (default) or at the top. They are separate from the
    map's own "Map: last N QSOs" (default 20). The box can list QSOs the map doesn't show; those
    rows are faded and can't be clicked.
  - Click any other row to open its popup; the **Last QSOs** button hides the box. Popups show
    each QSO's entity too.
- **Scale**: a metric scale bar, shown on the flat map only.

Each browser remembers every viewer choice across restarts: Globe/Flat, Auto-zoom, Last QSOs,
the map's "Last N", and the box size and order.

## Configuration

Every setting can be given three ways, and the first one found wins:

1. a command-line flag;
2. an `LQM_*` environment variable;
3. a TOML file (`--config`).

The full list is in
[`specs/001-live-qso-map/contracts/configuration.md`](specs/001-live-qso-map/contracts/configuration.md),
with the base-map changes in
[`specs/002-maplibre/contracts/configuration.md`](specs/002-maplibre/contracts/configuration.md)
the band selection in
[`specs/003-band-filter/contracts/configuration.md`](specs/003-band-filter/contracts/configuration.md),
and the last-QSO box in
[`specs/006-recent-box-config/contracts/configuration.md`](specs/006-recent-box-config/contracts/configuration.md),
and [`deploy/config.example.toml`](deploy/config.example.toml) is a commented starting point.
The most common settings:

| Setting | Flag / env | Default |
|---------|-----------|---------|
| UDP port | `--udp-port` / `LQM_UDP_PORT` | 12060 |
| HTTP port | `--http-port` / `LQM_HTTP_PORT` | 8080 |
| Station callsign and locator (used when a QSO lacks them) | `--station-callsign`, `--station-locator` | none |
| QSOs kept in memory (0 = unbounded) | `--history-size` | 10000 |
| QRZ.com credentials | `LQM_QRZ_USERNAME`, `LQM_QRZ_PASSWORD` | none |
| HamQTH credentials | `LQM_HAMQTH_USERNAME`, `LQM_HAMQTH_PASSWORD` | none |
| QRZCQ credentials | `LQM_QRZCQ_USERNAME`, `LQM_QRZCQ_PASSWORD` | none |
| Bands to consider | `--bands hf-contest,-60m` / `LQM_BANDS` | every band |
| Band colours | `--band-color 20m=#1f77b4` (repeatable) | built-in palette |
| Last-QSO box columns, in order (`call`, `band`, `mode`, `date`, `time`, `cqzone`, `country`) | `--recent-columns` / `LQM_RECENT_COLUMNS` | `call,band,time,cqzone,country` |
| Last-QSO box header row | `--recent-header true\|false` / `LQM_RECENT_HEADER` | `true` |
| Base map style (MapLibre style JSON) | `--style-url` / `LQM_STYLE_URL`, `--style-attribution` | OpenFreeMap Liberty, with its credit |
| Raster tiles instead of a style | `--tile-url` / `LQM_TILE_URL`, `--tile-attribution` (required) | none |

A style and raster tiles can't be configured together. Raster tiles need an attribution, as
their usage terms require.

Passwords are never written to the logs. Prefer environment variables for them.

> **Warning: open access.** By design (FR-035), the map page and the UDP port have no
> authentication or source filtering. Anyone who can reach the UDP port can put QSOs on your map.
> When the service runs on the Internet, restrict the UDP port with a firewall to your station's
> address.

### Band selection

`--bands` (or `LQM_BANDS`, or `bands = [...]` in the file) limits the map to some bands. QSOs on
other bands, or with no known band, are dropped as soon as they arrive. They cost no callbook
lookups and never reach the map, and the legend lists only the considered bands.

- **Entries**: ADIF band names (`20m`, `70cm`) or groups, in any mix. Prefix an entry with `-` to
  exclude it.
- **Groups**: `lf-mf`, `hf` (160m–10m), `hf-contest` (160m 80m 40m 20m 15m 10m), `warc`, `vhf`,
  `uhf`, `shf`, `ehf`.
- **Examples**: `hf,2m,-60m` is every HF band plus 2m, without 60m. `-60m` alone is every band except
  60m.
- **Startup**: the service logs the effective set (`bands considered: …`), and stops when an entry is
  unknown or the selection leaves no bands.
- **Corrections**: when a logger corrects a QSO onto an ignored band, it disappears from the map.

The history lives in memory only and is lost on restart.

## Deployment

**Container**

```bash
docker build -t live-qso-map .
docker run -p 12060:12060/udp -p 8080:8080 \
  -e LQM_STATION_CALLSIGN=IS0MZK -e LQM_STATION_LOCATOR=JM49NH \
  -v lqm-data:/data live-qso-map
```

**Static binaries**: built with [`cross`](https://github.com/cross-rs/cross). Targets are listed
in `Cross.toml`.

```bash
cross build --release --target aarch64-unknown-linux-musl       # Raspberry Pi 4/5 (64-bit)
cross build --release --target armv7-unknown-linux-musleabihf   # Raspberry Pi (32-bit OS)
cross build --release --target x86_64-unknown-linux-musl        # PC, VM, cloud
```

**systemd** (Raspberry Pi or mini PC): see `deploy/live-qso-map.service`. Secrets go in
`/etc/live-qso-map/env`.

**Health check**: `GET /healthz` returns `{"status":"ok","history":…,"sessions":…,"uptime_s":…}`.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test web/                                              # frontend logic, no npm packages
cargo test --release --test load_ws -- --ignored --nocapture  # load test (SC-004)
```

| Document | Location |
|----------|----------|
| Project rules | [`.specify/memory/constitution.md`](.specify/memory/constitution.md) |
| Rust conventions | [`docs/rust-guidelines.md`](docs/rust-guidelines.md) |
| Specification, plan and tasks | [`specs/001-live-qso-map/`](specs/001-live-qso-map/) |
| Debugging an unknown logger | `tools/udp_capture.py` records raw datagrams |
| HTTP and WebSocket rules | [`docs/http-api.md`](docs/http-api.md): JSON payloads, JSON error bodies |
| Entity extents for auto-zoom | `tools/entity_extents.py` (below) |

The DXCC entity extents in `src/web/entity_extents.json` are generated from
[Natural Earth](https://www.naturalearthdata.com/) (public domain). To regenerate them after
changing `tools/entity_extents.toml` or updating `cty.dat`:

```bash
python3 -m unittest tools/test_entity_extents.py
python3 tools/entity_extents.py --cty cty.dat --mapping tools/entity_extents.toml \
  --out src/web/entity_extents.json   # downloads Natural Earth v5.1.2 once into .cache/
```

The script fails when a cty.dat entity is neither mapped nor listed under `[unmapped]`.
