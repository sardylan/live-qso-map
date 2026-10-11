# Data Model: Platform Packages, Friendlier Command Line and Configuration Export

## Exported configuration

What `--save-config` / `--dump-config` write (R9). It is the existing configuration file format
(`contracts/configuration.md` of feature 001, extended by features 002–007), with every key
present.

| Section | Keys | Source in `Config` | Notes |
|---------|------|--------------------|-------|
| (top) | `bind_address`, `udp_port`, `http_port`, `history_size`, `workers`, `ingest_queue`, `broadcast_queue`, `log_level` | same names | always written |
| (top) | `bands` | `band_filter` | the considered bands as explicit ADIF band names, in frequency order (HR-2); omitted when there is no filter (every band) |
| `[station]` | `callsign`, `locator`, `latitude`, `longitude` | `station.*` | each omitted when not set (`None`) |
| `[lookup]` | `timeout_secs`, `provider_order` | `lookup.timeout`, `lookup.provider_order` | provider names lowercase |
| `[lookup.qrz]`, `[lookup.hamqth]`, `[lookup.qrzcq]` | `username`, `password` | `lookup.<provider>` | written when the provider is enabled; the password is in clear text (Q3) |
| `[country_data]` | `path`, `url`, `max_age_days` | `country_data.*` | `path` exactly as configured (relative stays relative) |
| `[map]` | `style_url`, `style_attribution`, `tile_url`, `tile_attribution`, `fallback_color`, `recent_columns`, `recent_header` | `map.*` | unset base-map keys omitted |
| `[map.bands]` | `<band> = "<#rrggbb>"` | `map.bands` | the full table, defaults included |

**Header** (TOML comments, ignored on load):

```text
# live-qso-map 2.0.0 configuration, written 2026-10-11T14:03:00Z.
# This file contains provider passwords: keep it private.     (only when credentials are present)
```

**Validation**: the configuration is fully loaded and checked before rendering. Rendering cannot
fail for a valid `Config`.

**Invariant**: `Config::load(file = render(c)) == c` for every valid `c`, with no command-line
and no environment input (SC-004).

## Configuration file lookup

| Order | Source | Applies |
|-------|--------|---------|
| 1 | `--config <PATH>` | all platforms |
| 2 | `LQM_CONFIG` | all platforms |
| 3 | `live-qso-map.toml` in the program's folder, if the file exists | all platforms (new) |
| — | none: built-in defaults | |

## Data path base

| Platform | Relative `country_data.path` resolves against |
|----------|-----------------------------------------------|
| Linux | the current folder (unchanged) |
| Windows, macOS | the program's folder; the current folder, with a warning, if it cannot be determined |

## Release artifact

| Field | Values |
|-------|--------|
| kind | `deb`, `image`, `zip`, `tar.gz`, `checksums` |
| target | `amd64`, `arm64`, `armhf` (deb); the three Linux target triples (tar.gz); `linux/amd64`, `linux/arm64` (image); `windows-x64`, `macos-arm64` (zip) |
| version | the release tag without `v`; equals `Cargo.toml` `version` (FR-002) |
| name | per `contracts/release-artifacts.md` |
| checksum | SHA-256, listed in `SHA256SUMS` (not for the image, which is addressed by digest) |

## Service account (Debian)

| Attribute | Value |
|-----------|-------|
| name / group | `live-qso-map` / `live-qso-map` |
| kind | system account (UID and GID in the system range) |
| home | `/var/lib/live-qso-map` (not created by `adduser`; created by the package) |
| shell | `/usr/sbin/nologin` |
| GECOS | `live-qso-map service`, set with `usermod -c` right after `adduser`: the marker of an account this package created |

## Package life cycle (Debian)

| Action | Service | `/usr/bin/live-qso-map` | `/etc/live-qso-map/` | `/var/lib/live-qso-map/` | Account |
|--------|---------|------------------------|----------------------|--------------------------|---------|
| install (first) | registered, disabled, stopped | installed | `config.toml` 0640 root:live-qso-map | created, owned by the account, 0750 | created if missing |
| install (account exists) | as above | installed | as above | as above | reused unchanged |
| upgrade | restarted if running; enabled/disabled state kept | replaced | kept (dpkg asks if changed upstream and locally) | kept | kept |
| remove | stopped, disabled | removed | kept | kept | kept |
| purge (after remove or directly) | stopped if running | removed | deleted | deleted | deleted if GECOS is the marker; group deleted if empty |
| reinstall after remove | as before removal | installed | kept | kept | reused |

## Windows error pause

| Exit | Standard input interactive | Behaviour |
|------|----------------------------|-----------|
| success | any | exit at once |
| error | yes | print the error, then `Press Enter to close this window.`, wait for one line, exit with failure |
| error | no (redirected, piped) | print the error, exit with failure at once |

Applies only on Windows; other platforms always exit at once.
