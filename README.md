# live-qso-map

[![CircleCI](https://dl.circleci.com/status-badge/img/gh/sardylan/live-qso-map/tree/main.svg?style=shield)](https://dl.circleci.com/status-badge/redirect/gh/sardylan/live-qso-map/tree/main)

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
   - **Compound callsigns** (`IS0/DL1ABC`, `IS0ABC/P`, `K1ABC/6`): if no online source knows the
     callsign as logged, it is looked up once more without the prefix or suffix (`DL1ABC`,
     `IS0ABC`, `K1ABC`). Maritime and aeronautical mobile (`/MM`, `/AM`) are never retried.
3. **Positions that can't be right are ignored**:
   - an online position outside the callsign's country (for `IS0/DL1ABC`, outside Sardinia; a
     country inside another one, like Sardinia in Italy, counts as outside);
   - 0°/0° from any source, and the `AA00` locator except for stations in Antarctica.

   The QSO then uses the next source, or the country centre. The QSO's field
   `APP_LIVEQSOMAP_POS_REJECTED` lists what was ignored and why, for example
   `qrz:outside-entity` or `logger:placeholder`, with `@base` when the answer came from the
   base callsign.
4. **Nothing found**: the QSO is still kept, but it isn't drawn.

When every online provider fails instead of answering (network error, timeout, used-up quota,
refused login), the QSO uses the country centre and its field `APP_LIVEQSOMAP_LOOKUP_FAILED` says
why, for example `qrz:timeout,hamqth:error`. The next start looks it up again (see
[History file](#history-file)).

Notes:

- Every popup shows where the position came from and how precise it is.
- **Provider quotas**: lookups are deliberately **not cached** (FR-016), so every QSO that needs a
  lookup counts against your QRZ.com/HamQTH/QRZCQ quota. QRZ.com needs a subscription for full
  data; HamQTH needs a free account.
- **Country data**: cty.dat is downloaded at startup and refreshed after 30 days. If the download
  fails, the cached copy is used.

## The map page

The page renders with [MapLibre GL JS](https://maplibre.org/) 5.24.0, loaded from cdnjs and
checked with Subresource Integrity. It is laid out for windows 1024 px wide or wider:

| Corner | Holds |
|--------|-------|
| top left | zoom, **Globe/Flat**, **Auto-zoom** |
| top right | the gear that opens the settings panel |
| bottom right | the band legend |
| bottom left | the station status line, the contest score line, and the last-QSO box under them when shown |

- **Settings panel**: the gear at the top right opens a panel on the right edge (full width on
  narrow screens), with every viewer setting in four groups:
  - **Map**: "Last N QSOs" (10 to 1000), **Atmosphere on the globe**, **Grayline (day and night)** and
    **Sun position**;
  - **Last QSOs box**: shown or hidden, its size and where the newest row goes;
  - **Appearance**: the theme;
  - **Legend**: which band groups to show.

  Every change applies at once. The panel closes with ✕, Escape, the gear, or after 5 minutes
  without input in it; the map stays usable beside it.
- **Themes**: Light, Dim and Dark, for the panels, the legend, the base map and the globe.
  - **Automatic (device)**, the default, follows the device's light/dark preference.
  - **Automatic (sun at station)** follows the sun at the house: Light by day, Dim in civil
    twilight (sun up to 6° below the horizon), Dark at night. It is checked every minute. With no
    station position yet, it follows the device.
  - The fixed choices show one theme always. A line under the choice says which theme is shown,
    and why.
  - With the default base map, each theme has its own OpenFreeMap style (Liberty, Fiord, Dark); a
    configured base map is dimmed instead.
- **Atmosphere**: on the globe, a sky and a thin glow around the Earth in the theme's colours,
  brightest on the edge towards the Sun, and turning with the globe as it moves. It never covers
  the map, its markers or its lines. On by default; the flat map has none.
- **Grayline**: the night side is shaded on the flat map and on the globe, fading from the
  horizon to full night 12° below it, so the twilight band where low-band paths open shows at a
  glance. It follows the Sun through the day. Place names, borders, QSO markers and paths stay
  on top. On by default.
- **Sun position**: a small Sun symbol where the Sun is overhead now. Off by default.
- **Legend groups**: "All considered bands" (the default), or any mix of the band groups the
  service considers, such as "HF contest (6 bands)" or "WARC (3 bands)". The legend, the map and
  the box then show only those bands, and QSOs with no band are hidden.
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
  - It refits after every change, at most once a second, and keeps clear of the legend, the box,
    the score line and the controls.
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
  - **Size and order** are chosen by each viewer in the settings panel: "Box: last" 3 to 20
    (default 5), and "Newest" at the bottom (default) or at the top. They are separate from the
    map's own "Map: last N QSOs" (default 20). The box can list QSOs the map doesn't show; those
    rows are faded and can't be clicked.
  - Click any other row to open its popup; the **Last QSOs** checkbox in the settings panel hides
    the box. Popups show each QSO's entity too.
- **Contest score**: one line above the box, always shown, from the score QARTest broadcasts
  every minute, for example `1.234.567 pts · 1.523 QSO · 183 mults`.
  - "mults" is the sum of every multiplier type the contest has (countries plus zones in CQ WW).
  - It reads `Score —` until the first score arrives, and again after the service restarts.
  - With no new score for 5 minutes it turns grey; hovering shows `Last updated HH:MMZ` (UTC).
- **Station status**: one line above the score, with the band and the operator of the latest
  QSO by QSO time, for example `20m · IS0XYZ`.
  - The operator is the one the logger stamps on each QSO (QARTest's OPON), so a new operator
    shows with their first QSO. With no operator, the station callsign is shown.
  - It reads `— · —` until a QSO is logged; after a restart it comes from the saved history.
  - 15 minutes after the latest QSO it turns grey; hovering shows `Last QSO HH:MMZ` (UTC).
- **Scale**: a metric scale bar, shown on the flat map only.

Each browser remembers every viewer choice across restarts: Globe/Flat, Auto-zoom, Last QSOs,
the map's "Last N", the box size and order, the theme, the atmosphere, the grayline, the Sun
position and the legend groups. The panel itself always starts closed.

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
and [`deploy/config.example.toml`](deploy/config.example.toml) lists every setting of the file,
commented out, with its default or a `"<…>"` placeholder (callsign, locator, position, provider
usernames and passwords). It ships as `config.example.toml` in every release file (next to the
license and read-me, or in `/usr/share/doc/live-qso-map/` in the `.deb` and the container image)
and as a download of its own on each GitHub release. Copy it to your configuration file,
uncomment what you change and replace the placeholders.
The most common settings:

| Setting | Flag / env | Default |
|---------|-----------|---------|
| UDP port | `--udp-port` / `LQM_UDP_PORT` | 12060 |
| HTTP port | `--http-port` / `LQM_HTTP_PORT` | 8080 |
| Station callsign and locator (used when a QSO lacks them) | `--station-callsign`, `--station-locator` | none |
| History file (see [History file](#history-file)) | `--history-file` / `LQM_HISTORY_FILE` | `qso-history.adi` next to `cty.dat` |
| QRZ.com credentials | `LQM_QRZ_USERNAME`, `LQM_QRZ_PASSWORD` | none |
| HamQTH credentials | `LQM_HAMQTH_USERNAME`, `LQM_HAMQTH_PASSWORD` | none |
| QRZCQ credentials | `LQM_QRZCQ_USERNAME`, `LQM_QRZCQ_PASSWORD` | none |
| Bands to consider (`all` for every band) | `--bands hf,-60m` / `LQM_BANDS` | `hf-contest` |
| Band colours | `--band-color 20m=#1f77b4` (repeatable) | built-in palette |
| Last-QSO box columns, in order (`call`, `band`, `mode`, `date`, `time`, `cqzone`, `country`) | `--recent-columns` / `LQM_RECENT_COLUMNS` | `call,band,time,cqzone,country` |
| Last-QSO box header row | `--recent-header true\|false` / `LQM_RECENT_HEADER` | `true` |
| Auto-zoom globe border: degrees of arc every QSO keeps inside the globe's horizon, else the flat map (0 to 45) | `--globe-border-deg` / `LQM_GLOBE_BORDER_DEG` | `10` |
| Base map style (MapLibre style JSON) | `--style-url` / `LQM_STYLE_URL`, `--style-attribution` | OpenFreeMap Liberty, with its credit |
| Raster tiles instead of a style | `--tile-url` / `LQM_TILE_URL`, `--tile-attribution` (required) | none |

A style and raster tiles can't be configured together. Raster tiles need an attribution, as
their usage terms require.

Passwords are never written to the logs. Prefer environment variables for them.

> **Warning: open access.** By design (FR-035), the map page and the UDP port have no
> authentication or source filtering. Anyone who can reach the UDP port can put QSOs on your map.
> When the service runs on the Internet, restrict the UDP port with a firewall to your station's
> address.

### Saving the configuration to a file

`--save-config <PATH>` writes the effective configuration to a file and exits, without starting
the map. It combines the command line, environment variables, the configuration file and the
defaults, so every setting is written with the value the program would use. `--dump-config <PATH>`
does the same, then starts the map. Use `-` as the path to print to the terminal.

This is the quickest way to create a configuration file on a fresh installation:

```bash
live-qso-map --station-callsign IS0MZK --station-locator JM49NH --bands hf-contest \
  --save-config live-qso-map.toml
```

The configuration is checked first, so an invalid setting writes nothing. An existing file is
replaced only once the new one is complete. On Linux and macOS a new file is readable by its owner
only, and a replaced file keeps its owner and permissions.

The file includes provider usernames and **passwords**, so it is a complete, working
configuration. The program prints a note when it holds any: keep that file private.

### Band selection

`--bands` (or `LQM_BANDS`, or `bands = [...]` in the file) limits the map to some bands. QSOs on
other bands, or with no known band, are dropped as soon as they arrive. They cost no callbook
lookups and never reach the map, and the legend lists only the considered bands.

- **Entries**: ADIF band names (`20m`, `70cm`) or groups, in any mix. Prefix an entry with `-` to
  exclude it.
- **Groups**: `lf-mf`, `hf` (160m–10m), `hf-contest` (160m 80m 40m 20m 15m 10m), `warc`, `vhf`,
  `uhf`, `shf`, `ehf`.
- **Default**: with no setting, the six contest HF bands: 160m 80m 40m 20m 15m 10m.
- **Every band**: `all` (`--bands all`, `LQM_BANDS=all` or `bands = ["all"]`). It goes alone:
  `all` with other entries is an error.
- **Upgrading from 2.0 or earlier**: the default used to be every band. To keep that, set
  `bands = ["all"]`; an existing band setting works unchanged.
- **Examples**: `hf,2m,-60m` is every HF band plus 2m, without 60m. `-60m` alone is every band except
  60m.
- **Startup**: the service logs the effective set (`bands considered: …`), and stops when an entry is
  unknown or the selection leaves no bands. `--save-config` always writes `bands`.
- **Corrections**: when a logger corrects a QSO onto an ignored band, it disappears from the map.

### History file

The service keeps every QSO in memory and shows it live, as before. It also keeps them in one ADIF
file, so a restart doesn't empty the map:

- **At startup** it reads the file. QSOs that already have a position are back on the map at once,
  with no new lookup. The others are looked up like QSOs from the logger, on their own worker,
  so live QSOs never wait for them. The page and the UDP port are available straight away.
- **At a graceful stop** (Ctrl+C, `systemctl stop`, `docker stop`) it writes every QSO back,
  including the positions it found. The file is replaced only once the new one is complete.
- **Not saved**: QSOs received since the last graceful stop are lost on a crash or a power cut;
  the logger can resend them. The contest score and the viewers' settings aren't saved either.
- **Where**: `--history-file` / `LQM_HISTORY_FILE` / `history_file`. By default `qso-history.adi`
  in the folder of `cty.dat`: `/var/lib/live-qso-map/` for the Debian package and the container,
  the program's folder on Windows and macOS.
- **Reset**: stop the service, delete the file, start it again. Deleting the file while the service
  runs does nothing: the stop writes it again.
- **Your own log**: you can put an ADIF log there while the service is stopped. Records with
  `GRIDSQUARE` or `LAT`/`LON` keep them; the others are looked up. Your fields are never changed;
  the service only adds fields, and lists them in `APP_LIVEQSOMAP_ADDED`. Records on bands outside
  `--bands` stay in the file untouched and aren't shown.
- **Retries**: a QSO whose lookups failed (`APP_LIVEQSOMAP_LOOKUP_FAILED`) is looked up again at
  each start. A rejected position (`APP_LIVEQSOMAP_POS_REJECTED`) isn't, to save quota.
- **Unreadable file**: if the file can't be read, it is renamed to
  `qso-history.adi.unreadable-<UTC time>` and the service starts empty.
- **No limit**: the service keeps every QSO; a page shows at most the newest 1000. The old
  `history_size` setting is ignored, with a warning.

### Completing a log file

`--enrich-only [FILE]` completes the QSOs in `FILE` (by default the history file) and writes it
back, then exits. It opens no port, so it can run next to the service on another file:

```bash
live-qso-map --enrich-only my-contest.adi --qrz-username IS0MZK --qrz-password …
```

- Every band is completed, whatever `--bands` says. Rejected and failed lookups are tried again.
- It logs a summary: QSOs placed from the record, by lookup, with a failed lookup, and without a
  position.
- Ctrl+C saves what was completed so far.
- Exit code 0 when the file was written; 1 when it is missing, unreadable or can't be written, and
  then the file is left as it was.

## Deployment

**Container** (Docker Hub, `linux/amd64` and `linux/arm64`, based on Debian 13 slim):

```bash
docker run -p 12060:12060/udp -p 8080:8080 \
  -v lqm-data:/var/lib/live-qso-map \
  -v "$PWD/config.toml:/etc/live-qso-map/config.toml:ro" \
  sardylan/live-qso-map:2.0.0
```

- **Configuration**: a file mounted at `/etc/live-qso-map/config.toml` is used as is. Without
  one, settings come from `LQM_*` environment variables (`-e LQM_STATION_CALLSIGN=IS0MZK`) or
  options after the image name. To start the file from the example in the image:
  `docker run --rm --entrypoint cat sardylan/live-qso-map:2.0.0 /usr/share/doc/live-qso-map/config.example.toml > config.toml`.
- **Data**: `cty.dat` and the history file `qso-history.adi` are kept in the
  `/var/lib/live-qso-map` volume, so they survive restarts.
- **User**: the process runs as the unprivileged user `10001`, not root.
- **Tags**: `2.0.0` for a release, `latest` for the newest one.
- **Building locally**: `docker build -t live-qso-map .` builds the same image from source on any
  host.

**Release binaries**: the CircleCI pipeline builds every target and attaches the binaries to
the GitHub release of each `vX.Y.Z` tag. The targets are static Linux for x86_64 (PC, VM,
cloud), aarch64 (Raspberry Pi 4/5) and armv7 (Raspberry Pi, 32-bit OS), plus macOS (Apple
silicon) and Windows (x64). Cross-compiling happens only in the pipeline; to build for your own
machine:

```bash
cargo build --release
```

**Debian and Ubuntu** (Debian 12 and 13, Ubuntu 22.04 and later; PC, Raspberry Pi, server):
download the `.deb` for your architecture (`amd64`, `arm64`, or `armhf` for a 32-bit Raspberry
Pi OS) from the GitHub release, then:

```bash
sudo apt install ./live-qso-map_2.0.0-1_amd64.deb
# edit /etc/live-qso-map/config.toml, or write it from options:
sudo live-qso-map --config /etc/live-qso-map/config.toml --station-callsign IS0MZK \
  --station-locator JM49NH --save-config /etc/live-qso-map/config.toml
sudo systemctl enable --now live-qso-map
```

- **Layout**: the program is `/usr/bin/live-qso-map`, the configuration is
  `/etc/live-qso-map/config.toml` (a short file that sets only where the data goes), and data
  (`cty.dat` and the history file `qso-history.adi`) goes to `/var/lib/live-qso-map/`. Every
  setting, commented out, is in `/usr/share/doc/live-qso-map/config.example.toml`: copy the lines
  you need into the configuration.
- **Account**: the service runs as the unprivileged `live-qso-map` account, created at install.
  The configuration file is readable only by root and that account, because it may hold
  provider passwords. `--save-config` over it keeps its owner and permissions.
- **Not started at install**: the UDP port has no authentication, so the service starts only
  after you enable it. Upgrades restart a running service.
- **Upgrades**: the example in `/usr/share/doc` is always replaced. The configuration follows
  the standard Debian rules for configuration files: a file you never changed is updated
  silently, a file you changed is kept, and when both you and the new version changed it, the
  upgrade asks whether to keep yours or take the new one (and can show the difference). The
  first upgrade to a version with the short configuration file changes the packaged file, so
  if you edited yours it asks once.
- **Remove versus purge**: `sudo apt remove live-qso-map` keeps the configuration, the data and
  the account; `sudo apt purge live-qso-map` deletes all three.

**Windows** (x64) **and macOS** (Apple silicon): download
`live-qso-map-2.0.0-windows-x64.zip` or `live-qso-map-2.0.0-macos-arm64.zip` from the GitHub
release and extract it to a folder of your choice. Then double-click the program: a terminal
window opens and the map starts at `http://localhost:8080/`. No installer and no administrator
rights are needed.

- **One folder**: everything stays in the program's folder. It reads `live-qso-map.toml` from
  there, downloads `cty.dat` there and keeps the history file `qso-history.adi` there, whatever
  folder the terminal starts in. Create the file by copying the bundled `config.example.toml`
  to `live-qso-map.toml` and uncommenting what you change, or
  with `--save-config live-qso-map.toml` from a terminal in that folder (`.\live-qso-map.exe` on
  Windows, `./live-qso-map` on macOS).
- **Windows errors**: if the program stops with an error, the message stays visible until you
  press Enter. This also happens when started from a terminal.
- **macOS first start**: the program isn't signed by an Apple developer, so the first time open
  System Settings → Privacy & Security and click **Open Anyway**.

**Health check**: `GET /healthz` returns `{"status":"ok","history":…,"sessions":…,"uptime_s":…}`.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test 'web/*.test.js'                                   # frontend logic, no npm packages; quoted so Node expands it
cargo test --release --test load_ws -- --ignored --nocapture  # load test (SC-004)
```

**Browser tests**: `node tests/browser/run.mjs` opens the map page of a release build
(`cargo build --release` first) in Chrome or Chromium and plays a short viewer session: QSOs, the
settings panel, the themes, a reload, a restart and a band-group choice. It needs Node 22+ and a
browser, and no npm packages; `--headed` shows the window. The report, the log and the
screenshots go to `target/browser-test/`. CI runs it only for releases, before publishing
(`tests/browser/README.md`, [`docs/ci.md`](docs/ci.md) section 11).

**Releases**: pushing a `vX.Y.Z` tag that matches the `Cargo.toml` version runs the CircleCI
release workflow. It builds every target, packages the `.deb` files, zips and Linux archives,
tests the packages, the image and the map page in a real browser, publishes the GitHub release
with `SHA256SUMS`, and only then
publishes the Docker Hub tags. The scripts in `scripts/` that it uses are POSIX `sh`, for CI
only. The publishing secrets, the triggers and the protection of `main` are described in
[`docs/ci.md`](docs/ci.md); cutting a release, step by step, in
[`docs/releasing.md`](docs/releasing.md).

| Document | Location |
|----------|----------|
| Project rules | [`.specify/memory/constitution.md`](.specify/memory/constitution.md) |
| Rust conventions | [`docs/rust-guidelines.md`](docs/rust-guidelines.md) |
| CI setup, triggers, protection of `main` | [`docs/ci.md`](docs/ci.md) |
| Specification, plan and tasks | [`specs/001-live-qso-map/`](specs/001-live-qso-map/) |
| Debugging an unknown logger | `tools/udp_capture.py` records raw datagrams |
| Simulating QARTest QSOs | `scripts/simulate_qartest_qso.py CALL ...` sends one `contactinfo` per callsign |
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
