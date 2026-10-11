# Implementation Plan: Platform Packages, Friendlier Command Line and Configuration Export

**Branch**: `feature/008-platform-packaging` | **Date**: 2026-10-11 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/008-platform-packaging/spec.md`

## Summary

**Release artifacts** (tags only, built by the pipeline, Principle V):
- **Debian packages**: one per Linux architecture (`amd64`, `arm64`, `armhf`), each holding the
  static musl binary, built with cargo-deb from `[package.metadata.deb]` in `Cargo.toml` (R1, R2).
  - **Account**: a dedicated `live-qso-map` account, created in `postinst` and deleted on purge
    only if the package created it (R3).
  - **Service**: a hardened systemd unit, installed disabled and stopped (R4).
  - **Configuration**: the commented example ships as an `/etc` conffile, mode `0640`
    root:live-qso-map (R5).
  - **Tests**: a CI script installs, upgrades, removes and purges the package in Debian 12/13 and
    Ubuntu 22.04/24.04 containers (R7).
- **Container image**: `debian:trixie-slim` with the pipeline binaries, for `linux/amd64` and
  `linux/arm64`, pushed to Docker Hub as `sardylan/live-qso-map`. It is tagged `<version>-rc` first
  and re-tagged to `<version>` and `latest` only after the GitHub release is published (R6, R7).
- **Windows and macOS**: one zip each, with the program, the license and a read-me (R12).
- **Every release**: a version check against the tag (R7), the static Linux binaries as
  `.tar.gz` archives with the license, and a `SHA256SUMS` file (R13).

**Program changes**:
- **Configuration export**: `--save-config <PATH>` and `--dump-config <PATH>` write the
  effective configuration (`-` = standard output). They reuse the file-format structs, now
  serialisable, and write atomically with owner-only permissions on Unix (R9).
- **Same folder as the program**: `live-qso-map.toml` next to the program is the last-resort
  configuration file. On Windows and macOS, relative data paths resolve against the program's
  folder (R10).
- **Windows**: after an error, the program waits for Enter when its input is interactive (R11).
- **Command line**: clap's `wrap_help` and `unicode` features and a coloured style. Short help
  holds one line per option; long help adds the environment variable and default (R8).

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2)
**Primary Dependencies**: none new (clap gains `wrap_help` and `unicode`; existing `toml` writes the export); CI-only release tools cargo-deb 3.8.0, cross, docker buildx, zip
**Storage**: none new; Debian `/etc/live-qso-map/` and `/var/lib/live-qso-map/`, the same paths in the container (volume), the program's folder on Windows and macOS

**Testing**: `cargo test`; `node --test web/` (unchanged); `scripts/test-deb.sh` in CI for the
packages.
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary
**Performance Goals**:
- **Debian**: from download to a running map in under 5 minutes (SC-001).
- **Windows and macOS**: from download to a running map in under 2 minutes (SC-005).
- **Save**: exits without binding any port.

**Constraints**:
- the UDP port stays closed until the operator enables the service (FR-011);
- conffiles are kept across upgrades (FR-009);
- purge leaves nothing behind (SC-002);
- new exported files are owner-only, replaced files keep their owner and mode, and the files can
  contain passwords (FR-022);
- the release is all-or-nothing (SC-008).

**Scale/Scope**:
- **Release artifacts**: 3 Debian packages, 1 multi-architecture image, 2 zips, 3 Linux `.tar.gz`
  and 1 checksum file.
- **Command line**: about 35 options.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests come before code.
  - **Export**:
    - the rendered TOML parses back;
    - round trip `load(export(load(x))) == load(x)` over combinations of command line,
      environment and file, including credentials, a band filter, a raster base map and custom
      columns;
    - an invalid configuration writes nothing;
    - `-` writes to standard output;
    - an atomic replace of an existing file;
    - mode `0600` on Unix;
    - an unwritable folder gives an error naming the path;
    - `--save-config` with `--dump-config` is rejected.
  - **Example config sync**: every exported key appears in `deploy/config.example.toml`, and the
    example with comments removed loads as the defaults.
  - **Paths**:
    - `config_path` order (`--config`, then `LQM_CONFIG`, then the file next to the program), with
      the program's folder injected for tests;
    - relative path resolution, with a `cfg` test for Windows and macOS and one for Linux.
  - **Help**:
    - the 80-column width limit;
    - `-h` without and `--help` with "Environment:";
    - the typo suggestion.
  - **Windows pause**: the decision function (error × interactive) is tested on every host. Only
    the call site is `cfg(windows)`.
  - **Packages**: `scripts/test-deb.sh` asserts the state after each step of the life cycle (in
    CI, on tags).
- [x] **II. Rust Safety & Quality**:
  - no `unsafe`; the Windows check uses only `std::io::IsTerminal` (R11);
  - write failures are a new `Error::ConfigWrite { path, source }` variant;
  - no `unwrap`/`expect` in runtime paths;
  - fmt and clippy remain gates.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new direct crate, only two clap features; their five small transitive crates are listed
    below;
  - still one self-contained binary; packaging tools run only in CI;
  - no new configuration setting; the two new options are actions.
- [x] **Domain rules**:
  - HR-2: the exported band list is in frequency order;
  - HR-1: the export header date is UTC.
- [x] **IV. Persistent Viewer Settings**: not affected; no page change.
- [x] **V. Portable Development, Pipeline-Built Releases**:
  - all new program behaviour builds and is tested on Linux, macOS and Windows hosts;
  - platform-specific code (`cfg(windows)` pause, `cfg(any(windows, target_os = "macos"))` path
    base, Unix file mode) has a path for every target;
  - packages, images and archives are built only by the pipeline, on tags.
- [x] **Interface rules**: not affected; no HTTP or WebSocket change.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none new (direct) | clap features `wrap_help`, `unicode` (R8) on the existing dependency | `std` for terminal width (needs platform calls); clap's own feature is the supported path |
| `terminal_size` 0.4 (+ `rustix` 1.1, `linux-raw-sys` 0.12 on Linux; `windows-sys` already present) | transitive, from `wrap_help`: reads the terminal width to wrap help (FR-027) | a fixed width breaks narrow and wide terminals (SC-007) |
| `unicode-width` 0.2, `unicase` 2.10 | transitive, from `unicode`: width of non-ASCII help text, case-insensitive suggestions | ASCII-only width; acceptable, but the operator asked for every useful help feature |

## Project Structure

### Documentation (this feature)

```text
specs/008-platform-packaging/
├── plan.md
├── research.md          # R1–R13
├── data-model.md        # exported configuration, release artifacts, service account, package life cycle
├── quickstart.md
├── contracts/
│   ├── command-line.md        # new options, help behaviour, exit codes, Windows pause
│   ├── configuration.md       # export format, file lookup order, path resolution, permissions
│   ├── debian-package.md      # files, owners, modes, maintainer-script behaviour per action
│   └── release-artifacts.md   # names, contents, image tags, checksums, CI variables
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
Cargo.toml               # clap features; [package.metadata.deb] (assets, conffile, scripts, units)
src/
├── config.rs            # Cli: help text split, styles, --save-config/--dump-config;
│                        # File* structs Serialize + From<&Config>; config_path(.., exe_dir);
│                        # relative path base on Windows/macOS                       [tests]
├── config_export.rs     # NEW: render(&Config) -> String; write(path, text) atomic, 0600 [tests]
├── errors.rs            # Error::ConfigWrite                                          [tests]
├── console.rs           # NEW: should_wait(failed, interactive, windows) decision      [tests]
├── lib.rs               # pub mod config_export, console
└── main.rs              # save/dump flow; data-path resolution; Windows pause call site
deploy/
├── config.example.toml  # conffile source; data path /var/lib/live-qso-map/cty.dat
├── debian/
│   ├── live-qso-map.service   # moved from deploy/: User=live-qso-map, /usr/bin, --config
│   ├── postinst               # account, conffile owner/mode, try-restart, first-install hint
│   └── postrm                 # purge: dirs, account (GECOS marker), group
├── docker/Dockerfile.release  # trixie-slim + pipeline binaries per TARGETARCH, user 10001
├── windows/README.txt
└── macos/README.txt
Dockerfile               # local build from source: rust:<pinned>-trixie → trixie-slim
scripts/test-deb.sh      # CI-only package life-cycle test
scripts/check-version.sh # tag vs Cargo.toml version gate (POSIX sh), used by CI
scripts/test-check-version.sh  # its test
scripts/test-image.sh    # CI-only container smoke test
.circleci/config.yml     # check-version, package-deb, test-deb, package-archives, docker-image,
                         # SHA256SUMS in publish
README.md                # installation per platform; save/dump; Docker Hub image
tests/
├── cli_help.rs          # width, short/long help, suggestion
└── config_export.rs     # binary-level save/dump behaviour (exit codes, files, messages)
```

**Structure Decision**: single crate, as today. Packaging inputs live under `deploy/` per platform.
Their wiring lives in `Cargo.toml` (`[package.metadata.deb]`) and `.circleci/config.yml`.

## Complexity Tracking

No violations. The R11 decision removed the only candidate, an `unsafe` Windows API call.
