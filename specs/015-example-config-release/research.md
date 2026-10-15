# Research: Example Configuration in Every Release

No item in the Technical Context was left as NEEDS CLARIFICATION. The points below settle the
design choices the spec left to planning.

## R1. Keep `deploy/config.example.toml` as the single source

- **Decision**: The existing file keeps its name and place. Every deliverable copies it unchanged:
  the archives and zips, the Debian package (`usr/share/doc/live-qso-map/`), both Dockerfiles and
  the standalone release download. Nothing generates or rewrites it at build time.
- **Rationale**: Its name already contains "example". It differs from every live configuration
  name (`live-qso-map.toml` next to the program, `/etc/live-qso-map/config.toml` for Debian and
  the container). It is already tested (`src/config_export.rs`) and linked from the README. A
  byte-for-byte copy makes FR-008 (identical everywhere) trivially true and easy to check with
  `cmp`.
- **Alternatives considered**: Generating it from `--save-config` output at release time. That
  would put the uncommented values of the build machine in it, lose the explanatory comments, and
  still need placeholders added by hand. Rejected. Moving it to `docs/` was also rejected: links
  from README, Cargo.toml and tests would churn for no user-visible gain, because the spec's
  "with the documentation" is about where the file sits inside each deliverable.

## R2. Every line commented out; placeholders for settings without a default

- **Decision**: The uncommented `[country_data] path = "/var/lib/live-qso-map/cty.dat"` line goes
  (it moves to the Debian live file, R3). Every setting is `# key = value`. A setting with a
  default shows that default. A setting with no default shows a placeholder in one style:
  a TOML string `"<…>"` that says what goes there.

  | Setting | Example line |
  |---------|--------------|
  | `station.callsign` | `# callsign = "<your callsign>"` |
  | `station.locator` | `# locator = "<your Maidenhead locator>"` |
  | `station.latitude` / `longitude` | `# latitude = "<decimal degrees, north positive>"` (likewise for east) |
  | `lookup.<p>.username` | `# username = "<your QRZ.com username>"` (HamQTH, QRZCQ likewise) |
  | `lookup.<p>.password` | `# password = "<your QRZ.com password>"` (HamQTH, QRZCQ likewise) |

  Settings whose default depends on the platform or the installation (`country_data.path`,
  `history_file`) show the plain default (`"./cty.dat"`, not set), with a comment for the Debian,
  container, Windows and macOS locations. Alternatives that are not defaults (raster tiles
  instead of a style, a per-band colour) keep illustrative values, labelled as examples in their
  comment. `bands` shows its real default, `["hf-contest"]`, and the comment keeps `["all"]` as
  another example.
- **Rationale**: With every line commented, the example used unchanged is the same as no file
  (FR-003, SC-003). A string placeholder on a number setting (latitude, longitude) or on a
  validated one (locator) fails loudly at load if someone uncomments it without editing. That is
  better than a plausible real value, such as the author's callsign, quietly placing someone
  else's QTH on the map. A credential placeholder loads, and the provider then fails to log in,
  which the program already reports (spec edge case).
- **Alternatives considered**: Empty strings (today). They are not recognisable as "fill me in"
  (FR-004). Real sample values (IS0MZK, JM49NH) were also rejected: they look valid and would be
  used silently.

## R3. Debian: a separate short live configuration

- **Decision**: New file `deploy/debian/config.toml`, installed by cargo-deb as
  `etc/live-qso-map/config.toml` (0640; postinst keeps setting `root:live-qso-map`). It holds only
  `[country_data] path = "/var/lib/live-qso-map/cty.dat"`, a header comment that points to
  `/usr/share/doc/live-qso-map/config.example.toml`, and the `--save-config` hint. The example
  becomes a separate asset, `usr/share/doc/live-qso-map/config.example.toml` (0644). The test
  `example_sets_the_packaged_data_path` moves to the Debian file, and a new test checks that this
  file loads.
- **Rationale**: Spec clarifications 2 and 3. cargo-deb marks every file under `etc/` as a
  conffile, so the live file follows dpkg's standard conffile flow, kept on purpose (FR-010):
  - never edited → replaced, no question;
  - edited, packaged file unchanged → kept, no question (`test-deb.sh` already checks this);
  - edited and packaged file changed → dpkg asks (keep / install the new one / show the diff).

  Upgrading from 3.x to this version changes the packaged file, so operators who edited theirs
  get the question once. Every answer keeps the same country data path, so the service behaves
  the same. The postinst only sets owner and mode (unless `dpkg-statoverride` overrides them) and
  never rewrites the content.

  The example is installed under `usr/share/doc/`, outside `etc/`, so cargo-deb does not mark it
  as a conffile. dpkg overwrites it on every install and upgrade with no question (FR-010b).
  `package-deb` asserts that the package's `conffiles` control file lists exactly
  `/etc/live-qso-map/config.toml`.
