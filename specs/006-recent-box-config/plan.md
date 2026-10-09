# Implementation Plan: Configurable Last-QSO Box

**Branch**: `006-recent-box-config` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-recent-box-config/spec.md`

## Summary

**Operator settings**: two new settings, `map.recent_columns` and `map.recent_header`, choose the
box columns, their order and whether the header row is shown.
- They are validated at startup like `bands` (research R2).
- They reach pages in `hello.recent`.

**CQ zone**: it joins the feature-005 entity, from the same cty.dat match and with the
per-callsign overrides (R3).

**Viewer controls**: the top-right panel offers map "Last N", box size and newest top or bottom.
All three are saved in local storage (Principle IV, R5).
- The page asks the server for `max(map, 20)` QSOs, so the box (at most 20 rows) is always
  covered without a protocol change (R1).
- The map and the box then take their own windows locally.

**Box rendering**: a borderless table with an optional header (R4). Rows outside the map window
are faded and can't be clicked.

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules
**Primary Dependencies**: none new
**Storage**: browser `localStorage` for `lqm.limit` (moved from session storage),
`lqm.recent.size` and `lqm.recent.order`; no server storage
**Testing**: `cargo test` and `node --test web/`
**Target Platform**: unchanged
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**: at most 10 extra QSOs per page when the map is below 20; the box re-renders
in the same pass as the map (SC-004: within 2 s)
**Constraints**:
- Principle IV: every viewer setting is in local storage;
- interface rules: additive JSON fields, ordered arrays, protocol 3;
- HR-1 (UTC) and HR-6 (entity);
- plain-text rendering.

**Scale/Scope**: up to 20 box rows

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests come first.
  - **Rust**:
    - `recent_columns::parse`: valid lists, case and spaces, unknown keywords, repeats, empty;
    - config layering for both settings, including invalid header values;
    - `entity_info` with zone overrides (VY1, R8H) and exact calls;
    - the `hello.recent` and `entity.cq_zone` JSON;
    - an end-to-end check that a page receives `recent` and `cq_zone`.
  - **JS**:
    - `visible`/`held`/`serverLimit`;
    - `recentQsos` with size and order;
    - `headerCells`/`rowCells` for every column;
    - `onMap`;
    - every new load/save helper, including storage that throws.
- [x] **II. Rust Safety & Quality**: no `unsafe`; the new settings fail through
  `Error::Config`; fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**: no new crates. There is no protocol change for
  the box window (R1): it reuses `set_limit`.
- [x] **IV. Persistent Viewer Settings**: map "Last N", box size and order are all kept in local
  storage. Reads fall back to the defaults. Feature 001's session-only limit is fixed here.
- [x] **Domain rules**: HR-1 (UTC date and time), HR-5 (callsigns compared ignoring case), HR-6
  (entity and CQ zone from cty.dat).
- [x] **Interface rules**: `hello.recent` and `entity.cq_zone` are additive JSON fields in
  `snake_case`, and the ordered column list is an array (API-4). The protocol stays 3.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/006-recent-box-config/
├── plan.md
├── research.md          # R1–R5
├── data-model.md        # Column, MapConfig fields, EntityInfo.cq_zone, page state and helpers
├── quickstart.md
├── contracts/
│   ├── configuration.md         # map.recent_columns, map.recent_header
│   └── websocket-protocol.md    # hello.recent, entity.cq_zone; set_limit = max(map, 20)
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── recent_columns.rs    # NEW: Column, DEFAULT_COLUMNS, parse()                        [tests]
├── lib.rs               # pub mod recent_columns
├── config.rs            # MapConfig.recent_columns/.recent_header; CLI, env, TOML layering [tests]
├── enrich/cty.rs        # entity_info() with cq_zone (replaces entity_of)                [tests]
├── enrich/mod.rs        # QsoEntity.cq_zone from entity_info                             [tests]
├── pipeline.rs          # station_entity uses entity_info
├── history.rs           # QsoEntity.cq_zone
└── web/
    ├── protocol.rs      # Hello.recent (RecentView)                                      [tests]
    └── assets/
        ├── index.html   # top-right panel: map, box size, order selects; box table
        ├── app.css      # header row, off-map rows, panel layout
        ├── app.js       # glue: panel, local storage, set_limit(max(map, 20)), box table
        ├── map-state.js # mapLimit, held(), setMapLimit(), serverLimit()                 [node tests]
        ├── recent.js    # recentQsos(size, order), headerCells, rowCells, onMap           [node tests]
        ├── view.js      # load/saveRecentSize, load/saveRecentOrder                       [node tests]
        └── connection.js # unchanged API; loadLimit/saveLimit now get local storage

web/map-state.test.js, web/recent.test.js, web/view.test.js, web/connection.test.js   # extended
tests/ws_protocol.rs     # hello.recent; entity.cq_zone over a real WebSocket
deploy/config.example.toml, README.md                     # the two settings, the panel
specs/001-live-qso-map/contracts/{configuration,websocket-protocol}.md   # notes pointing to 006
```

**Structure Decision**: unchanged. The column parsing is a small pure module, like
`band_filter.rs`. Page decisions stay in pure modules, and `app.js` remains glue.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] **I**: every new rule maps to a planned test that is written first.
- [x] **II**: configuration errors are structured, with no panics.
- [x] **III**: no new crates or protocol messages.
- [x] **IV**: three viewer settings, all in local storage, with tested fallbacks.
- [x] **Domain and interface rules**: respected. Columns travel as an ordered array, and the
  fields are additive.
- [x] **Security**: QSO text, including the CQ zone and entity names, reaches the DOM only
  through `textContent` and `title`. The header labels are constants.
