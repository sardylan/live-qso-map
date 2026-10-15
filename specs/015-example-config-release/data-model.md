# Data Model: Example Configuration in Every Release

No program data changes. The entities are files.

## Example configuration file

- **Source**: `deploy/config.example.toml` (single source, research R1).
- **Published name**: `config.example.toml` in every deliverable and as a release download.
- **Content**:
  - A header comment: what the file is, how to use it (copy it to the live file name for the
    platform, uncomment, replace `<…>` placeholders), the precedence rule, the `LQM_*` and
    `--help` pointer, and the platform locations of the live file.
  - Sections in the order of the exported configuration: top level, `[station]`, `[lookup]`,
    `[lookup.qrz]`, `[lookup.hamqth]`, `[lookup.qrzcq]`, `[country_data]`, `[map]`,
    `[map.bands]`.
  - One commented line per setting (`# key = value`), and nothing uncommented.
- **Validation (cargo test)**:
  - The uncommented key set ⊇ the exported key set with station and all credentials set.
  - Parsed unchanged → empty TOML table; loaded unchanged → same `Config` as no file.
  - Placeholder keys (`station.callsign`, `station.locator`, `station.latitude`,
    `station.longitude`, `lookup.{qrz,hamqth,qrzcq}.{username,password}`) hold a string matching
    `<…>`.
  - Uncommented with placeholders replaced by samples (and the raster-tile alternative dropped)
    → loads.
- **Lifecycle**: edited by developers with any setting change; copied unchanged into a release.

## Debian live configuration file

- **Source**: `deploy/debian/config.toml` (new).
- **Installed as**: `/etc/live-qso-map/config.toml`, a dpkg conffile, mode 0640,
  `root:live-qso-map`.
- **Content**: a header comment pointing to `/usr/share/doc/live-qso-map/config.example.toml` and
  to `--save-config`, then `[country_data] path = "/var/lib/live-qso-map/cty.dat"`.
- **Validation (cargo test)**: loads; `country_data.path` is the packaged path; the derived
  history file is `/var/lib/live-qso-map/qso-history.adi`.
- **Lifecycle (dpkg)**:

  | Before upgrade | After upgrade |
  |----------------|---------------|
  | no file (first install) | new short file |
  | file never edited | replaced by the new version, no question |
  | file edited, new version ships the same file | kept, no question |
  | file edited, new version changed it (e.g. old example copy → short file) | dpkg asks: keep, install the new one, or show the difference (`--force-confold`/`--force-confnew` answer it non-interactively) |

  It is the only conffile in the package **[checked in `package-deb`]**.

## Installed example (Debian, container)

- **Installed as**: `/usr/share/doc/live-qso-map/config.example.toml`, mode 0644, an ordinary
  package file (not a conffile).
- **Lifecycle**: overwritten by every install and upgrade, with no question, even if edited.

## Release deliverable

| Deliverable | Built by job | Example at | Checked by |
|-------------|--------------|------------|------------|
| Linux `.tar.gz` ×3 | `build-linux` (archive step) | root | `check-release-files.sh` |
| Windows / macOS `.zip` | `package-archives` | root | `check-release-files.sh` |
| `.deb` ×3 | `package-deb` | `/usr/share/doc/live-qso-map/` | `package-deb` contents check, `check-release-files.sh` |
| Container image | `image-test`, `image-push` | `/usr/share/doc/live-qso-map/` | `test-image.sh` |
| Standalone `config.example.toml` | `package-archives` | release download | `check-release-files.sh` |
