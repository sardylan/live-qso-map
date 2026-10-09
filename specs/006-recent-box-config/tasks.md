---
description: "Task list for 006-recent-box-config"
---

# Tasks: Configurable Last-QSO Box

**Input**: Design documents from `/specs/006-recent-box-config/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation
it covers, run it, and confirm it fails first. Use `cargo test` for Rust and `node --test web/`
for the frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`).

**Conventions** (as in features 001–005):

- **Errors**: the crate `Error`/`Result` (`docs/rust-guidelines.md`). Configuration errors are
  `Error::Config { setting, message }`.
- **Rules**:
  - domain rules: `docs/ham-radio.md` §1 (HR-1 UTC, HR-5 callsigns, HR-6 entities);
  - interface rules: `docs/http-api.md` (additive JSON fields, arrays for ordered data);
  - constitution Principle IV: every viewer setting is in `localStorage`, with defaults when
    reading fails.
- **Page code**: decisions live in pure modules with `web/*.test.js` tests; `app.js` is glue only.
  QSO text reaches the DOM only through `textContent` and `title`.
- **Rust tests**:
  - unit tests go in `#[cfg(test)] mod tests` in the same file;
  - integration tests start with `#![allow(clippy::unwrap_used, clippy::expect_used,
    clippy::panic)]` and use `tests/common`.
- **Commits**: one commit per phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US2 from spec.md

---

## Phase 1: Setup

- [X] T001 Create `src/recent_columns.rs` with the module doc comment
  `//! Last-QSO box columns chosen by the operator (feature 006, research R2).`, and add
  `pub mod recent_columns;` to `src/lib.rs`. `cargo build` must pass.

---

## Phase 2: User Story 1 - The operator chooses the box columns (Priority: P1) 🎯 MVP

**Goal**:
- the configured columns (default `call, band, time, cqzone, country`) appear in that order;
- an optional grey header row labels them;
- the CQ zone comes from cty.dat;
- an invalid setting stops startup (FR-001 to FR-006a, FR-013).

**Independent Test**: quickstart §2.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T002 [P] [US1] Write unit tests in `src/recent_columns.rs` (data-model "Column"):
  - **parsing**:
    - `parse(["call","band","time","cqzone","country"])` gives `DEFAULT_COLUMNS`;
    - `[" Call ", "COUNTRY"]` gives `[Call, Country]`;
    - all seven keywords in a non-default order are kept in that order;
  - **errors**, each `Error::Config { setting: "map.recent_columns", .. }` with the message from
    `contracts/configuration.md`:
    - `["call","freq"]` (unknown, the message names `freq` and lists the valid keywords);
    - `["call","call"]` (repeated);
    - `[]` and `[" ", ""]` (no columns);
  - **JSON**: `Column` serialises as its keyword string.
- [X] T003 [P] [US1] Extend the unit tests in `src/config.rs` (contracts/configuration.md):
  - **defaults**: `map.recent_columns` is `DEFAULT_COLUMNS` and `map.recent_header` is `true`;
  - **TOML**: `[map] recent_columns = ["country","call"]` and `recent_header = false` are applied;
  - **environment**: `LQM_RECENT_COLUMNS="call, mode"` replaces the TOML list;
  - **command line**: `--recent-columns date --recent-columns time` replaces both, and so does
    `--recent-header true`;
  - **errors**: an invalid list gives `Error::Config { setting: "map.recent_columns" }` from any
    layer; `LQM_RECENT_HEADER=maybe` gives `Error::Config { setting: "map.recent_header" }`.
- [X] T004 [P] [US1] Write unit tests in `src/enrich/cty.rs` for
  `CountryData::entity_info(call)`. Use `tests/fixtures/cty-excerpt.dat`, plus a new small
  fixture, `tests/fixtures/cty-zones.dat`. It holds the real cty.dat headers of Canada (`VE`,
  zone 5) and Asiatic Russia (`UA9`, zone 17), with the aliases `VE`, `VY1(1)`, `UA9` and
  `R8H(18)`:
  - **entity and zone**: `K1ABC` gives `K` / `United States` / 5; `IS0MZK` gives `IS` /
    `Sardinia` / 15;
  - **overrides**: `VY1ABC` gives zone 1 under Canada; `R8HAA` gives 18;
  - **exact calls**: `KH6QZ` gives `K`, with that entry's zone override when present;
  - **unknown call**: `None`;
  - **consistency (SC-007)**: for 20 callsigns, `entity_info(c).cq_zone` equals the `CQZ` that
    `lookup(c)` returns.
