# Changelog

All notable changes to live-qso-map are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format, with dates in UTC. Every pull
request adds its entries under "Unreleased"; a release turns them into the version's section,
which becomes the GitHub release notes (`docs/releasing.md`).

## [Unreleased]

## [2.0.1] - 2026-10-13

### Fixed
- CI checks out the code over HTTPS through the CircleCI GitHub App; every earlier run had failed
  at checkout over SSH.
- The page tests run on Node 24 in CI (`node --test 'web/*.test.js'`); `node --test web/` loaded
  `web` as a module there and ran no test.
- Re-running the publishing of a release keeps its notes and only replaces files.
- Every tool the pipeline downloads is pinned and checked against its SHA-256.

### Changed
- CI runs the quality gates on pushes to `main`, on pull requests and on release tags, once per
  event; branches without a pull request no longer start a run.
- `main` is protected: changes arrive through pull requests with green gates.
- Publishing secrets live in a CircleCI context that only release tags can use.
- New `audit` gate: vulnerable dependencies, licences and documentation warnings.
- The ARM64 Debian package and container image are tested on ARM64 before a release.
- Releases are cut with cargo-release in two steps (`docs/releasing.md`), and their notes come
  from this file.

### Notes
- 2.0.0 was tagged but never published, because every CI run failed at checkout. 2.0.1 is the
  2.0.0 program plus these pipeline fixes.

## [2.0.0] - 2026-10-11

### Notes
- Tagged, never released: the CI pipeline failed at checkout, so no package, archive or image was
  built. Its changes ship in 2.0.1.

Versions up to 1.0.0 are listed in the
[GitHub history](https://github.com/sardylan/live-qso-map/releases).
