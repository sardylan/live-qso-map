# Implementation Plan: Station Status Box

**Branch**: `feature/017-station-status-box` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/017-station-status-box/spec.md`

## Summary

Show the station's current band and operator on every page, taken from the most recent QSO by QSO
time. QARTest sends no operator-change message and its score message has no operator, so the source
is the `OPERATOR` already stored on each QSO, falling back to the station callsign (research R1,
R4). The service derives the status from the history and sends a new `station_status` WebSocket
message on connect, on lag and after every `upsert` or `remove`. The message carries an age, so the
page greys the box after 15 minutes without using its own clock (R2, R3, R5). The page shows
`40m · IK0ABC` in the bottom-left corner, above the score line (R6).

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2)
and browser ES modules
**Primary Dependencies**: none new (serde, tokio as today)
**Storage**: none; derived from the in-memory history (which feature 014 already persists)
**Testing**: `cargo test`, `node --test web/`, release browser steps (`tests/browser/run.mjs`)
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary
**Performance Goals**: status on every page within 2 s of a QSO (FR-003); one extra small message
per QSO per page
**Constraints**: no browser clock for staleness; protocol version unchanged (R7)
**Scale/Scope**: contest rates (a few QSOs a minute), tens of pages

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: unit tests first for the status builder (latest by QSO time, operator
      fallback chain, empty history, future QSO time gives age 0, missing band), for `Session`
      (message order on connect, after upsert and remove, on lag), a `tests/ws_protocol.rs`
      contract test, `web/station.test.js` for the text and tooltip helpers, and the browser step.
- [x] **II. Rust Safety & Quality**: no `unsafe`; the age uses the existing `now_unix` pattern with
      saturating arithmetic, no `unwrap` in runtime paths; fmt and clippy gates unchanged.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate; no stored copy of the status, it is
      recomputed from the history (R2); no setting to move or hide the box (spec, out of scope).
- [x] **Domain rules** (`docs/ham-radio.md` §1): UTC `qso_at` and `HH:MMZ` tooltip; ADIF band name
      as stored; `OPERATOR` and `STATION_CALLSIGN` with their glossary meaning (§2.1).
- [x] **IV. Persistent Viewer Settings**: the feature adds no viewer setting.
- [x] **V. Portable Development, Pipeline-Built Releases**: no platform-specific code; the simulator
      option is plain Python standard library.
- [x] **Interface rules** (`docs/http-api.md`): JSON text frame with `type`, snake_case fields, UTC
      times, nulls for missing values; additive, so protocol stays 4 (R7); contract test planned.

Post-design re-check: all still pass; no Complexity Tracking entry.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | — |

## Project Structure

### Documentation (this feature)

```text
specs/017-station-status-box/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/websocket-protocol.md
└── tasks.md            (next: /speckit-tasks)
```

### Source Code (repository root)

```text
src/web/protocol.rs        ServerMessage::StationStatus and its builder from History + fallback callsign + now
src/web/session.rs         send it on connect, after upsert/remove, on lag; drive locks history on changes
src/web/assets/station.js  new: pure text and tooltip helpers, STATION_STALE_MS = 900_000
src/web/assets/app.js      onStationStatus, reset on hello, render on the 5 s tick
src/web/assets/index.html  <div id="station"> above the score line
src/web/assets/app.css     .station tweaks, if the shared .score style is not enough
web/station.test.js        new: node tests for station.js
tests/ws_protocol.rs       contract test for station_status
tests/browser/run.mjs      new `station` step (LL-007: rehearse before merge)
scripts/simulate_qartest_qso.py  --operator option for the quickstart
docs/ham-radio.md          note under §2.1: the current operator is that of the latest QSO
```

**Structure Decision**: existing single-binary layout; one new browser module, no new Rust module.
The status builder lives next to `QsoView` in `protocol.rs`, as `ServerMessage::score` does.

## Complexity Tracking

None.
