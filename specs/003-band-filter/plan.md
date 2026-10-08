# Implementation Plan: Band Filter

**Branch**: `003-band-filter` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-band-filter/spec.md`

## Summary

A `bands` setting selects the bands to consider. Entries are ADIF bands, named groups, or either
prefixed with `-` to exclude it. It resolves to a `BandSet` in frequency order.

- **Ingest**: QSOs on ignored or unknown bands are dropped right after normalisation, before the
  resend gate, lookups, the history and the pages.
- **Corrections onto an ignored band**: when the dropped QSO is a correction of a QSO the gate
  knows, ingest issues a revision-numbered removal. The history deletes the QSO and keeps a
  tombstone, so a stale in-flight revision cannot bring it back. Pages receive a new `remove`
  message.
- **Legend**: `hello` lists only considered bands, still in frequency order.

With no setting, behaviour is exactly as in feature 001.

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98)
**Primary Dependencies**: none new (existing tokio, axum, serde, clap, toml)
**Storage**: none new; tombstones are kept in memory next to the history
**Testing**: `cargo test` (unit and integration) and `node --test web/`
**Target Platform**: unchanged from feature 001
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**: a band check per QSO costs at most 33 comparisons; a removal reaches pages
within 2 s (SC-004)
**Constraints**: no behaviour change without the setting (FR-009); HR-2 frequency order everywhere
**Scale/Scope**: 8 groups and up to 33 bands

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - selection parsing and resolution get unit tests for every User Story 2 example, plus error,
    case and duplicate cases (SC-003);
  - a test checks that every group is valid and in frequency order (SC-005);
  - history tombstones, gate `contains` and the session `remove` message get unit tests;
  - `applyRemove` gets JS tests;
  - an integration test covers User Stories 1 and 3 (SC-001, SC-002, SC-004).
- [x] **II. Rust Safety & Quality**: no `unsafe`; errors are `Error::Config { setting: "bands", .. }`
  per `docs/rust-guidelines.md`; no panics on input; fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate; one new module; the filter is
  `Option<BandSet>`, so the feature-001 path is untouched when it is `None`.
- [x] **Domain rules** (`docs/ham-radio.md` §1):
  - HR-2: groups, the effective set, the startup log and the legend are all in frequency order, and
    a test enforces it;
  - HR-3: the band used is the normalised one, including the band derived from `FREQ`;
  - HR-9: filtering is by band only, so dupes are unaffected.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/003-band-filter/
├── plan.md
├── research.md          # R1–R7
├── data-model.md        # BandGroup, SelectionEntry, BandSet, History/gate/session changes
├── quickstart.md
├── contracts/
│   ├── configuration.md         # the `bands` setting (delta)
│   └── websocket-protocol.md    # hello.bands filtering, new `remove` message (delta)
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── band_filter.rs       # NEW: groups, SelectionEntry parsing, BandSet resolve/contains/Display   [tests]
├── lib.rs               # + pub mod band_filter
├── config.rs            # + `bands` setting (CLI --bands, LQM_BANDS, TOML) → Config.band_filter    [tests]
├── history.rs           # + remove(id, rev) with tombstones; upsert respects tombstones            [tests]
├── ingest/
│   ├── gate.rs          # + contains(&QsoEvent)                                                     [tests]
│   └── mod.rs           # + band check after normalise; removal via Pipeline::remove               [tests]
├── pipeline.rs          # + HistoryChange::Remove, Pipeline::remove, startup log of effective set  [tests]
└── web/
    ├── protocol.rs      # + ServerMessage::Remove; hello(map, filter) filters bands, keeps protocol 2 + basemap                 [tests]
    ├── session.rs       # + Remove → `remove` message                                               [tests]
    └── assets/
        ├── view.js      # + popupToClose (open popup of a removed marker)               [node tests]
        ├── map-state.js # + applyRemove                                                             [node tests]
        └── app.js       # + handle `remove` (glue only)

tests/band_filter.rs     # NEW: US1 + US3 end to end (counting lookup source, QARTest correction to 60m)
web/map-state.test.js    # + applyRemove tests
deploy/config.example.toml, README.md, specs/001-…/contracts/configuration.md   # document `bands`
```

**Structure Decision**: one new pure module, `band_filter.rs`. Everything else is additive changes to
the feature-001 modules along the existing pipeline.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] I: every functional requirement maps to a test (see Constitution Check I).
- [x] II: new inputs (`bands` entries) go through fallible parsing with structured config errors.
- [x] III: no new dependencies. Tombstones are the minimum needed to keep removals consistent with
  parallel workers (R3).
- [x] Domain rules: HR-2 is enforced by a test over every group and over the legend output.
