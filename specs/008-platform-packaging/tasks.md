---
description: "Task list for 008-platform-packaging"
---

# Tasks: Platform Packages, Friendlier Command Line and Configuration Export

**Input**: Design documents from `/specs/008-platform-packaging/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation
it covers, run it, and confirm it fails first. Use `cargo test`. Packaging is tested by
`scripts/test-deb.sh` and by smoke steps in the pipeline. Those run only on release tags
(Principle V); verify them by pushing a tag to a fork.

**Conventions** (as in features 001–007, plus):

- **Errors**: the crate `Error`/`Result` (`docs/rust-guidelines.md`). The new variant is
  `Error::ConfigWrite { path, source }`.
- **No new lint suppressions**: a project hook rejects inline `allow` attributes.
  - Unit tests live in `#[cfg(test)] mod tests` of the same file, covered by the crate's existing
    test settings.
  - New integration tests in `tests/` return `Result<(), Box<dyn std::error::Error>>` and use `?`,
    never `unwrap`/`expect`/`panic!`. They need no attribute.
- **Platform code**:
  - `cfg(windows)` for the pause;
  - `cfg(any(windows, target_os = "macos"))` for the data-path base;
  - `cfg(unix)` for file modes.

  Each needs a path on every target (Principle V). Decisions live in plain functions that are
  tested on every host.
- **No local cross-compiling or packaging** (Principle V): Debian packages, images and archives
  are built only by `.circleci/config.yml` on tags.
- **Commits**: one commit per phase, signed, with no co-author trailers. Leave the operator's
  in-progress `Cargo.toml` edit to T001, which owns that line.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US5 from spec.md
- **[R:TIER]**: risk; unmarked tasks are GREEN

**Story order**: US2 (configuration export, P1) comes before US1 (Debian package, P1), because
the package's configuration-sync test (T025) uses the export renderer. US3–US5 follow in priority
order.

---

## Phase 1: Setup

- [X] T001 In `Cargo.toml`, set the clap line to
  `clap = { version = "4.6", features = ["derive", "wrap_help", "unicode"] }`. This replaces the
  operator's in-progress `["derive", ]` edit. Keep the default features, which include `color`,
  `suggestions` and `error-context`. Run `cargo build` and commit `Cargo.lock`.
- [X] T002 [P] Create stub modules so later tests compile, and register both in `src/lib.rs`:
  - `src/config_export.rs`: `pub fn render(config: &Config, version: &str, now_utc: &str) -> String`
    and `pub fn write(path: &Path, text: &str, stdout: &mut dyn Write) -> Result<()>`. The stubs
    return an empty string and `Ok(())`; no `todo!()`;
  - `src/console.rs`: `pub fn should_wait(failed: bool, interactive: bool, windows: bool) -> bool`,
    returning `false`.

---

## Phase 2: Foundational (blocking prerequisites)

- [X] T003 [P] Write failing tests in `src/errors.rs` for `Error::ConfigWrite { path: PathBuf,
  source: std::io::Error }`:
  - `Display` is `cannot write configuration file <path>: <io error>`;
  - `source()` returns the I/O error.
- [X] T004 Add `Error::ConfigWrite` to `src/errors.rs` to make T003 pass.
- [X] T005 [P] Write `scripts/test-check-version.sh`, a POSIX `sh` test for
  `scripts/check-version.sh <tag> [Cargo.toml]`. Using a temporary `Cargo.toml` with
  `version = "2.0.0"` under `[package]` (and another `version` under `[dependencies]` to catch a
  naive match), it checks:
  - `v2.0.0` → exit 0;
  - `v2.0.1` → exit 1, and the output names both `2.0.1` and `2.0.0`;
  - `2.0.0` (no `v`) → exit 1;
  - an empty tag → exit 1.

  It fails until T006 exists. Check it with `sh -n`.
