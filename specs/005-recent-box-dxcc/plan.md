# Implementation Plan: DXCC Entity and Date in the Last-QSO Box

**Branch**: `005-recent-box-dxcc` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-recent-box-dxcc/spec.md`

## Summary

Every QSO now carries its DXCC/WAE entity: the cty.dat primary prefix and name, resolved from the
remote callsign when the QSO is enriched, and stored with it (research R1).

- **Pages**: they receive the entity as an additive `entity` field on each QSO. Protocol 3 is
  unchanged (R3).
- **Last-QSO box**: the mode is dropped. Each row shows the UTC date and time, then
  `prefix · name`, with long names cut at 18 characters and the full name on hover (R4).
- **Marker popups**: each QSO row shows the uncut entity (R5).

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules
**Primary Dependencies**: none new
**Storage**: none new; the entity is stored on the enriched QSO in the in-memory history
**Testing**: `cargo test` and `node --test web/`
**Target Platform**: unchanged
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**:

- one extra hash lookup per enriched QSO;
- nothing extra per page or snapshot;
- SC-004: the entity is on the page within the existing 2 s.

**Constraints**:

- HR-1: UTC date and time;
- HR-6: cty.dat entities, with the WAE `*` stripped;
- plain-text rendering (feature 004, FR-022);
- `docs/http-api.md`: JSON, `snake_case`, `null` for absent values.

**Scale/Scope**: five box rows; popups with every QSO at a marker

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests are written first for:
  - Rust:
    - `enrich()` filling `entity`, including WAE, portable and exact-call cases, and with no
      country data;
    - a correction changing the entity;
    - the `QsoView` JSON with an entity and with `null`;
    - an end-to-end check that a page receives the entity.
  - JS:
    - `shortName`, `entityLabel` and the new `rowView` (date, time, entity, no mode).
- [x] **II. Rust Safety & Quality**: no `unsafe`; `entity_of` returns an `Option`, so there is no
  new error path. fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**: no new crates; the existing `CountryData` is
  reused.
- [x] **Domain rules** (`docs/ham-radio.md` §1):
  - HR-1: the date and time are UTC;
  - HR-5: `entity_of` compares callsigns ignoring case;
  - HR-6: entities come from cty.dat (DXCC/WAE).
- [x] **Interface rules** (`docs/http-api.md`): the field is added to JSON text frames, uses
  `snake_case` and is `null` when absent (API-2, API-4). The protocol version stays 3, because the
  change only adds a field.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/005-recent-box-dxcc/
├── plan.md
├── research.md          # R1–R5
├── data-model.md        # QsoEntity, EnrichedQso.entity, Lookups.country, recent.js helpers
├── quickstart.md
├── contracts/
│   └── websocket-protocol.md   # QSO object gains `entity`; protocol stays 3
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── history.rs           # QsoEntity; EnrichedQso.entity
├── enrich/mod.rs        # Lookups.country; enrich() fills entity                          [tests]
├── pipeline.rs          # passes the country data to Lookups; start_with_country (tests)
└── web/
    ├── protocol.rs      # QsoView.entity                                                  [tests]
    └── assets/
        ├── recent.js    # shortName, entityLabel; rowView: date, time, entity, no mode  [node tests]
        ├── app.js       # box columns and entity title; popup entity cell
        └── app.css      # entity column style

web/recent.test.js       # extended
tests/ws_protocol.rs     # a page receives `entity` (start_with_country + cty fixture)
(every EnrichedQso literal in src/ and tests/ gains `entity: None`)
specs/004-qth-autozoom/contracts/websocket-protocol.md   # note pointing to this delta
```

**Structure Decision**: unchanged. Decisions stay in pure modules, and `app.js` remains glue.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] **I**: every new rule has a planned test that is written first: the Rust entity resolution
  and JSON shape, and the JS cutting and row content.
- [x] **II**: no new error paths and no panics.
- [x] **III**: no new crates; the data source is unchanged.
- [x] **Domain and interface rules**: HR-1, HR-5 and HR-6 are respected. The field is additive
  and `null` when absent.
- [x] **Security**: entity names reach the DOM only through `textContent` and `title`.