- [X] T005 [P] [US1] Extend the tests in `src/enrich/mod.rs`:
  - `enrich()` sets `entity.cq_zone`, for example `K1ABC` gives 5;
  - a corrected callsign gets the new zone.
- [X] T006 [P] [US1] Extend the tests in `src/web/protocol.rs`:
  - `hello` contains `"recent": {"columns": ["call","band","time","cqzone","country"],
    "header": true}` for the default `MapConfig`, and follows a custom configuration;
  - the QSO `entity` serialises as `{"prefix","name","cq_zone"}`;
  - `protocol` is still 3.
- [X] T007 [P] [US1] Extend `tests/ws_protocol.rs`:
  - over a real WebSocket, `hello.recent` matches `test_config()`;
  - with `start_with_country` and the fixture, an upsert for `K1ABC` carries
    `entity.cq_zone == 5`.
- [X] T008 [P] [US1] Extend `web/recent.test.js` (data-model "Box"):
  - **`headerCells`**: `headerCells(['call','band','time','cqzone','country'])` gives the labels
    Call, Band, UTC, CQ, Country, in order; every keyword has its label (Mode, Date);
  - **`rowCells`**:
    - for each of the seven keywords: `call`, `band`, `mode` (with submode), `date`
      (`YYYY-MM-DD`), `time` (`HH:MMZ`), `cqzone` (`'5'`, or `'—'` when unknown), `country`
      (the cut label, with the full label as `title`, or `'—'`);
    - the cells follow the order of the column list;
    - missing values give `''` (`mode`, `date`, `time`, `band`) or `'—'` (`cqzone`, `country`);
  - the text is returned verbatim.

### Implementation for User Story 1

- [X] T009 [US1] Implement `Column` (with `keyword()` and serialisation as the keyword),
  `DEFAULT_COLUMNS` and `parse()` in `src/recent_columns.rs`. T002 must pass.
- [X] T010 [US1] In `src/config.rs`:
  - add `recent_columns: Vec<Column>` and `recent_header: bool` to `MapConfig`, with defaults;
  - add the TOML keys and the CLI flags, `--recent-columns` (repeatable, comma-separated) and
    `--recent-header <true|false>`, with their env names in the help text. `--recent-header`
    takes a value, as in `contracts/configuration.md`; it is not a bare flag;
  - layer them like `bands`: the highest layer replaces the whole list, and the header accepts
    only `true` or `false`.

  T003 must pass.
- [X] T011 [US1] Rename `CountryData::entity_of` to `entity_info` in `src/enrich/cty.rs`. It now
  returns `EntityInfo { prefix, name, cq_zone }`, with `cq_zone = overrides.cq.unwrap_or(e.cq)`.
  Then:
  - add `cq_zone: u8` to `QsoEntity` in `src/history.rs`;
  - fill it in `enrich()` in `src/enrich/mod.rs`;
  - update `station_entity` in `src/pipeline.rs` and every caller and test.

  T004 and T005 must pass, and so must every feature-004/005 test.
- [X] T012 [US1] In `src/web/protocol.rs`, add `RecentView { columns: Vec<Column>, header: bool }`
  and `ServerMessage::Hello.recent`, built from `MapConfig` in `ServerMessage::hello`. T006 and
  T007 must pass.
- [X] T013 [US1] Implement `headerCells` and `rowCells` in `src/web/assets/recent.js`, reusing
  `shortName` and `entityLabel`. Remove the feature-005 fixed `rowView`, or keep it as a wrapper
  over `rowCells`, and update its tests. T008 must pass.