- [X] T006 [R:YELLOW] Write `scripts/check-version.sh` (POSIX `sh`, `sed`/`grep` on the
  `version` line of the `[package]` table only), so T005 passes. Then, in
  `.circleci/config.yml`, restructure the tag-only part of the workflow
  (contracts/release-artifacts.md):
  - a `check-version` job runs `sh scripts/test-check-version.sh` and then
    `sh scripts/check-version.sh "$CIRCLE_TAG"`. Every build job requires it.
  - rename the build outputs to `live-qso-map-<X.Y.Z>-<target>`, and pack each Linux binary
    (mode 755) with `LICENSE.md` into `live-qso-map-<X.Y.Z>-<target>.tar.gz` (FR-004, R13); no
    bare binary is attached;
  - replace `store-artifacts` with a `collect` step inside `publish-github-release`, which writes
    `SHA256SUMS` (`sha256sum *`) over the attached files.

  Validate with the CircleCI `buildConfig` API (as done for feature 007's pipeline update).

**Checkpoint**: the error variant exists and the release workflow has its gate.

---

## Phase 3: User Story 2 - Generate a configuration file from the current settings (Priority: P1) 🎯 MVP

**Goal**: `--save-config` and `--dump-config` write the effective configuration (FR-020–FR-025).
**Independent Test**: quickstart §2. The saved file loads back to the identical configuration;
`--save-config` exits without opening ports.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T007 [P] [US2] Unit tests for `render` in `src/config_export.rs` (data-model.md "Exported
  configuration", contracts/configuration.md):
  - **Header**: line 1 is `# live-qso-map 2.0.0 configuration, written 2026-10-11T14:03:00Z.`,
    with version and time injected. The password line is present only when a provider has
    credentials.
  - **Defaults**: every top-level key, plus `[lookup]`, `[country_data]`, `[map]` and the full
    `[map.bands]` table.
  - **Omitted when unset**: `[station]` keys and the base-map keys.
  - **Band filter**: `bands` lists the considered bands in frequency order (`160m` before `80m`,
    never alphabetical), and is absent with no filter.
  - **Passwords**: written in clear text for each enabled provider; disabled providers have no
    section.
  - **Kept as configured**: `country_data.path` exactly as configured, a relative path included.
  - **Format**: `toml::from_str` accepts the output.
- [X] T008 [P] [US2] Round-trip unit tests in `src/config_export.rs`. For each case, build `x` with
  `Cli::try_parse_from`, an injected env closure and file text, then assert that
  `Config::load(empty cli, no env, Some(render(load(x))))` equals `load(x)` (SC-004). Cases:
  - defaults;
  - station with locator;
  - station with coordinates;
  - all three providers, with a custom `provider_order` and timeout;
  - `--bands hf,2m,-60m`;
  - raster tiles with attribution;
  - a custom style;
  - custom `recent_columns` with `recent_header = false`;
  - band colour overrides;
  - every top-level number changed;
  - values from env overriding the file.
- [X] T009 [P] [US2] Unit tests for `write` in `src/config_export.rs`, using a temporary folder
  under `std::env::temp_dir()` with a unique name:
  - **Standard output**: `-` writes the exact text to the `stdout` sink and creates no file.
  - **New file**: written with the exact text.
  - **Existing file**: replaced, and no temporary file is left in the folder.
  - **Missing folder**: `Error::ConfigWrite` naming the path, and nothing created.
  - **Unix** (`#[cfg(unix)]`): a new file has mode `0o600`; replacing an existing `0o640` file
    keeps `0o640`, and replacing a `0o644` file keeps `0o644` (FR-022). Owner and group are kept
    as well (same user in the test).
- [X] T010 [P] [US2] Unit tests in `src/config.rs` for the new options (contracts/command-line.md):
  - **Parsing**: `--save-config a.toml` and `--dump-config -` parse.
  - **Conflict**: both together fail with `ErrorKind::ArgumentConflict`.
  - **No variable**: `Config::load` ignores `LQM_SAVE_CONFIG` / `LQM_DUMP_CONFIG`.
  - **Decision**: `Cli::export_action()` returns `None`, `Some(Save(path))` or `Some(Dump(path))`.
- [X] T011 [P] [US2] Integration tests in `tests/config_export.rs`. They run
  `env!("CARGO_BIN_EXE_live-qso-map")` with `std::process::Command`, using `Result` + `?` and no
  unwrap:
  - **Save**: `--station-callsign IS0MZK --bands hf-contest --save-config <tmp>/a.toml` exits 0
    within 5 s. stderr says `configuration written to …`, and the file contains
    `callsign = "IS0MZK"`.
  - **Standard output**: `--save-config -` prints the TOML on stdout.
  - **Invalid configuration**: `--bands nope --save-config <tmp>/b.toml` exits 1, and `b.toml`
    doesn't exist.
  - **Bad folder**: `--save-config <tmp>/missing/c.toml` exits 1, and stderr contains
    `cannot write configuration file`.
  - **Conflict**: `--save-config x --dump-config y` exits 2.
  - **Passwords**: `LQM_QRZ_USERNAME=u LQM_QRZ_PASSWORD=p` in the child environment, with
    `--save-config <tmp>/d.toml`, makes stderr contain `contains provider passwords`.

### Implementation for User Story 2

- [X] T012 [US2] In `src/config.rs`:
  - derive `Serialize` on `FileConfig`, `FileStation`, `FileLookup`, `FileCredentials`,
    `FileCountryData` and `FileMap`, with `skip_serializing_if = "Option::is_none"` and empty-map
    skips;
  - implement `From<&Config> for FileConfig` per data-model.md: `BandSet` → explicit list in
    frequency order via the existing band order, timeout → `timeout_secs`, providers → lowercase
    names, `Secret::expose()` for passwords;
  - keep field order as in contracts/configuration.md.
- [X] T013 [US2] Implement `render` in `src/config_export.rs` (header + `toml::to_string_pretty`
  of `FileConfig::from(config)`), so T007–T008 pass.
- [X] T014 [US2] [R:YELLOW] Implement `write` in `src/config_export.rs`, so T009 passes:
  - **Standard output**: `-` goes to the sink.
  - **Temporary file**: `<name>.tmp-<pid>` in the target's folder, created with
    `OpenOptions::new().write(true).create_new(true)`. On Unix, `.mode(0o600)` comes from
    `std::os::unix::fs::OpenOptionsExt`.
  - **Existing target** (Unix): before the rename, copy its permissions with
    `fs::set_permissions(tmp, meta.permissions())`, and its owner and group with
    `std::os::unix::fs::chown(tmp, Some(meta.uid()), Some(meta.gid()))`, ignoring a `chown`
    error (a writer that isn't root keeps its own ownership).
  - **Write**: write all, `sync_all`, then `fs::rename` over the target.
  - **Failure**: remove the temporary file, then return `Error::ConfigWrite`.
- [X] T015 [US2] In `src/config.rs`, add to `Cli`:
  - `--save-config <PATH>` and `--dump-config <PATH>` (`PathBuf`, `conflicts_with` each other);
  - `pub enum ExportAction { Save(PathBuf), Dump(PathBuf) }` and `Cli::export_action()`.

  Neither option is read from the environment, so T010 passes.
- [X] T016 [US2] In `src/main.rs`, after `Config::load` succeeds and before logging and the
  runtime:
  - on an export action, call `render(&config, env!("CARGO_PKG_VERSION"), <now UTC
    ISO 8601 with Z>)`, using the crate's existing UTC formatting helper (`history.rs`), and then
    `write`;
  - on success, print `configuration written to <path>` to stderr, except for `-`, followed by
    the password note when credentials exist;
  - on error, print it and exit 1;
  - `Save` returns `ExitCode::SUCCESS` here, and `Dump` continues.

  This makes T011 pass.
- [X] T017 [P] [US2] In `README.md`, under Configuration, document `--save-config` and
  `--dump-config` with the "generate from a clean installation" example and the passwords note.

**Checkpoint**: export works on every host. Run quickstart §2.

---

## Phase 4: User Story 1 - Install and run as a service on Debian or Ubuntu (Priority: P1)

**Goal**: one `.deb` per Linux architecture with the account, conffile and disabled service
(FR-005–FR-014).
**Independent Test**: `scripts/test-deb.sh` in the four distribution containers (CI, tags), then
quickstart §4 on a real machine.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T018 [P] [US1] [R:YELLOW] Write `scripts/test-deb.sh <path-to-deb>`, a POSIX `sh` script run
  as root inside a container. It follows contracts/debian-package.md "Life-cycle test":
  - **After install**:
    - mode, owner and group of `/usr/bin/live-qso-map`, `/etc/live-qso-map/config.toml` (0640
      root:live-qso-map) and `/var/lib/live-qso-map` (0750 live-qso-map);
    - `getent passwd live-qso-map` has shell `/usr/sbin/nologin` and GECOS
      `live-qso-map service`;
    - `systemctl is-enabled live-qso-map` is not `enabled`;
    - `live-qso-map --version` prints the version.
  - **Upgrade**: append a comment to the config, install again with
    `dpkg -i --force-confold`; the comment is still there, and `systemctl is-enabled` is still
    not `enabled`.
  - **Save over the conffile**: `live-qso-map --config /etc/live-qso-map/config.toml
    --save-config /etc/live-qso-map/config.toml` as root; the file is still `0640
    root:live-qso-map`.
  - **Remove**: the binary is gone; `/etc/live-qso-map`, `/var/lib/live-qso-map` and the account
    remain; `systemctl is-enabled` is not `enabled`.
  - **Install, then purge**: `/etc/live-qso-map`, `/var/lib/live-qso-map`, the account and the
    group are gone.
  - **Foreign account**: create a `live-qso-map` account with GECOS `someone else`, install,
    purge; the account still exists.

  Each failure prints the step and the check, then exits 1. Check the script with `sh -n`, and
  with `shellcheck` when it is available.
- [X] T019 [P] [US1] Unit tests in `src/config_export.rs` for the shipped configuration
  (`include_str!("../deploy/config.example.toml")`) (R5, FR-010):
  - every key that `render(&Config::default())` writes appears in the example, commented
    (`# key =`) or not, under the same section;
  - the example with every `# <key> = ` line uncommented loads without error through
    `Config::load`;
  - the example's uncommented `country_data.path` is `/var/lib/live-qso-map/cty.dat`.

### Implementation for User Story 1

- [X] T020 [US1] Update `deploy/config.example.toml`, so T019 passes:
  - a header saying it is `/etc/live-qso-map/config.toml`, the service runs as `live-qso-map`,
    and how to regenerate it with `--save-config`;
  - `path = "/var/lib/live-qso-map/cty.dat"` uncommented under `[country_data]`;
  - every setting the export writes is present.
- [X] T021 [US1] [R:YELLOW] Run `git mv deploy/live-qso-map.service
  deploy/debian/live-qso-map.service` and rewrite it per contracts/debian-package.md "Service
  unit":
  - `User`/`Group=live-qso-map`, `/usr/bin`, `--config /etc/live-qso-map/config.toml`;
  - `StateDirectory=live-qso-map` with `StateDirectoryMode=0750`;
  - drop `DynamicUser` and the `EnvironmentFile`/`LQM_CTY_PATH` lines; keep every hardening
    line.
- [X] T022 [US1] [R:RED] Write `deploy/debian/postinst` (`#!/bin/sh`, `set -e`) per
  contracts/debian-package.md "Maintainer scripts". On `configure`:
  1. if `getent passwd live-qso-map` fails, run `adduser --system --group --home
     /var/lib/live-qso-map --no-create-home --shell /usr/sbin/nologin live-qso-map`, then
     `usermod -c "live-qso-map service" live-qso-map` (no `--gecos`/`--comment`, which differ
     between `adduser` releases);
  2. create `/var/lib/live-qso-map` (0750, live-qso-map:live-qso-map) if missing;
  3. set the config owner and mode unless `dpkg-statoverride --list` has an entry;
  4. `#DEBHELPER#`;
  5. upgrade (`$2` not empty): `deb-systemd-invoke try-restart live-qso-map.service || true`;
  6. first install: print the next steps (edit the file or use `--save-config`, then
     `systemctl enable --now live-qso-map`).

  It must be idempotent. Make the file executable.
- [X] T023 [US1] [R:RED] Write `deploy/debian/postrm` (`#!/bin/sh`, `set -e`), with `#DEBHELPER#`
  first. On `purge`:
  1. `rm -rf /etc/live-qso-map /var/lib/live-qso-map`;
  2. if `getent passwd live-qso-map | cut -d: -f5` is exactly `live-qso-map service`, run
     `deluser --system --quiet live-qso-map`;
  3. if the group exists, run `delgroup --system --only-if-empty --quiet live-qso-map || true`.

  `remove` does nothing beyond `#DEBHELPER#`. Make the file executable.
- [X] T024 [US1] In `Cargo.toml`, add `[package.metadata.deb]` (contracts/debian-package.md
  "Control fields"):
  - `section = "hamradio"`, `priority = "optional"`, `depends = "adduser"`;
  - `extended-description-file` (the README's first paragraph, or inline text);
  - `maintainer-scripts = "deploy/debian/"`;
  - `systemd-units = { unit-scripts = "deploy/debian/", enable = false, start = false,
    restart-after-upgrade = true }`;
  - `assets`:
    - the binary from `target/release/live-qso-map` to `usr/bin/` (cargo-deb rewrites the path
      for `--target`), mode 755;
    - `deploy/config.example.toml` to `etc/live-qso-map/config.toml`, mode 640;
    - `README.md` to `usr/share/doc/live-qso-map/`, mode 644;
    - `LICENSE.md` to `usr/share/doc/live-qso-map/copyright`, mode 644.
- [X] T025 [US1] [R:YELLOW] In `.circleci/config.yml`, add two tag-only jobs:
  - `package-deb`, parameterised by target (requires `build-linux-<target>`): install cargo-deb
    3.8.0 (`cargo install cargo-deb --version 3.8.0 --locked`), put the workspace binary at
    `target/<target>/release/live-qso-map`, run
    `cargo deb --target <target> --no-build --no-strip`, then check the result:
    `dpkg-deb --field <deb> Architecture` is `amd64`, `arm64` or `armhf` as expected for the
    target, and `dpkg-deb --contents <deb>` lists `./usr/bin/live-qso-map`,
    `./etc/live-qso-map/config.toml` and the unit file. Persist the `.deb`;
  - `test-deb`, matrix over images `debian:12`, `debian:13`, `ubuntu:22.04`, `ubuntu:24.04`
    (requires `package-deb` for `x86_64-unknown-linux-musl`): runs
    `apt-get update && apt-get install -y adduser systemd` and then
    `sh scripts/test-deb.sh /workspace/live-qso-map_*_amd64.deb`.

  `publish-github-release` requires both. Validate the config with the CircleCI API.
- [ ] T026 [US1] Verify the generated maintainer scripts: on a fork, push a tag, download the
  `amd64` `.deb`, and check
  `dpkg-deb -e live-qso-map_*_amd64.deb /tmp/ctl && cat /tmp/ctl/postinst /tmp/ctl/postrm`. The
  systemd fragments must not enable or start the unit (R4). Fix the metadata if they do.
- [X] T027 [P] [US1] In `README.md`, add a "Debian and Ubuntu" install section:
  - download the `.deb` for your architecture, `sudo apt install ./live-qso-map_…deb`;
  - edit `/etc/live-qso-map/config.toml`, or generate it with `sudo live-qso-map … --save-config
    /etc/live-qso-map/config.toml` (the file keeps its owner and mode);
  - `sudo systemctl enable --now live-qso-map`;
  - remove versus purge.

  Replace the old systemd paragraph.

**Checkpoint**: the package life cycle passes in all four distributions (CI on a fork tag).

---

## Phase 5: User Story 3 - Run the map on Windows or macOS by double-clicking (Priority: P2)

**Goal**: archives that run from their folder (FR-017–FR-019).
**Independent Test**: quickstart §3 on any host, then §6 and §7 on Windows and macOS.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T028 [P] [US3] Unit tests in `src/config.rs` for
  `config_path(cli, env, exe_dir: Option<&Path>, exists: &dyn Fn(&Path) -> bool)`
  (contracts/configuration.md "Configuration file lookup"):
  - `--config` wins over `LQM_CONFIG`, which wins over the file next to the program;
  - `exe_dir/live-qso-map.toml` is used only when `exists` says so;
  - `exe_dir = None` gives `None`.
- [X] T029 [P] [US3] Unit tests in `src/config.rs` for
  `resolve_data_path(path: &Path, exe_dir: Option<&Path>, program_folder_base: bool) -> PathBuf`:
  - with the base on, a relative path joins `exe_dir`, an absolute path is unchanged, and
    `exe_dir = None` leaves it unchanged;
  - with the base off, it is always unchanged;
  - a `cfg` test asserts that `program_folder_base()` is true exactly on Windows and macOS.
- [X] T030 [P] [US3] Unit tests in `src/console.rs` for
  `should_wait(failed, interactive, windows)`. Only `(true, true, true)` waits; the other seven
  combinations do not (data-model.md "Windows error pause").

### Implementation for User Story 3

- [X] T031 [US3] In `src/config.rs`:
  - change `config_path` to the T028 signature;
  - add `resolve_data_path` and `pub fn program_folder_base() -> bool`
    (`cfg!(any(windows, target_os = "macos"))`).

  Update the existing `config_path` tests and the caller in `src/main.rs`, which passes
  `std::env::current_exe()` parent and `Path::is_file`. When `current_exe` fails, log a warning
  and pass `None`.
- [X] T032 [US3] In `src/main.rs`, after the export step (T016), so the export keeps relative
  paths, set `config.country_data.path = resolve_data_path(..)`.
- [X] T033 [US3] Implement `should_wait` in `src/console.rs` and use it in `src/main.rs`:
  - on every failure exit path, compute
    `should_wait(true, std::io::stdin().is_terminal(), cfg!(windows))`;
  - when true, print `Press Enter to close this window.` to stderr and read one line from stdin,
    ignoring read errors.
- [X] T034 [P] [US3] Write `deploy/windows/README.txt` (CRLF line endings) and
  `deploy/macos/README.txt` (LF) with the five points of contracts/release-artifacts.md "Read-me
  files in the zips". The macOS file adds the Open Anyway step.
- [X] T035 [US3] [R:YELLOW] In `.circleci/config.yml`, add a tag-only `package-archives` job
  (requires `build-windows`, `build-macos`). It zips each binary with `LICENSE.md` and its
  `README.txt` into `live-qso-map-<X.Y.Z>-windows-x64.zip` (binary named `live-qso-map.exe`) and
  `live-qso-map-<X.Y.Z>-macos-arm64.zip` (binary named `live-qso-map`, `chmod 755` before
  zipping), then persists them. `publish-github-release` requires it.
- [X] T036 [P] [US3] In `README.md`, add "Windows" and "macOS" install sections: download,
  extract, double-click, `live-qso-map.toml` next to the program, and the macOS Open Anyway
  step.

**Checkpoint**: quickstart §3 passes on the development host; §6 and §7 pass with archives from a
fork tag.

---

## Phase 6: User Story 4 - Run the map as a container (Priority: P2)

**Goal**: a Debian Trixie image on Docker Hub for amd64 and arm64 (FR-015, FR-016).
**Independent Test**: quickstart §5 on x86-64 and ARM64 hosts.

### Tests for User Story 4 (write first, confirm they FAIL)

- [X] T037 [P] [US4] Write `scripts/test-image.sh <image>`, a POSIX `sh` script that runs:
  - `docker run --rm <image> --version`, which prints the version;
  - `docker run --rm --entrypoint id <image> -u`, which prints `10001`;
  - a detached run with `-v` of a temporary `config.toml` setting `http_port = 18080` and
    `--save-config -` as the argument, which prints `http_port = 18080` (the entry point passes
    `--config` when the file exists);
  - the same run with the file setting `[country_data] path = "/var/lib/live-qso-map/x.dat"`
    prints that path, so no image variable overrides the mounted file;
  - a run without a mounted file and `--save-config -` prints
    `path = "/var/lib/live-qso-map/cty.dat"`.

  It exits 1 with the step name on any mismatch. Check it with `sh -n`.

### Implementation for User Story 4

- [X] T038 [US4] Write `deploy/docker/entrypoint.sh`. If `/etc/live-qso-map/config.toml` exists,
  it runs `exec /usr/bin/live-qso-map --config /etc/live-qso-map/config.toml "$@"`; otherwise
  `exec /usr/bin/live-qso-map --cty-path /var/lib/live-qso-map/cty.dat "$@"`.
- [X] T039 [US4] [R:YELLOW] Write `deploy/docker/Dockerfile.release` per research R6 and
  contracts/release-artifacts.md "Container image":
  - a `FROM --platform=$BUILDPLATFORM debian:trixie-slim AS prep` stage installs
    `ca-certificates` and creates `/etc/live-qso-map` and `/var/lib/live-qso-map` (owner
    10001:10001, 0750);
  - the final `FROM debian:trixie-slim` copies the CA bundle, the folders, the entry point
    (`COPY --chmod=755`), `LICENSE.md` and `dist/linux/${TARGETARCH}/live-qso-map`
    (`COPY --chmod=755`) to `/usr/bin/`;
  - then `USER 10001:10001`, `VOLUME /var/lib/live-qso-map`, `EXPOSE 12060/udp 8080/tcp` and the
    OCI labels. No `ENV LQM_CTY_PATH` (it would override a mounted file's path).

  No `RUN` in the final stage.
- [X] T040 [US4] Update the local `Dockerfile` (build from source on any host):
  - build stage `rust:${RUST_VERSION}-trixie`, keeping the `RUST_VERSION` pin in step with CI;
  - runtime stage the same as T039's final stage, but copying the binary built in the image;
  - remove distroless.

  Keep the `RUST_VERSION` comment referring to `docs/rust-guidelines.md` §2.2.
- [X] T041 [US4] [R:YELLOW] In `.circleci/config.yml`, add a tag-only `docker-image` job on the
  `linux-machine` executor (requires `build-linux-x86_64-unknown-linux-musl` and
  `build-linux-aarch64-unknown-linux-musl`):
  1. lay out `dist/linux/amd64` and `dist/linux/arm64`;
  2. `echo "$DOCKERHUB_TOKEN" | docker login -u "$DOCKERHUB_USERNAME" --password-stdin`;
  3. `docker buildx create --use`;
  4. build `--platform linux/amd64 --load -t lqm-test` and run `sh scripts/test-image.sh
     lqm-test`;
  5. `docker buildx build --platform linux/amd64,linux/arm64 -f deploy/docker/Dockerfile.release
     -t sardylan/live-qso-map:<X.Y.Z>-rc --push .` (only the internal `-rc` tag).

  `publish-github-release` requires it. Validate the config with the CircleCI API.
- [X] T042 [US4] [R:YELLOW] In `.circleci/config.yml`, add a tag-only `docker-promote` job that
  requires `publish-github-release`. It logs in to Docker Hub and runs
  `docker buildx imagetools create -t sardylan/live-qso-map:<X.Y.Z> -t sardylan/live-qso-map:latest
  sardylan/live-qso-map:<X.Y.Z>-rc` (re-tag, no rebuild), so the public tags appear only after
  the release is published (SC-008).
- [X] T043 [P] [US4] In `README.md`, rewrite the Container section around
  `sardylan/live-qso-map:<version>`: ports, the config mount at
  `/etc/live-qso-map/config.toml`, the `lqm-data` volume at `/var/lib/live-qso-map`, and the
  non-root user. Mention the local `docker build .`.

**Checkpoint**: the image passes `scripts/test-image.sh` in CI and quickstart §5 on ARM64.

---

## Phase 7: User Story 5 - A command line that helps (Priority: P3)

**Goal**: coloured, wrapped help with short and long forms, and typo suggestions (FR-026,
FR-027).
**Independent Test**: quickstart §1.

### Tests for User Story 5 (write first, confirm they FAIL)

- [X] T044 [P] [US5] Unit tests in `src/config.rs` using `Cli::command()`:
  - `.term_width(80).render_long_help()` has no line longer than 80 characters (count chars,
    not bytes);
  - `render_help()` (`-h`) contains no `Environment:`;
  - `render_long_help()` contains `Environment: LQM_UDP_PORT. Default: 12060.`;
  - for every long option of `Cli::command().get_arguments()` longer than 4 characters, swap
    two adjacent letters in the middle of the name (`--udp-port` → `--udp-prot`); parsing fails,
    and the rendered error contains the real `--<name>` (SC-007);
  - `Cli::command().debug_assert()` passes.
- [X] T045 [P] [US5] Integration test in `tests/cli_help.rs`, running the binary with
  `std::process::Command` and using `Result` + `?`:
  - `--help` with `NO_COLOR=1` and piped output contains no `\x1b[`;
  - `-h` exits 0;
  - `--udp-prot 1` exits 2 with `--udp-port` on stderr.

### Implementation for User Story 5

- [X] T046 [US5] In `src/config.rs`, rewrite the doc comment of every `Cli` option as a one-line
  summary, a blank line, then `Environment: LQM_<NAME>. Default: <value>.`, leaving out the
  parts an option does not have. Remove the old inline `[env: …] [default: …]` text.
  Add `#[command(styles = …)]` with coloured `header`, `usage`, `literal` and `placeholder`
  styles (`clap::builder::styling`), so T044–T045 pass.

**Checkpoint**: quickstart §1 passes.

---

## Phase 8: Polish & Quality Gates

- [ ] T047 `cargo fmt --check` passes
- [ ] T048 `cargo clippy --all-targets -- -D warnings` passes, with no new inline lint
  suppression anywhere
- [ ] T049 `cargo test` passes; `node --test web/` still passes
- [ ] T050 Platform-specific code is `cfg`-gated with a path for every configured target
  (Principle V). Review `src/console.rs`, `src/config.rs` and `src/config_export.rs` for
  `cfg(windows)`, `cfg(any(windows, target_os = "macos"))` and `cfg(unix)`, each with a
  fallback. The Windows and macOS builds on the fork tag (T052) compile them.
- [ ] T051 Review: no new crate (only clap features). No unjustified complexity; plan Complexity
  Tracking is still empty.
- [ ] T052 [R:YELLOW] Push a test tag to a fork with `DOCKERHUB_*` pointing at a test repository.
  The whole tag workflow must pass. Then check each artifact against
  contracts/release-artifacts.md and run `sha256sum -c SHA256SUMS`. Also push a mismatching tag
  and confirm that `check-version` stops the workflow before any build (FR-002).
- [ ] T053 Run quickstart §1–§8 and record the results in `checklists/verification.md`,
  following feature 007's verification note.
- [ ] T054 [P] Add the three CircleCI variables to `README.md`'s Development section:
  `GITHUB_TOKEN`, `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`
  (contracts/release-artifacts.md).
- [ ] T055 Close the feature:
  1. set spec `Status: Complete`;
  2. move TR-010 and TR-011 to `accepted` in `docs/tradeoffs.md`;
  3. merge `feature/008-platform-packaging` into `main`;
  4. delete the branch locally and on the remote (constitution 1.6.0, Development Workflow).

## Dependencies & Execution Order

- **Setup (T001–T002)** → **Foundational (T003–T006)** → stories.
- **US2 (T007–T017)** first; it is the MVP and blocks US1's sync test (T019 uses `render`).
- **US1 (T018–T027)** after US2. Its CI jobs (T025) need T006's renamed outputs.
- **US3 (T028–T036)** after US2, because T032 runs after T016's export step in `main.rs`. It is
  independent of US1.
- **US4 (T037–T043)** after T006. It is independent of US1–US3, apart from T041 and T042 sharing
  `.circleci/config.yml` with T025 and T035; do those one at a time.
- **US5 (T044–T046)** after T001 (clap features). It touches only the `Cli` doc comments and
  styles, and can run any time after Phase 2. Do T046 after T015, which also edits `Cli`.
- **Polish (T047–T055)** last. T052 needs every CI job, and T055 needs everything.
- **Within each story**: tests → implementation → docs.

## Parallel Opportunities

- **Phase 1**: T001 and T002 touch different files.
- **US2**: the test tasks T007–T011 are all [P]. T007–T009 share `src/config_export.rs`, so write
  them as one batch or in sequence within the file. T017 can run alongside the implementation.
- **US1**: T018 (shell) and T019 (Rust) run in parallel; T021–T023 are three separate files; T027
  is independent.
- **US3**: T028–T030 run in parallel; T034 and T036 run alongside the code.
- **US4**: T037, T038 and T043 run in parallel; T039 and T040 are separate files.
- **Across stories**: US4's files (`deploy/docker/`, `scripts/test-image.sh`) don't overlap with
  US3's (`src/console.rs`, `deploy/windows|macos/`). Only `.circleci/config.yml` and `README.md`
  are shared.

### Parallel example: User Story 2

```text
Task: "T010 [US2] Cli option tests in src/config.rs"
Task: "T011 [US2] binary integration tests in tests/config_export.rs"
Task: "T007–T009 [US2] render/round-trip/write tests in src/config_export.rs"  (one agent, one file)
```

## Implementation Strategy

1. **MVP (US2)**: Phases 1–3 give a working `--save-config` / `--dump-config` on every platform,
   useful even before any package exists. Stop and validate with quickstart §2.
2. **US1**: the Debian packages, the most common deployment. Validate with a fork tag.
3. **US3 and US4**: Windows/macOS archives and the image, in either order.
4. **US5**: the help polish.
5. **Release**: polish, then tag `v2.0.0` on `main` after the merge.
