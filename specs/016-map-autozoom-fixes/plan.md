# Implementation Plan: Map Auto-Zoom Fixes

**Branch**: `feature/016-map-autozoom-fixes` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/016-map-autozoom-fixes/spec.md`

## Summary

Auto-zoom stops shrinking the globe to a dot and stops mis-framing the flat map. On the globe,
the zoom is the free-area fit only. The globe is kept only when every framed marker is at least
`map.globe_border_deg` degrees of arc (default 10) inside the visible horizon at that zoom, or
when the horizon is out of view; otherwise the flat view is shown. On the flat view with a known
QTH, auto-zoom centres on the house and computes the highest zoom that holds every marker in the
free area. The operator setting reaches the page as `hello.globe_border_deg`. Research R1–R7
holds the numbers measured on the operator's snapshot.

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2) + browser ES modules
**Primary Dependencies**: none new (existing `clap`, `serde`, `toml`; MapLibre GL JS 5.24.0 on the page)
**Storage**: N/A (one new operator setting in the config file)
**Testing**: `cargo test`, `node --test web/`, `node tests/browser/run.mjs`
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary serving the map page
**Performance Goals**: one refit at most per second (feature 004 FR-014); the new frame maths is O(markers), like `boundingCap`
**Constraints**: no change to the protocol version; no viewer storage; feature 004 behaviour kept apart from FR-001–FR-011
**Scale/Scope**: up to 1,000 displayed QSOs per page (feature 014 cap)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: unit tests for the border check and the centred flat frame (snapshot,
      antimeridian, pole, all-at-QTH), Rust tests for the new setting's parsing, range and
      malformed values, and a browser regression step for the snapshot are planned before code.
- [x] **II. Rust Safety & Quality**: no `unsafe`; the value is validated with a `config_error`
      (crate `Error`), with no `unwrap` in runtime paths; fmt and clippy gates unchanged.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate. One setting, which the operator
      asked for. Two rules (the limb share and the 5° margin) are deleted.
- [x] **Domain rules** (`docs/ham-radio.md` §1): no change to times, bands, entities or position
      precision and source. Positions are only read for framing.
- [x] **IV. Persistent Viewer Settings**: the border is an operator setting (Clarifications), not
      stored in the browser. Existing viewer settings are untouched.
- [x] **V. Portable Development, Pipeline-Built Releases**: plain Rust and JS; no
      platform-specific code; the browser step is rehearsed in the pipeline (LL-007).
- [x] **Interface rules** (`docs/http-api.md`): one added snake_case field in `hello` (JSON
      number); protocol stays 3 (API-4); contract tests on `hello` serialisation planned.

Post-design re-check (after Phase 1): still all pass. The design adds one field and one setting,
and deletes code.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | — |

## Project Structure

### Documentation (this feature)

```text
specs/016-map-autozoom-fixes/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── configuration.md
│   └── websocket-protocol.md
└── tasks.md            (next: /speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── config.rs                    # CLI arg, env, file field, MapConfig.globe_border_deg, range check, export
├── config_export.rs             # save-config test lists the new key
└── web/
    ├── protocol.rs              # hello.globe_border_deg
    └── assets/
        ├── autozoom-geo.js      # border check replaces limb rule + fitsGlobe; new centred flat frame
        └── app.js               # read hello value; fitNow uses the new globe zoom and flat frame
deploy/config.example.toml       # commented-out globe_border_deg
web/autozoom-geo.test.js         # rewritten globe tests, new flat-frame tests, snapshot fixture
tests/browser/run.mjs            # snapshot step at 1024×768 and 1920×1080
tests/ws_protocol.rs             # hello carries globe_border_deg (if hello is asserted there)
```

**Structure Decision**: no new modules. Pure maths stays in `autozoom-geo.js` (tested with
`node --test`), DOM and MapLibre calls stay in `app.js`, and the setting follows the
`recent_header` path through `config.rs` and `protocol.rs`.

## Complexity Tracking

No violations.
