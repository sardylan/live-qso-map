# Data Model: Working, Safe and Proven CI Pipeline

This feature has no program data. Its "entities" are pipeline concepts and the few files that
drive releases.

## Run kind

What started a pipeline run decides its workflow, its jobs and its secrets.

| Kind | Started by | Pipeline values | Workflow | Jobs | Secrets |
|------|------------|-----------------|----------|------|---------|
| gate (`main`) | push to `main` (trigger `only-build-prs`) | `pipeline.git.branch = main` | `test_and_build` | gates | none |
| gate (pull request) | pull request opened, re-opened or pushed to (same trigger) | `pipeline.git.branch = <pr branch>` | `test_and_build` | gates | none |
| release | push of tag `vX.Y.Z` (same trigger) | `pipeline.git.tag = vX.Y.Z` | `test_and_build` | gates + release chain | context `live-qso-map-release`, in four jobs only |
| rehearsal | `circleci pipeline run --param rehearsal=true` on a branch | `rehearsal = true` | `rehearsal` | build, package, archive, image test | none |
| scheduled | weekly schedule trigger | `newest-toolchain = true` | `newest_toolchain` | `lint-stable`, `test-rust-stable` | none |

**Rules:**
- **Exactly one workflow per run:** the `when` conditions are mutually exclusive.
  `test_and_build` runs only when neither parameter is set.
- **Nothing else starts a run:** a branch with no pull request and a fork pull request start no
  run at all (FR-004, FR-010).
- **Secrets on tags only:** the context's expression restriction
  `pipeline.git.tag starts-with "v"` stops any other kind from receiving it, even through a
  wrong `context:` entry.

## Job graph

### Gates (every run of `test_and_build`)

`lint`, `test-rust`, `test-web`, `test-tools`, `audit`. These five names are the required checks
on `main` (R9).

### Release chain (tags `vX.Y.Z` only, in addition to the gates)

```
check-version (context; version + changelog section + secrets present)
  └─ requires: check-version + all five gates
       ├─ build-linux ×3 ─┬─ package-deb ×3 (requires cargo-deb) ─┬─ test-deb amd64 ×4
       │                  │                                       └─ test-deb arm64 ×4
       │                  └─ image-test amd64, image-test arm64 ─ image-push (context, -rc)
       ├─ build-macos ─┐
       └─ build-windows ┴─ package-archives
cargo-deb (cache) ────────────────────────────────────────────────┘
publish-github-release (context) ← every build, package, test, archive and image-push job
docker-promote (context) ← publish-github-release
```

### Rehearsal

The same graph without `check-version`, `image-push`, `publish-github-release` and
`docker-promote`. Its final job `rehearsal-summary` lists the artefacts kept in the run, and
states "publishing secrets: not checked (rehearsal)".

## Release candidate image

| State | Tags on Docker Hub | Entered by | Left by |
|-------|--------------------|------------|---------|
| absent | none | — | `image-push` |
| candidate | `X.Y.Z-rc` | `image-push` | `docker-promote` |
| published | `X.Y.Z`, `latest` (same digest) | `docker-promote` (re-tag) | — |

`docker-promote` deletes `X.Y.Z-rc` after re-tagging. If the release fails before promotion, the
candidate stays until a re-run (edge case "release re-run after a partial failure").

## CHANGELOG.md

```
# Changelog
<intro line>

## [Unreleased]

### Fixed
- …

## [X.Y.Z] - YYYY-MM-DD

### Added | Changed | Fixed | Removed | Security | Notes
- …
```

**Validation:**
- **"Unreleased" heading:** always present, exactly once.
- **Version sections:**
  - unique, newest first;
  - dates are UTC `YYYY-MM-DD`;
  - each has at least one `###` subsection with at least one `- ` entry.
- **Release:** the release needs a section for its version: `scripts/changelog-section.sh` in CI
  and `tests/changelog.rs` in `cargo test`.
- **Prepare step:** it refuses an empty "Unreleased" (`exactly = 1` replacement in
  `release.toml`).

## Release version

| Source | Role |
|--------|------|
| `Cargo.toml` `[package] version` | single source (FR-030) |
| `Cargo.lock` root package version | follows it in the release commit |
| tag `vX.Y.Z` | derived by `cargo release tag` (`tag-name = "v{{version}}"`) |
| `.deb` version `X.Y.Z-1`, archives, image tags | derived in CI from the tag, checked equal to `Cargo.toml` by `check-version` |

## Release flow (state of one version)

```
unreleased entries on main
  └─ prepare (cargo release version/replace/commit on release/next) ─▶ release PR open
       └─ gates green + merge ─▶ version on main, untagged
            └─ tag (cargo release tag/push) ─▶ tag vX.Y.Z pushed
                 └─ release chain green ─▶ GitHub release + image X.Y.Z/latest
```

## CI service objects (outside the repository)

| Object | Identity | Defined in |
|--------|----------|------------|
| GitHub App pipeline | `f5014549-bbc3-4712-901f-e2bd315c9e6e` | `contracts/ci-service-setup.md` |
| Event trigger | one, preset `only-build-prs` | same |
| Schedule trigger | weekly, `newest-toolchain = true` | same |
| OAuth pipeline | `baf57313-…`, no triggers (it can't be deleted) | same |
| Context | `live-qso-map-release`, three variables, project + expression restrictions | same |
| Ruleset on `main` | `deploy/github/main-ruleset.json` | `contracts/ci-service-setup.md` |
