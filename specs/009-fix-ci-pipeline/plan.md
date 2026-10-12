# Implementation Plan: Working, Safe and Proven CI Pipeline

**Branch**: `feature/009-fix-ci-pipeline` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-fix-ci-pipeline/spec.md`

## Summary

**Already done outside the repository (2026-10-12):**
- **R0:** the project's CircleCI pipeline now uses the GitHub App, with HTTPS checkout. The
  OAuth trigger is deleted.
- **First run:** the first run through the App (on `main`) passed three gates and exposed R16.

This plan covers the rest. It changes the repository (`.circleci/config.yml`, scripts, release
files, documentation) and the CI service and code host settings recorded in
[`contracts/ci-service-setup.md`](contracts/ci-service-setup.md).

**When runs start (R1, R2, R10):**
- **Triggers:** one GitHub App trigger with preset `only-build-prs` covers pushes to `main`, pull
  requests and tags, with exactly one run each. Branches without a pull request and fork pull
  requests start nothing.
- **Workflows:** pipeline parameters choose between `test_and_build`, a manual `rehearsal` and a
  weekly `newest_toolchain`.

**Gates (R4, R14):**
- **Page tests:** they use `node --test 'web/*.test.js'`, which works on Node 24 and 26.
- **New `audit` gate:** cargo-deny (advisories and licences), plus `cargo doc` with warnings
  denied.
- **Protection:** `main` is protected by a versioned GitHub ruleset. It requires the five gates
  and pull requests, with an admin bypass (R9).

**Safe releases (R3, R5, R8, R11):**
- **Secrets:** they move to a context restricted to this project and to `v*` tags, and only four
  jobs use it. `check-version` also checks that they are present before any build.
- **GitHub release:** publishing uses the GitHub CLI, creating the release with the changelog
  notes, or uploading `--clobber` without touching the notes.
- **Verified downloads:** every downloaded tool is pinned and SHA-256-verified, and `Cross.toml`
  pins the cross images by digest.
- **Image:** it is smoke-tested natively on amd64 and arm64. crane promotes it without a VM and
  then deletes the `-rc` tag.

**Rehearsal (R2):**
- the full build, package, archive and image-test chain, started by hand with
  `--param rehearsal=true`;
- no context, nothing published.

**Speed (R13):**
- caches keyed by job, toolchain, architecture and lock file, refreshed every run (`{{ epoch }}`);
- macOS and Windows cached too;
- cargo-deb installed once per release.

**Wider checks (R8, R12):**
- ARM64 package life-cycle tests on Arm Docker;
- the arm64 image smoke test on an Arm machine;
- a weekly schedule on Rust stable, with job names distinct from the gates.

**Releases (R6, R7, R16):**
- **Version and tag:** `Cargo.toml` is the version's single source. cargo-release 1.1.6 cuts a
  release in two steps: prepare a `release/next` pull request (version, `Cargo.lock`,
  changelog), then tag the merged commit and push only the tag.
- **Notes:** `CHANGELOG.md` (Keep a Changelog, UTC dates) provides the release notes.
- **Changelog checks:** `tests/changelog.rs` and `scripts/changelog-section.sh` make sure every
  release has a section.
- **First release:** 2.0.1. `v2.0.0` is left untouched.

**Fix (R17):** the image licence label becomes `GPL-3.0-or-later`, matching `Cargo.toml`.

## Technical Context

**Language/Version**:
- CircleCI config 2.1, POSIX `sh` scripts (CI only);
- one Rust integration test (edition 2024; pinned 1.99 in CI, latest stable for development,
  `docs/rust-guidelines.md` §2);
- Node 24 in CI for the existing page tests.

**Primary Dependencies**:
- **Crates:** none new.
- **CI-only tools, pinned and verified:** cross 0.2.5, cargo-deb 3.8.0, cargo-deny 0.20.2,
  gh 2.102.0, crane 0.22.1, rustup-init (pinned).
- **Maintainer-only tool:** cargo-release 1.1.6.
- **CircleCI orb:** `circleci/windows@5.1.1`, unchanged.

**Storage**:
- `CHANGELOG.md`, `release.toml`, `deny.toml` and `deploy/github/main-ruleset.json` in the
  repository;
- CircleCI context `live-qso-map-release`;
- CircleCI caches.

**Testing**:
- **Scripts:** `sh scripts/test-*.sh` for each CI script (`check-version`, `changelog-section`,
  `check-secrets`).
- **Changelog:** `cargo test` (`tests/changelog.rs`).
- **Config regressions:** `tools/test_ci_config.py` (Python standard library, any host) guards
  the R16 and R2 fixes in `.circleci/config.yml`.
- **Validation:** `circleci config validate` / `process`.
- **On CircleCI:** the rehearsal, plus the event checks in `quickstart.md`.

**Target Platform**:
- **CI:** CircleCI cloud executors:
  - Docker x86 and `arm.medium`;
  - machine `ubuntu-2604:current` (x86) and `ubuntu-2204:current` (`arm.medium`);
  - macOS `xcode: "27.0"` with `m4pro.medium`;
  - Windows Server 2025.
- **Releases:** the targets from Principle V are unchanged.

**Project Type**: single binary; this feature changes CI, release tooling and documentation only.

**Performance Goals**:
- gate runs with no dependency change at least 30% faster than cold-cache runs (SC-009);
- gate feedback within minutes.

**Constraints**:
- **Secrets:** only for `v*` tag jobs that publish (SC-007).
- **One run per event** (FR-006).
- **No direct pushes to `main`** (FR-009).
- **Developer tooling unchanged:** build, test and lint still need only `cargo`, `node` and
  `python3` (Principle V).
- **Free plan:** macOS concurrency is 1.

**Scale/Scope**:
- **Gates:** 5 jobs per gate run.
- **Release run:** about 30 jobs: 5 gates, `check-version`, `cargo-deb`, 5 builds, 3 packages,
  8 package tests, archives, 2 image tests, push, publish, promote.
- **Rehearsal run:** about 23 jobs.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - every new CI script gets a `test-*.sh` written first and seen failing, like
    `test-check-version.sh` in 008:
    - `changelog-section.sh`: missing section, empty section, last section in the file, versions
      that share a prefix (`2.0.1` vs `2.0.10`), CRLF input;
    - `check-secrets.sh`: missing, empty and set variables; values never printed;
  - `tests/changelog.rs` is written before `CHANGELOG.md` and fails until the file exists;
  - pipeline behaviour is checked by the rehearsal and the quickstart event table. A CI config
    can't be unit-tested.
- [x] **II. Rust Safety & Quality**:
  - the only Rust code is an integration test (where `unwrap`/`expect` are allowed);
  - no `unsafe`;
  - fmt and clippy stay gates;
  - `cargo doc` with warnings denied is added (`audit`).
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new crate;
  - the new tools are CI-only or maintainer-only, each pinned with a stated reason (research);
  - one trigger instead of three;
  - one config file;
  - no wrapper scripts for releases.
- [x] **Domain rules**: HR-1: changelog dates and the schedule are UTC. Nothing else is touched.
- [x] **IV. Persistent Viewer Settings**: not affected.
- [x] **V. Portable Development, Pipeline-Built Releases** (constitution 1.7.0):
  - gates run on every commit to `main` (and on pull requests);
  - cross-compilation stays in the pipeline;
  - release steps are plain `git` / `cargo release` / `gh` commands, the same on Linux, macOS
    and Windows;
  - CI scripts are POSIX `sh`, used only by CI, as in 008;
  - the `rehearsal` builds release binaries on a branch, without publishing them and without
    secrets. Constitution 1.7.0 (amended by T003 for this feature) allows exactly this.
- [x] **Interface rules**: not affected.

**Post-design re-check (after Phase 1)**: unchanged. The contracts add no crate, keep the
release artefacts of 008 (FR-027), and keep cross-compiling and publishing in the pipeline.
`/speckit-analyze` found the rehearsal conflicting with Principle V 1.6.0. It was resolved by
amending the constitution to 1.7.0 (T003), not by a waiver.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | `tests/changelog.rs` uses `std::fs` and string matching only | a Markdown parser crate: not needed for a line-oriented check |

## Project Structure

### Documentation (this feature)

```text
specs/009-fix-ci-pipeline/
├── spec.md
├── plan.md
├── research.md          # R1–R17
├── data-model.md        # run kinds, job graph, release candidate, changelog, release flow
├── quickstart.md        # local checks, event table, ruleset, rehearsal, dry run, first release
├── contracts/
│   ├── pipeline.md            # parameters, workflows, jobs, contexts, commands, scripts
│   ├── release-process.md     # release.toml, prepare and tag steps, refusals
│   └── ci-service-setup.md    # triggers, schedule, context, ruleset, fork procedure
├── reference/circleci-review.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
.circleci/config.yml          # rewritten per contracts/pipeline.md
Cross.toml                    # image digests per target
release.toml                  # new: cargo-release configuration
deny.toml                     # new: advisories + licence allow list
CHANGELOG.md                  # new: Keep a Changelog, [Unreleased] + 2.0.1 entries
deploy/github/main-ruleset.json   # new: ruleset for main
deploy/docker/Dockerfile.release  # licence label GPL-3.0-or-later (R17)
scripts/
├── check-version.sh              # unchanged
├── test-check-version.sh         # unchanged
├── changelog-section.sh          # new
├── test-changelog-section.sh     # new
├── check-secrets.sh              # new
└── test-check-secrets.sh         # new
tests/changelog.rs            # new: CHANGELOG has [Unreleased] and a section for the crate version
docs/ci.md                    # new: CI service setup, triggers, secrets, ruleset, forks, diagnosis
docs/releasing.md             # new: the two release steps
README.md                     # page-test command; Releases section points to docs/releasing.md and docs/ci.md
specs/008-platform-packaging/contracts/release-artifacts.md   # licence label
```

**Structure Decision**:
- The existing layout is kept. The new files sit where the project already keeps their kind:
  - `scripts/` for CI shell scripts with their tests;
  - `deploy/` for platform configuration;
  - `docs/` for maintainer documentation;
  - the repository root for tool configuration.
- `src/` is not touched.

## Complexity Tracking

No violations. The rehearsal's former deviation from Principle V was resolved by the
constitution amendment to 1.7.0 (T003, TR-014).