- [X] T014 [US1] Glue in `src/web/assets/app.js` and `src/web/assets/app.css`:
  - keep `hello.recent`;
  - in `renderRecent()`, build one `<table>`, with a `<thead>` row from `headerCells` when
    `header` is true, and a `<tbody>` row per QSO from `rowCells`;
  - give each cell a class named after its column, and set `title` when present;
  - style it: no borders; `call` bold; `country` grey (spec FR-005); the header row smaller and
    grey;
  - the empty-state message replaces the whole table (FR-006a);
  - the feature-004/005 clickable-row behaviour is unchanged.
- [ ] T015 [US1] Manually run quickstart §2, including the startup errors and the CQ zones
  (SC-007). Record the results in `specs/006-recent-box-config/checklists/verification.md`
  (create the file).

**Checkpoint**: the operator's columns and header work; US1 is deliverable on its own.

---

## Phase 3: User Story 2 - The viewer chooses the box size and order (Priority: P1)

**Goal**:
- the top-right panel offers map "Last N", box size and newest top or bottom;
- all three are saved in local storage;
- the box is independent of the map window, and its off-map rows are faded (FR-007 to FR-012).

**Independent Test**: quickstart §3.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T016 [P] [US2] Extend `web/map-state.test.js` (data-model "Map state"):
  - **map and server windows**: with 25 QSOs held, `mapLimit` 10 and a snapshot `limit` of 20,
    `visible()` gives the newest 10 and `held()` gives all 25, both ascending;
  - **All**: `setMapLimit(state, null)` makes `visible()` return every QSO;
  - **`serverLimit`**: 10 → 20, 20 → 20, 50 → 50, `null` → `null`;
  - **separation**: applying a snapshot does not change `mapLimit`.
- [ ] T017 [P] [US2] Extend `web/recent.test.js`:
  - **`recentQsos(state, size, order)`**:
    - size 5 with `'bottom'` gives the newest 5 ascending (newest last);
    - `'top'` gives them descending;
    - size 20 with only 8 held gives 8;
    - QSOs beyond `mapLimit` are included (FR-009);
  - **`onMap(qso, markers)`**: true for a QSO whose marker key is in `markers(state)`; false for
    a QSO outside the map window or without a remote position;
  - **SC-003**: a table-driven test with 25 QSOs held. For each map size (10, 20, 50, `null`) and
    each box size (3, 5, 10, 15, 20), `recentQsos` returns exactly min(box size, 25) rows, and
    `visible` returns exactly min(map size, 25).
- [ ] T018 [P] [US2] Extend `web/view.test.js` and `web/connection.test.js`:
  - **`loadRecentSize`/`saveRecentSize`** (`lqm.recent.size`): the default is 5; 3, 10, 15 and 20
    round-trip; an unknown value or `'7'` gives 5; a storage that throws gives 5, and saving
    doesn't throw;
  - **`loadRecentOrder`/`saveRecentOrder`** (`lqm.recent.order`): the default is `'bottom'`;
    `'top'` round-trips; unknown values or a throwing storage give `'bottom'`;
  - **`loadLimit`/`saveLimit`** keep their behaviour (default 20, `'all'` gives `null`) whatever
    storage is passed in. This guards the move to local storage.

### Implementation for User Story 2

- [ ] T019 [P] [US2] Implement `mapLimit`, `held`, `setMapLimit` and `serverLimit` in
  `src/web/assets/map-state.js`. `visible()` uses `mapLimit`; `setLimit()` and snapshots keep the
  server window only. Update the feature-001 tests that relied on `setLimit` also driving the map.
  T016 must pass.
- [ ] T020 [US2] After T019, which provides `held()`, implement
  `recentQsos(state, size, order)` (over `held`) and `onMap` in
  `src/web/assets/recent.js`. T017 must pass.
- [ ] T021 [P] [US2] Implement the four new helpers in `src/web/assets/view.js`. `connection.js`
  needs no change. T018 must pass.
- [ ] T022 [US2] In `src/web/assets/index.html`, turn the top-right `.limit-control` into a panel
  with three labelled selects:
  - "Map: last [10…500, All] QSOs";
  - "Box: last [3, 5, 10, 15, 20]";
  - "Newest: [bottom, top]".

  Style it in `src/web/assets/app.css` as one compact panel, as today, with the map selector
  visibly separate from the box settings.
