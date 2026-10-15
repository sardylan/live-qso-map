# Contract: `config.example.toml`

Normative for `deploy/config.example.toml`. The checks in `src/config_export.rs` enforce the
points marked **[tested]**.

## Name and identity

- File name `config.example.toml`, identical bytes wherever it is shipped **[tested in the
  pipeline: `check-release-files.sh`]**.
- Never installed or copied as a live configuration by any deliverable.

## Header (illustrative wording)

```toml
# live-qso-map example configuration: every setting, commented out.
#
# Copy this file to your live configuration and uncomment only what you change:
#   Windows, macOS, Linux archive: live-qso-map.toml next to the program (read automatically)
#   Debian package:                /etc/live-qso-map/config.toml
#   Container image:               mount it at /etc/live-qso-map/config.toml
# Replace every "<…>" placeholder with your own value; a line left as a placeholder fails or
# does nothing useful.
#
# Precedence: command line > LQM_* environment variables > this file > built-in defaults.
# Every setting has an LQM_* variable and a command-line option: live-qso-map --help lists them.
# A complete file with your current values can also be written with --save-config <PATH>.
```

## Body rules

1. Every setting of the configuration file appears once, as `# key = value` **[tested]**,
   including `username` and `password` under `[lookup.qrz]`, `[lookup.hamqth]` and
   `[lookup.qrzcq]` **[tested]**.
2. No setting is active: the file unchanged loads exactly as no file **[tested]**.
3. Settings with a default show it. Platform-dependent defaults (`country_data.path`,
   `history_file`) show the plain default, with a comment giving the Debian, container, Windows
   and macOS locations.
4. Settings with no default show a `"<…>"` placeholder **[tested]**:

   ```toml
   [station]
   # callsign = "<your callsign>"
   # locator = "<your Maidenhead locator>"
   # latitude = "<decimal degrees, north positive>"    # set both latitude and longitude, or neither
   # longitude = "<decimal degrees, east positive>"

   [lookup]
   # Passwords can also come from LQM_QRZ_PASSWORD, LQM_HAMQTH_PASSWORD and LQM_QRZCQ_PASSWORD
   # (usernames: LQM_QRZ_USERNAME, LQM_HAMQTH_USERNAME, LQM_QRZCQ_USERNAME), keeping them out
   # of this file. A provider is enabled when both its username and its password are set.

   [lookup.qrz]
   # username = "<your QRZ.com username>"
   # password = "<your QRZ.com password>"

   [lookup.hamqth]
   # username = "<your HamQTH username>"
   # password = "<your HamQTH password>"

   [lookup.qrzcq]
   # username = "<your QRZCQ username>"
   # password = "<your QRZCQ password>"
   ```

5. `style_url` and `style_attribution` show their real defaults (OpenFreeMap Liberty and its
   credit, the latter as a TOML literal string) **[tested]**. Alternatives and open-ended maps
   keep labelled illustrative values: the raster-tile pair, and one or two `[map.bands]` colours.
6. Command-line-only options (`--config`, `--save-config`, `--dump-config`, `--enrich-only`) and
   the ignored `--history-size` do not appear.
7. Uncommented, with placeholders replaced by valid samples and the raster-tile alternative
   removed, the file loads **[tested]**.
