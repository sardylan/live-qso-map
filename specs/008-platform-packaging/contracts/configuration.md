# Contract: Configuration Export and Locations (feature 008)

Extends `specs/001-live-qso-map/contracts/configuration.md` and its later extensions (002, 003,
006). The file format and every key are unchanged.

## Exported file

- **Format**: the configuration file format (TOML), UTF-8, `\n` line endings, ending with a
  newline.
- **Header comments**:
  1. `# live-qso-map <version> configuration, written <UTC ISO 8601 with Z>.`
  2. When any provider has credentials: `# This file contains provider passwords: keep it private.`
- **Content**: every setting with its effective value, in this order:
  1. top-level keys;
  2. `[station]`;
  3. `[lookup]`, then `[lookup.qrz]`, `[lookup.hamqth]`, `[lookup.qrzcq]`;
  4. `[country_data]`;
  5. `[map]`, then `[map.bands]`.

  See data-model.md "Exported configuration" for which unset values are omitted.
- **Passwords**: written in clear text for every enabled provider.
- **Round trip**: loading the written file alone gives the same effective configuration.

Example (defaults, station set, one provider):

```toml
# live-qso-map 2.0.0 configuration, written 2026-10-11T14:03:00Z.
# This file contains provider passwords: keep it private.
bind_address = "0.0.0.0"
udp_port = 12060
http_port = 8080
history_size = 10000
workers = 1
ingest_queue = 1024
broadcast_queue = 1024
log_level = "info"

[station]
callsign = "IS0MZK"
locator = "JM49NH"

[lookup]
timeout_secs = 5
provider_order = ["qrz", "hamqth", "qrzcq"]

[lookup.qrz]
username = "is0mzk"
password = "example-password"

[country_data]
path = "./cty.dat"
url = "https://www.country-files.com/cty/cty.dat"
max_age_days = 30

[map]
fallback_color = "#7f7f7f"
recent_columns = ["call", "band", "time", "cqzone", "country"]
recent_header = true

[map.bands]
160m = "#8c564b"
# … every band of the colour table
```

## Writing

- **Path `-`**: the text goes to standard output; nothing else is written there.
- **Any other path**:
  - the text is written to a temporary file in the same folder, which is then renamed over the
    path;
  - an existing file is replaced;
  - a missing folder is an error, and folders are not created.
- **Permissions**:
  - on Linux and macOS, a new file has mode `0600` (owner read and write) from creation;
  - on Linux and macOS, a replaced file keeps its mode, owner and group: they are copied to the
    temporary file before the rename. Ownership is copied only when the writer may set it (root);
    otherwise the writer's ownership stays;
  - on Windows, the file gets the folder's default permissions.
- **Failure**: no partial file is left at the path; the temporary file is removed.

## Configuration file lookup

1. `--config <PATH>`
2. `LQM_CONFIG`
3. `live-qso-map.toml` in the folder that holds the running program, when that file exists (new)
4. none: built-in defaults

A missing file named by 1 or 2 is an error, as today. A missing file at 3 is not.

## Relative data paths

| Platform | `country_data.path` (`--cty-path`, `LQM_CTY_PATH`, file, default `./cty.dat`) when relative |
|----------|-------------------------------------------------------------------------------------------|
| Linux | relative to the current folder (unchanged) |
| Windows, macOS | relative to the program's folder (new) |

Absolute paths are used as given on every platform.

## Packaged defaults

| Platform | Configuration file | Data |
|----------|--------------------|------|
| Debian package | `/etc/live-qso-map/config.toml` (the service passes `--config`) | `/var/lib/live-qso-map/cty.dat` (set in the shipped file) |
| Container | `/etc/live-qso-map/config.toml` (passed by the entry point when present) | `/var/lib/live-qso-map/cty.dat` (from the mounted file, or `--cty-path` passed by the entry point when no file is mounted) |
| Windows, macOS | `live-qso-map.toml` next to the program | `cty.dat` next to the program |