- **Alternatives considered**: See TR-023 (copy of the example with lines uncommented, or the
  path passed by the systemd unit).

## R4. Where the example sits in each deliverable

| Deliverable | Location of `config.example.toml` | Next to |
|-------------|-----------------------------------|---------|
| Linux `live-qso-map-<v>-<target>.tar.gz` (3) | archive root | `live-qso-map`, `LICENSE.md` |
| `live-qso-map-<v>-windows-x64.zip` | archive root | `live-qso-map.exe`, `LICENSE.md`, `README.txt` |
| `live-qso-map-<v>-macos-arm64.zip` | archive root | `live-qso-map`, `LICENSE.md`, `README.txt` |
| `live-qso-map_<v>-1_<arch>.deb` (3) | `/usr/share/doc/live-qso-map/` | `README.md`, `copyright` |
| Container image (both Dockerfiles) | `/usr/share/doc/live-qso-map/` | `LICENSE.md` |
| GitHub release | standalone download `config.example.toml` | the other release files, listed in `SHA256SUMS` |

- **Decision**: The Linux archives get no README (none is requested). The standalone copy is put
  into `ws/release/` by the `package-archives` job, which already writes the release files that
  do not depend on a target. `publish-github-release` therefore uploads it and lists it in
  `SHA256SUMS` with no change to the upload command.
- **Rationale**: Each location is next to the documentation that deliverable already ships
  (FR-007). The standalone file is unversioned, as the user asked. Release assets are per tag, so
  names do not clash.

## R5. Checks: content in `cargo test`, presence in the pipeline

- **Decision**:
  1. `cargo test` (`src/config_export.rs`):
     - The example lists every exported setting, now with credentials set for all three
       providers, so the `lookup.<p>.username` and `password` keys are required (FR-011).
     - Every line that sets something is commented out: parsing the example unchanged gives an
       empty table, and loading it gives `Config::default()`-equivalent results (FR-003).
     - Each placeholder key holds a `"<…>"` string (FR-004).
     - Uncommented, with placeholders replaced by valid sample values from a small table in the
       test, the example loads (the existing `uncommented_example_loads`, extended).
     - The Debian live file loads and sets the packaged path.
  2. A new POSIX `sh` script, `scripts/check-release-files.sh <release-dir> <example>`, fails
     unless:
     - the standalone `config.example.toml` exists and is identical to the source;
     - every `*.tar.gz` and `*.zip` contains an identical `config.example.toml` at its root;
     - every `*.deb` contains an identical `./usr/share/doc/live-qso-map/config.example.toml`.

     It runs in `publish-github-release` before the checksums, so nothing is published if it
     fails (FR-012), and in `rehearsal-summary`.
  3. `scripts/test-image.sh` gains a step: the image's `/usr/share/doc/live-qso-map/config.example.toml`
     exists. The image is not in `ws/release`, so it is checked where it is built, before
     `image-push`.
  4. The per-job content listings already in `package-deb` gain the doc path, so a broken
     package fails at its own job, not only at publish time.
- **Rationale**: The content rules are portable and run on every commit (Principle V). The
  packaging checks need `tar`, `unzip` and `dpkg-deb` and run only where packages exist, as
  `test-deb.sh` and `test-image.sh` do today. Checking the installed Debian file inside
  `test-deb.sh` is avoided on purpose: Debian and Ubuntu container images can exclude
  `/usr/share/doc/*` through dpkg `path-exclude`, so the file may be legitimately absent there.
  `dpkg-deb --contents` and `--fsys-tarfile` read the package itself.
- **Alternatives considered**: A Rust or Node test that unpacks the release files. Rejected:
  release files exist only in the pipeline, and a CI-only shell script matches the existing
  `scripts/test-*.sh`.

## R6. Environment variables and command-line names in the example

- **Decision**: The header keeps the precedence line and adds the naming rule: each setting has an
  `LQM_*` variable, and `--help` lists them all. The `[lookup]` section names the six credential
  variables (`LQM_QRZ_USERNAME` … `LQM_QRZCQ_PASSWORD`) as the alternative to writing passwords
  in a file (FR-005). Variables are not repeated next to every other setting.
- **Rationale**: FR-005 asks only for where secrets can come from instead. Repeating 30 variable
  names doubles the file's length and adds one more list that can drift (`--help` is generated
  from the code and cannot drift).
- **Alternatives considered**: One variable per setting comment. Rejected, as above.

## R7. Rehearse before merge

- **Decision**: The pipeline changes are proven with a release rehearsal
  (`circleci pipeline run … --param rehearsal=true`, docs/ci.md section 7) before merging. The
  rehearsal must show `config.example.toml` in `ws/release` and a passing
  `check-release-files.sh`.
- **Rationale**: LL-007. Packaging jobs run only on release tags and rehearsals, so a mistake
  would otherwise first appear while a real tag is being cut.
