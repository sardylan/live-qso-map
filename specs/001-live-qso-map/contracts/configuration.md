# Contract: Configuration

**Precedence** (FR-032): command line, then environment (`LQM_*`), then the TOML file, then built-in
defaults.
**File**: `--config <path>` / `LQM_CONFIG`. Optional; when omitted, only the command line,
environment and defaults apply.

## Settings

| TOML key | CLI flag | Env var | Default | Notes |
|----------|----------|---------|---------|-------|
| `bind_address` | `--bind-address` | `LQM_BIND_ADDRESS` | `0.0.0.0` | used for both the UDP and HTTP listeners |
| `udp_port` | `--udp-port` | `LQM_UDP_PORT` | `12060` | |
| `http_port` | `--http-port` | `LQM_HTTP_PORT` | `8080` | |
| `history_size` | `--history-size` | `LQM_HISTORY_SIZE` | `10000` | `0` means unbounded; also caps the resend gate |
| `workers` | `--workers` | `LQM_WORKERS` | `1` | number of enrichment workers; must be at least 1 |
| `ingest_queue` | `--ingest-queue` | `LQM_INGEST_QUEUE` | `1024` | capacity of channel 1 |
| `broadcast_queue` | `--broadcast-queue` | `LQM_BROADCAST_QUEUE` | `1024` | capacity of channel 2; a session that falls further behind gets a fresh snapshot |
| `log_level` | `--log-level` | `LQM_LOG_LEVEL` | `info` | `error`, `warn`, `info`, `debug` or `trace` |
| `station.callsign` | `--station-callsign` | `LQM_STATION_CALLSIGN` | none | fills `STATION_CALLSIGN` when the record lacks it (FR-011) |
| `station.locator` | `--station-locator` | `LQM_STATION_LOCATOR` | none | fills `MY_GRIDSQUARE` |
| `station.latitude` | `--station-latitude` | `LQM_STATION_LATITUDE` | none | decimal degrees; when both lat and lon are set, they take priority over the locator for the line |
| `station.longitude` | `--station-longitude` | `LQM_STATION_LONGITUDE` | none | |
| `lookup.timeout_secs` | `--lookup-timeout` | `LQM_LOOKUP_TIMEOUT` | `5` | per provider; providers run in parallel |
| `lookup.provider_order` | `--provider-order` | `LQM_PROVIDER_ORDER` | `qrz,hamqth,qrzcq` | breaks ties at the same precision; `cty` is always last |
| `lookup.qrz.username` | `--qrz-username` | `LQM_QRZ_USERNAME` | none | the provider is enabled when username and password are set |
| `lookup.qrz.password` | `--qrz-password` | `LQM_QRZ_PASSWORD` | none | secret: never logged |
| `lookup.hamqth.username` | `--hamqth-username` | `LQM_HAMQTH_USERNAME` | none | |
| `lookup.hamqth.password` | `--hamqth-password` | `LQM_HAMQTH_PASSWORD` | none | secret |
| `lookup.qrzcq.username` | `--qrzcq-username` | `LQM_QRZCQ_USERNAME` | none | |
| `lookup.qrzcq.password` | `--qrzcq-password` | `LQM_QRZCQ_PASSWORD` | none | secret |
| `country_data.path` | `--cty-path` | `LQM_CTY_PATH` | `./cty.dat` | |
| `country_data.url` | `--cty-url` | `LQM_CTY_URL` | `https://www.country-files.com/cty/cty.dat` | |
| `country_data.max_age_days` | `--cty-max-age-days` | `LQM_CTY_MAX_AGE_DAYS` | `30` | `0` means never re-download while the file exists |
| `map.tile_url` | `--tile-url` | `LQM_TILE_URL` | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` | |
| `map.tile_attribution` | `--tile-attribution` | `LQM_TILE_ATTRIBUTION` | `&copy; OpenStreetMap contributors` | |
| `map.fallback_color` | `--fallback-color` | `LQM_FALLBACK_COLOR` | `#7f7f7f` | |
| `map.bands` (table) | `--band-color BAND=#RRGGBB` (repeatable) | `LQM_BAND_COLORS` (`BAND=#RRGGBB,...`) | built-in palette | an entry from a higher-priority layer replaces only that band |

## Validation (at startup, before any socket is opened)

The service exits with a non-zero status and a clear message when:

- a port is outside 1–65535, or `workers` or a queue capacity is `0`;
- `station.locator` is not a valid 4, 6 or 8-character locator, or the latitude/longitude is out of
  range or only one of the pair is set;
- a colour is not `#RRGGBB`, or a band name is not an ADIF band;
- `provider_order` names an unknown provider;
- the TOML file cannot be read or parsed.

A provider with a username but no password, or the reverse, is a warning and the provider is disabled.

## Example `config.toml`

```toml
udp_port = 12060
http_port = 8080
history_size = 10000

[station]
callsign = "IS0MZK"
locator = "JM49NH"

[lookup]
timeout_secs = 5
provider_order = ["qrz", "hamqth", "qrzcq"]

[lookup.qrz]
username = "IS0MZK"
password = "change-me"     # prefer LQM_QRZ_PASSWORD

[lookup.hamqth]
username = "IS0MZK"
password = "change-me"

[country_data]
path = "/var/lib/live-qso-map/cty.dat"

[map.bands]
"40m" = "#d62728"
"20m" = "#1f77b4"
```
