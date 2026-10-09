---
description: "Task list for 005-recent-box-dxcc"
---

# Tasks: DXCC Entity and Date in the Last-QSO Box

**Input**: Design documents from `/specs/005-recent-box-dxcc/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation it
covers, run it, and confirm it fails first. Use `cargo test` for Rust and `node --test web/` for
the frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`).

**Conventions** (as in features 001–004):

- **Errors**: the crate `Error`/`Result` from `src/errors.rs` (`docs/rust-guidelines.md`).
- **Domain rules**: `docs/ham-radio.md` §1, in particular HR-1 (UTC), HR-5 (callsign
  comparison) and HR-6 (DXCC/WAE entities).
- **Interface rules**: `docs/http-api.md` (JSON, `snake_case`, `null` for absent values).
- **Page code**: decisions live in pure modules with `web/*.test.js` tests; `app.js` is glue only.
  QSO text reaches the DOM only through `textContent` and `title`.
- **Rust tests**: unit tests go in `#[cfg(test)] mod tests` in the same file. Integration tests
  start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` and use
  `tests/common`.
- **Commits**: one commit per phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US2 from spec.md

---

## Phase 1: Setup

- [ ] T001 Add `QsoEntity { prefix: String, name: String }` (with `Debug`, `Clone`, `PartialEq` and
  `Serialize`) to `src/history.rs`, and the field `entity: Option<QsoEntity>` to `EnrichedQso`.
  Add `entity: None` to every `EnrichedQso { … }` literal in `src/history.rs`,
  `src/pipeline.rs`, `src/web/session.rs`, `src/web/protocol.rs` and `src/enrich/mod.rs`.
  `cargo build --all-targets` must pass, and every existing test must still pass.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: every QSO sent to pages carries its cty.dat entity (FR-001 to FR-005). Both stories
read it.

### Tests first (write, run, confirm they FAIL)

- [ ] T002 [P] Write unit tests in `src/enrich/mod.rs`. Build `Lookups` with
  `CountryData::parse(include_str!("../../tests/fixtures/cty-excerpt.dat"))` and no sources:
  - `K1ABC` → `Some(QsoEntity { prefix: "K", name: "United States" })`;
  - `is0mzk` → `IS` / `Sardinia` (HR-5);
  - `IT9ABC` → `IT9` / `Sicily` (the WAE `*` is stripped);
  - `IS0MZK/P` → `IS` (portable);
  - `QQ9QQ` → `None`;
  - with `country: None` → `None`;
  - **position source does not matter (FR-002)**: a record with `GRIDSQUARE`, so the logger gives
    the position, still gets its entity;
  - **corrections (FR-004)**: enriching a second event with the same id and `CALL=K1ABC`, after
    `IS0MZK`, gives `K`.
- [ ] T003 [P] Write unit tests in `src/web/protocol.rs`:
  - `QsoView::from` of a QSO with an entity serialises
    `"entity": {"prefix": "IS", "name": "Sardinia"}`;
  - without an entity, it serialises `"entity": null` (contracts/websocket-protocol.md);
  - `hello.protocol` is still 3.
- [ ] T004 [P] Add `a_page_receives_each_qso_entity` to `tests/ws_protocol.rs`:
  - start the service with `pipeline::start_with_country(test_config(), Some(vec![]),
    Some(Arc::new(CountryData::parse(fixture))))`;
  - send ADIF QSOs for `K1ABC` and `QQ9QQ`;
  - assert that the upserts and a later snapshot carry `{"prefix": "K", "name": "United States"}`
    and `null`.

### Implementation

- [ ] T005 In `src/enrich/mod.rs`:
  - add `country: Option<Arc<CountryData>>` to `Lookups`, with
    `Lookups::new(sources, country, config)`;
  - in `enrich()`, after the record is final, set
    `entity = lookups.country.as_ref().and_then(|c| c.entity_of(call))` as a `QsoEntity`.

  T002 must pass.
- [ ] T006 In `src/pipeline.rs`:
  - pass the country data from `build_sources` to `Lookups::new`;
  - add `pub async fn start_with_country(config, sources, country)`, and make `start` delegate to
    it with no injected country (research R2);
  - update the existing `Lookups::new` callers, including tests.
- [ ] T007 Add `entity: Option<QsoEntity>` to `QsoView` in `src/web/protocol.rs`, copied in
  `From<&EnrichedQso>`. Keep `PROTOCOL_VERSION = 3` (research R3). T003 and T004 must pass.

**Checkpoint**: pages receive `entity` on every QSO; the existing page ignores it.

---

## Phase 3: User Story 1 - See where each recent QSO is (Priority: P1) 🎯 MVP

**Goal**: box rows show callsign, band, UTC date, UTC time and `prefix · name`, with no mode
(FR-006 to FR-009).

**Independent Test**: quickstart §2.

### Tests for User Story 1 (write first, confirm they FAIL)

- [ ] T008 [P] [US1] Extend `web/recent.test.js` (data-model "Frontend"):
  - **`shortName`**:
    - `shortName('Japan')` → `'Japan'`;
    - an 18-character name is unchanged;
    - `shortName('Central African Republic')` → `'Central African R…'` (17 characters + `…`);
    - a cut that ends in a space is trimmed before the `…` (`'Tristan da Cunha & Gough'` →
      `'Tristan da Cunha…'`);
  - **`entityLabel`**: `{prefix: 'JA', name: 'Japan'}` → `'JA · Japan'`, and `null` → `'—'`;
  - **`rowView`**:
    - it returns `{call, band, date, time, entity, entityTitle, markerKey}`, with no `mode` key;
    - for `datetime: '2026-10-09T15:04:00Z'` the date is `'2026-10-09'` and the time `'15:04Z'`;
    - a missing `datetime` gives `''` for both;
    - for a long name, `entity` is cut and `entityTitle` is the full label;
    - an unknown entity gives `entity: '—'` and `entityTitle: ''`;
    - two QSOs either side of midnight UTC have different dates;
  - update the feature-004 `rowView` tests that expected `mode`.

### Implementation for User Story 1

- [ ] T009 [US1] Implement `shortName`, `entityLabel` and the new `rowView` in
  `src/web/assets/recent.js`. T008 must pass.
- [ ] T010 [US1] Glue the box:
  - in `renderRecent()` in `src/web/assets/app.js`, the columns are call, band, date, time and
    entity. The entity cell gets the class `entity` and `title = entityTitle`, all through
    `textContent` and `title`;
  - in `src/web/assets/app.css`, add `.recent .entity` with no wrap.

  Feature-004 behaviour is unchanged (FR-009).
- [ ] T011 [US1] Manually run quickstart §2, including the 1024 px width check (SC-003) and the
  run with no country data (SC-002). Record the results in
  `specs/005-recent-box-dxcc/checklists/verification.md` (create the file).

**Checkpoint**: the box shows date, time and entity; US1 is deliverable on its own.

---

## Phase 4: User Story 2 - Entity in the marker popup (Priority: P2)

**Goal**: each QSO row in a marker popup also shows `prefix · name`, uncut (FR-010).

**Independent Test**: quickstart §3.

### Tests for User Story 2

`entityLabel`, which US2 uses, is covered by T008. The popup refresh on corrections is the
feature-003/004 path (`diffLayers` compares `[id, rev]`), which already has tests. No new test
file is needed.

### Implementation for User Story 2

- [ ] T012 [US2] In `popupContent()` in `src/web/assets/app.js`, add a cell with
  `entityLabel(qso.entity)`, via `textContent`, to each row, keeping mode, time and source. Add
  styling in `src/web/assets/app.css` if needed.
- [ ] T013 [US2] Manually run quickstart §3, including correcting a QSO while its popup is open,
  and record the result in `checklists/verification.md`.

---

## Phase 5: Polish & Cross-Cutting Concerns

- [ ] T014 [P] Documentation:
  - in `README.md`, under "The map page" → **Last QSOs**, list the new row content: date, time
    (UTC) and the DXCC entity from cty.dat, with long names cut and shown in full on hover;
  - add a note pointing to `specs/005-recent-box-dxcc/contracts/websocket-protocol.md` in
    `specs/004-qth-autozoom/contracts/websocket-protocol.md`.
- [ ] T015 Run the complete gates (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`). Complete
  `checklists/verification.md` with the automated results and SC-001 to SC-004.

---

## Dependencies & Execution Order

- **Setup (T001)**, then **Foundational (T002–T007)**, which blocks both stories.
- **US1 (T008–T011)** depends on Foundational. This is the MVP.
- **US2 (T012–T013)** depends on Foundational and on T009 (`entityLabel`). Its glue edits
  `app.js` after T010.
- **Polish (T014–T015)** comes last.

```text
T001 → T002–T007 → US1 (T008–T011) → US2 (T012–T013) → Polish (T014–T015)
```

---

## Parallel Opportunities

- **Foundational tests**: T002, T003 and T004 are in different files.
- **Implementation**: T005 and T007 are in different files, but T007 needs T001's `QsoEntity`
  and T006 needs T005's `Lookups::new`.
- **Polish**: T014 can run alongside the manual checks.

### Parallel example: Foundational

```text
Together:  T002 enrich tests · T003 protocol tests · T004 ws_protocol test
Then:      T005 enrich → T006 pipeline;  T007 protocol
```

---

## Implementation Strategy

1. **MVP** (Setup, Foundational, US1): every QSO carries its entity, and the box shows date, time
   and entity without the mode.
2. **US2**: popups show the entity too.
3. **Polish**: documentation, gates and the verification note.
