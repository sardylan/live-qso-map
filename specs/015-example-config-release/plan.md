# Implementation Plan: Example Configuration in Every Release

**Branch**: `feature/015-example-config-release` | **Date**: 2026-10-15 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/015-example-config-release/spec.md`

## Summary

Complete the existing `deploy/config.example.toml`:
- comment out every line, including the Debian-only `country_data.path`;
- show defaults;
- give every setting without a default a `"<…>"` placeholder, provider usernames and passwords
  included;
- say how to use the file and where secrets can come from instead.

Ship it unchanged as `config.example.toml` next to the documentation of every deliverable:
- the Linux archives, the Windows and macOS zips;
- `/usr/share/doc/live-qso-map/` in the `.deb` and in the container image;
- a standalone download on the GitHub release.

The Debian package gets its own short live `config.toml` (country data path only).

`cargo test` enforces the content. A new pre-publish script (`scripts/check-release-files.sh`) and
the existing package and image checks enforce the presence. No program behaviour changes.

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2). Tests only; no runtime code changes.
**Primary Dependencies**: none new (tests use the existing `toml` and `clap` dependencies)
**Storage**: N/A (files shipped in releases)
**Testing**: `cargo test` (`src/config_export.rs` unit tests); CI-only POSIX `sh` checks (`scripts/check-release-files.sh`, `scripts/test-image.sh`, the `package-deb` contents check); a release rehearsal
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary
**Performance Goals**: N/A
**Constraints**: the example must be byte-identical in every deliverable; Debian upgrades keep an edited live configuration
**Scale/Scope**: 1 example file (~110 lines), 1 new Debian file, 9 release files plus the image

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: the new `config_export` tests are written first and seen failing
      against today's example: all-commented, placeholder keys, credentials required, Debian live
      file. `check-release-files.sh` is first run in a rehearsal before the packaging jobs copy
      the file, and must fail there.
- [x] **II. Rust Safety & Quality**: no runtime code; test code only. fmt and clippy gates
      unchanged.
- [x] **III. Simplicity & Minimal Dependencies**: no crates; one source file copied unchanged; no
      generation step; one small script.
- [x] **Domain rules** (`docs/ham-radio.md` §1): examples keep ADIF band names. `bands` lists its
      groups in frequency order. Band-colour examples use real ADIF bands.
- [x] **IV. Persistent Viewer Settings**: not affected (operator configuration, not viewer
      settings).
- [x] **V. Portable Development, Pipeline-Built Releases**: content checks are `cargo test` on
      every host. Packaging checks run in CI only, as optional helpers like the existing
      `scripts/test-*.sh`. Nothing is cross-compiled locally.
- [x] **Interface rules** (`docs/http-api.md`): not affected (no HTTP change).

Post-design re-check (after research.md, data-model.md and contracts/): all gates still pass. The
design adds one CI-only shell script and one Debian data file, and no exceptions.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | — |

## Project Structure

### Documentation (this feature)

```text
specs/015-example-config-release/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── example-config.md
│   └── release-deliverables.md
└── tasks.md            (from /speckit-tasks)
```

### Source Code (repository root)

```text
deploy/
├── config.example.toml        rewritten: all commented, placeholders, usage header
├── debian/config.toml         NEW: short Debian live configuration (country data path)
├── docker/Dockerfile.release  + COPY example to /usr/share/doc/live-qso-map/
├── windows/README.txt         + "start from config.example.toml"
└── macos/README.txt           + same
Dockerfile                     + COPY example to /usr/share/doc/live-qso-map/ (local build parity)
Cargo.toml                     deb assets: live file from deploy/debian/config.toml (the only conffile);
                               example to usr/share/doc (plain file, always overwritten)
src/config_export.rs           tests: credentials required, all commented, placeholders, Debian file
scripts/
├── check-release-files.sh     NEW: example present and identical in every release file
└── test-image.sh              + step "example"
.circleci/config.yml           build-linux tar, package-archives zips + standalone copy,
                               package-deb contents, publish-github-release and
                               rehearsal-summary run check-release-files.sh
deploy/debian/postinst         first-install message names the example
README.md, docs/releasing.md   mention the example in each deliverable and the release download
```

**Structure Decision**: Single project. The feature touches packaging (`deploy/`, `Cargo.toml`
deb metadata, `.circleci/config.yml`, `scripts/`), the tests of the existing example in
`src/config_export.rs`, and the documentation. No module is added.

## Complexity Tracking

No constitution violations.

## Risks

- **Pipeline-only paths** (LL-007): the archive, zip, deb and image changes run only on tags and
  rehearsals. Mitigation: a rehearsal must pass before merge (research R7).
- **Debian conffile change**: the live file is a standard conffile (FR-010). Hosts with an
  unedited old live file get the new short file with no question. Hosts with an edited one get
  dpkg's standard question on the upgrade to this version, because the packaged file changed.
  Unattended upgrades answer it with their `--force-conf*` policy. The example in
  `/usr/share/doc` is never a conffile, so it is always overwritten (FR-010b).
- **Placeholder strings on numeric settings**: uncommenting `latitude` without editing fails at
  load with a TOML type error. This is intended (research R2); the comment beside it says to set
  decimal degrees.
