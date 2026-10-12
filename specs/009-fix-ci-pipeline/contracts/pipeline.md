# Contract: CI Pipeline (feature 009)

`.circleci/config.yml` after this feature. It refines feature 008's
`contracts/release-artifacts.md`, whose artefacts are unchanged (FR-027), except for the image
licence label (research R17).

## Pipeline parameters

| Name | Type | Default |
|------|------|---------|
| `rust-version` | string | `"1.99"` (pinned; raise together with the `Dockerfile`) |
| `rehearsal` | boolean | `false` |
| `newest-toolchain` | boolean | `false` |

## Workflows

| Workflow | `when` |
|----------|--------|
| `test_and_build` | `not rehearsal` and `not newest-toolchain` |
| `rehearsal` | `rehearsal` |
| `newest_toolchain` | `newest-toolchain` |

## Gate jobs (`test_and_build`, every run; required checks on `main`)

| Job | Executor | Commands (all MUST pass) |
|-----|----------|--------------------------|
| `lint` | `rust:<rust-version>` | `cargo fmt --check`; `cargo clippy --locked --all-targets -- -D warnings` |
| `test-rust` | `rust:<rust-version>` | `cargo test --locked` (includes `tests/changelog.rs`) |
| `test-web` | `node:24` | `node --test 'web/*.test.js'` |
| `test-tools` | `python:3.14` | `python3 -m unittest discover -s tools -p 'test_*.py'` (includes `tools/test_ci_config.py`, the regression tests for R16 and R2); `sh scripts/test-check-version.sh`; `sh scripts/test-changelog-section.sh`; `sh scripts/test-check-secrets.sh` |
| `audit` | `rust:<rust-version>` | `cargo deny check advisories licenses`; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked` |

The gates carry tag filters (`/^v\d+\.\d+\.\d+$/`), so they also run on release tags.

## Release jobs (`test_and_build`, tags `/^v\d+\.\d+\.\d+$/` only, branches ignored)

| Job | Context | Requires | Notes |
|-----|---------|----------|-------|
| `check-version` | `live-qso-map-release` | — | `check-version.sh "$CIRCLE_TAG"`; `changelog-section.sh X.Y.Z >/dev/null`; `check-secrets.sh GITHUB_TOKEN DOCKERHUB_USERNAME DOCKERHUB_TOKEN` |
| `cargo-deb` | — | `check-version` | installs cargo-deb 3.8.0 once (cached), persists `tools/cargo-deb` |
| `build-linux-<target>` ×3 | — | `check-version`, five gates | cross 0.2.5 (verified), images pinned by digest in `Cross.toml` |
| `build-macos`, `build-windows` | — | same | cached registry and `target/` |
| `package-deb-<target>` ×3 | — | `build-linux-<target>`, `cargo-deb` | |
| `test-deb-<distro>-<arch>` ×8 | — | `package-deb-x86_64…` or `package-deb-aarch64…` | distros: debian 12, 13; ubuntu 22.04, 24.04; arch amd64 (Docker), arm64 (Docker `arm.medium`) |
| `package-archives` | — | `build-macos`, `build-windows` | |
| `image-test-<arch>` ×2 | — | `build-linux-x86_64…` and `build-linux-aarch64…` | native `--load` build + `scripts/test-image.sh` |
| `image-push` | `live-qso-map-release` | both `image-test` | multi-platform push `X.Y.Z-rc` |
| `publish-github-release` | `live-qso-map-release` | every job above | gh 2.102.0: create with changelog notes, or upload `--clobber` (keeps notes) |
| `docker-promote` | `live-qso-map-release` | `publish-github-release` | crane: tag `X.Y.Z`, `latest`; delete `X.Y.Z-rc` |

**No other job declares a context.** No job uses `ghr`, `-delete`, or project-level secrets.

## Rehearsal workflow (`rehearsal = true`)

- **Jobs:** `cargo-deb`, `build-linux` ×3, `build-macos`, `build-windows`, `package-deb` ×3,
  `test-deb` ×8, `package-archives`, `image-test` ×2, and `rehearsal-summary` (requires all of
  them).
- **Not included:** no `check-version`, no context, no push, no publish, no promote.
- **Version in names:** the reusable command `set-release-version` exports `RELEASE_VERSION` and
  `DEB_VERSION`. On a tag they come from the tag. Without one (a rehearsal), artefact names use
  `Cargo.toml`'s version with the suffix
  `-rehearsal.<short-sha>`. Example: `live-qso-map-2.0.1-rehearsal.ab12cd3-x86_64-unknown-linux-musl.tar.gz`.
- **Packages:** for the `.deb` files, the version gate's `X.Y.Z-1` check is replaced by "equals
  `Cargo.toml` version".
- **Outputs:** `rehearsal-summary` stores `ws/release` as job artefacts, and prints
  `publishing secrets: not checked (rehearsal)`.

## Newest-toolchain workflow (`newest-toolchain = true`)

| Job | Commands |
|-----|----------|
| `lint-stable` | `rustup toolchain install stable --profile minimal -c rustfmt,clippy`; `RUSTUP_TOOLCHAIN=stable`; same commands as `lint` |
| `test-rust-stable` | same install; `cargo test --locked` |

The job names differ from the gates', so these jobs never set a required check's status.

## Commands (reusable)

| Command | Contract |
|---------|----------|
| `fetch-verified url sha256 dest` | downloads with `curl -fsSL`; fails unless `sha256sum -c` matches; never runs the file before the check |
| `install-rustup` | `rustup-init` for the host triple through `fetch-verified`, pinned rustup version |
| `with-cargo-cache key steps` | restores `cargo-v2-<key>-<rust-version>-{{ arch }}-{{ checksum "Cargo.lock" }}-` then `cargo-v2-<key>-<rust-version>-{{ arch }}-`; saves with `-{{ epoch }}`; prunes `target/*/incremental` |

## Scripts (POSIX `sh`, CI only, each with a `test-*.sh`)

| Script | Input | Exit 0 | Exit 1 |
|--------|-------|--------|--------|
| `scripts/check-version.sh` | tag, [Cargo.toml] | tag = `v` + version (unchanged from 008) | otherwise |
| `scripts/changelog-section.sh` | `X.Y.Z`, [CHANGELOG.md] | prints that section's body (without its heading) | section missing, or no `- ` entry in it |
| `scripts/check-secrets.sh` | variable names… | every variable set and non-empty | lists each missing name on stderr; never prints values |
