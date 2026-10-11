# Verification: Platform Packages, Friendlier Command Line and Configuration Export

**Feature**: `008-platform-packaging` | **Date**: 2026-10-11
**Host**: macOS (Apple Silicon), Rust 1.99.0, Node 26.11.1

## Automated gates (T047–T051)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass; no new inline lint suppression in the feature's diff |
| `cargo test` | pass: 328 tests, 0 failed (1 ignored, as before) |
| `node --test web/` | pass: 131 tests, 0 failed |
| `sh scripts/test-check-version.sh` | pass |
| `sh -n` on `scripts/test-deb.sh`, `scripts/test-image.sh`, `deploy/debian/postinst`, `deploy/debian/postrm`, `deploy/docker/entrypoint.sh` | pass |
| CircleCI `buildConfig` validation of `.circleci/config.yml` | valid |
| `cfg` review (T050) | `cfg!(windows)` pause, `cfg!(any(windows, target_os = "macos"))` path base, `cfg(unix)` file modes; each has a path on every target |
| Dependencies (T051) | no new direct crate; five transitive crates from the clap features, listed in plan.md |

## Quickstart (T053)

| Section | Result |
|---------|--------|
| §1 Help | pass: `-h` has no "Environment:" lines, `--help` has 33; at 80 columns no line is longer; `--udp-prot 1` exits 2 with `tip: a similar argument exists: '--udp-port'`; `NO_COLOR=1` piped help has no escape codes |
| §2 Configuration export | pass: `--save-config` writes `-rw-------`; loading it back and saving to `-` prints the same settings; invalid band and missing folder exit 1 with nothing written (also covered by `tests/config_export.rs`) |
| §3 File next to the program | pass on macOS: `live-qso-map.toml` in `target/debug/` is used when started from `/tmp`; `cty.dat` is looked up as `target/debug/./cty.dat` |
| §4 Debian package | pending: needs the `.deb` from a release tag (T026, T052) |
| §5 Container image | pending: needs the image from a release tag (T052) |
| §6 Windows | pending: needs the zip from a release tag (T052) |
| §7 macOS archive | pending: needs the zip from a release tag (T052) |
| §8 Release integrity | pending: needs a release tag on a fork (T052) |

## Deferred to the first release tag

The feature was closed on 2026-10-11 without a fork run, because the repository has no fork.
T026 (generated maintainer scripts), T052 (tag workflow, artifacts, `SHA256SUMS`, mismatching tag)
and quickstart §4–§8 are checked on the first release tag, `v2.0.0` on `main`. If that tag's build
fails, nothing is published; the fix lands on `main` and a new tag is cut (constitution,
Development Workflow).