- [ ] T023 [US2] Glue in `src/web/assets/app.js`:
  - **storage**: read and save the map limit with `localStore()` instead of `sessionStore()`
    (Principle IV); remove `sessionStore` if it is no longer used;
  - **map limit**: on change, `setMapLimit`, re-render the map and box, `autozoom.change()`, and
    send `set_limit` with `serverLimit(mapLimit)`. The handshake sends the same value;
  - **box size and order**: load them, save them on change, and re-render the box at once
    (FR-012);
  - **box rows**: `renderRecent()` uses `recentQsos(state, size, order)`. A row whose QSO is not
    `onMap` gets the class `off-map` (faded, `opacity: 0.55` in `app.css`) and the title
    `Not on the map (outside its last N)`, with no click handler, no `tabindex` and no `role`
    (FR-010);
  - **snapshots**: a snapshot no longer changes the map selector's value.
- [ ] T024 [US2] Manually run quickstart §3, including a browser restart, the 1024/1920 px
  alignment (SC-005) and step 6 (settings changed while the box is hidden). Record the results
  in `checklists/verification.md`.

---

## Phase 4: Polish & Cross-Cutting Concerns

- [ ] T025 [P] Documentation:
  - **`deploy/config.example.toml`**: commented `recent_columns` and `recent_header` under `[map]`,
    with the keyword list;
  - **`README.md`**: the configuration table rows; the "Last QSOs" text (operator columns, header,
    the viewer's box size and order, everything remembered across restarts);
  - **`specs/001-live-qso-map/contracts/configuration.md`**: a note pointing to
    `specs/006-recent-box-config/contracts/configuration.md`;
  - **`specs/001-live-qso-map/contracts/websocket-protocol.md`**: a note pointing to
    `specs/006-recent-box-config/contracts/websocket-protocol.md`.
- [ ] T026 Run the complete gates: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` and `node --test web/`. Complete
  `checklists/verification.md` with the automated results and SC-001 to SC-007:
  - SC-001 and SC-002 from T002, T003 and T015;
  - SC-003 from T016, T017 and T024;
  - SC-004 to SC-006 from T024;
  - SC-007 from T004.

---

## Dependencies & Execution Order

- **Setup (T001)** comes first.
- **US1 (T002–T015)** depends on Setup. This is the MVP.
  - T011 touches `cty.rs`, `history.rs`, `enrich/mod.rs` and `pipeline.rs`, so it runs after
    T004 and T005 are written.
  - T014 needs T012 and T013.
- **US2 (T016–T024)** depends on US1 for `renderRecent()` and `rowCells`.
  - T016, T018, T019 and T021 may start in parallel with US1, because they touch other files.
  - T017 and T020 edit `recent.test.js` and `recent.js` after T008 and T013.
  - T023 runs after T014, since both edit `app.js`.
- **Polish (T025–T026)** comes last.

```text
T001 → US1 (T002–T015) → US2 (T016–T024) → Polish (T025–T026)
         └ T016, T018, T019, T021 may start in parallel with US1 (other files);
           T017, T020 follow T008, T013 (same files)
```

---

## Parallel Opportunities

- **US1 tests**: T002–T008 are seven different files and can all run in parallel.
- **US2**: T016, T017 and T018; then T019 and T021 together, and T020 after T019.
- **Polish**: T025 runs alongside the manual checks.

### Parallel example: User Story 1

```text
Together:  T002 recent_columns · T003 config · T004 cty · T005 enrich · T006 protocol · T007 ws · T008 recent.js
Then:      T009 → T010;  T011;  T012;  T013
Then:      T014 app.js glue → T015 manual check
```

---

## Implementation Strategy

1. **MVP** (Setup and US1): operator-chosen columns with an optional header; the CQ zone from
   cty.dat; startup validation.
2. **US2**: the viewer panel with map, box size and order, all in local storage, and a box
   independent of the map, with faded off-map rows.
3. **Polish**: documentation, gates and the verification note.
