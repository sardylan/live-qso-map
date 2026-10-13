---
description: "Task list for 010-portable-callsigns"
---

# Tasks: Positions for Prefixed and Portable Callsigns

**Input**: Design documents from `/specs/010-portable-callsigns/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/enrichment.md, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the code it covers,
run it, and confirm it fails first. Use `cargo test`.

**Conventions:**
- **Unit tests** go in `#[cfg(test)] mod tests` of the same file.
- **Integration tests** go in the new `tests/portable_callsigns.rs`. They:
  - return `Result<(), Box<dyn std::error::Error>>` and use `?`, never
    `unwrap`/`expect`/`panic!`;
  - have no inline lint suppressions (LL-001);
  - use `live_qso_map::pipeline::start_with_country(test_config(), Some(sources), Some(country))`
    and the helpers in `tests/common/mod.rs` (`send_udp`, `next_upsert`).
- **Fake providers** are `LookupSource` closures (as in `tests/lookup_resolution.rs`). They
  answer per callsign: a map from call to `Option<LookupCandidate>`, where `None` means unknown,
  and count their calls.
- **Behaviour rows** refer to the table in `contracts/enrichment.md`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US3)

---

## Phase 1: Setup

- [X] T001 Create `tests/fixtures/cty-portable.dat`, holding entity blocks copied **verbatim** from `./cty.dat`: Fed. Rep. of Germany, France, Italy, Sardinia, Sicily, United States, Antarctica. Add a row to `tests/fixtures/README.md` naming the source and date, like the existing excerpt row.

---

## Phase 2: Foundational (blocking prerequisites)

**Entity areas with nesting (research R5), used by every story.**

- [X] T002 Write failing unit tests in `src/entity_extents.rs`:
  - `contains(bounds, lat, lon)` handles normal and 180°-crossing rectangles (`3D2`);
  - `nested("I")` holds `IS`, but not `IT9`, `DL` or `F`;
  - `nested("DL")` doesn't hold `SP`;
  - `inside("I", Cagliari)` is false; `inside("IS", Cagliari)` is true;
  - `inside("I", Rome)` is true; `inside("DL", eastern Germany 51.0N 14.5E)` is true;
  - `inside` of an unknown prefix is "unknown";
  - the embedded table nests the expected pairs: `I` ⊃ `IS`, `T7`, `HV`, `TK`; `F` ⊃ `LX`;
    `G` ⊃ `GW`; and none for `DL`. Totals aren't asserted, because they change whenever the
    bundled rectangles are regenerated.
- [X] T003 Implement it in `src/entity_extents.rs`:
  - promote `contains` out of the test module;
  - precompute `nested: HashMap<String, Vec<String>>` in `ExtentTable::parse`, using
    wrap-aware full containment;
  - add `pub fn inside(&self, prefix: &str, lat: f64, lon: f64) -> Option<bool>`, where `None`
    means no rectangle.

  Make T002 pass.

**Candidate filtering in `resolve_remote` (research R4, R6, R7), used by every story.**

- [X] T004 Write failing unit tests in `src/enrich/resolve.rs`, using a fake `accept` filter:
  - a rejected candidate loses its position, and its location fields (`GRIDSQUARE`, `LAT`,
    `LON`, `QTH`, `STATE`, `CNTY`, `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `IOTA`) aren't
    merged later, while `NAME` is;
  - `Lookup.candidates` holds every candidate to merge: accepted ones unchanged, and rejected
    ones with their position set to `None` and their location fields removed, so that their other
    fields (`NAME`, …) can still be merged;
  - `Lookup.rejections` lists the rejections in source order;
  - `Lookup.known_online` is true only when a non-`Cty` candidate exists;
  - the new `select(candidates, order)` reproduces today's results: the most precise wins, ties
    go by configured order, and fields are merged from the winner first, never overwriting.
    Port the existing selection tests to it.
- [X] T005 Change `resolve_remote` in `src/enrich/resolve.rs` to take
  `accept: &dyn Fn(&Position) -> Verdict`, and to return
  `Lookup { candidates: Vec<LookupCandidate>, rejections: Vec<(PositionSource, RejectReason)>, known_online: bool }`
  without choosing a winner.
  - **Selection:** move today's precision-then-order selection and the field merge into
    `pub fn select(candidates: &[LookupCandidate], order: &[PositionSource]) -> (Option<Position>, AdifRecord)`.
  - **New types,** in the new `src/enrich/acceptance.rs`:
    - `pub enum Verdict { Accept, Reject(RejectReason) }`;
    - `pub enum RejectReason { OutsideEntity, Placeholder }`, with `as_str` giving
      `outside-entity` and `placeholder`.

    No `Result` carries `RejectReason`: a rejection is a verdict, not an error
    (`docs/rust-guidelines.md`: one crate `Error`, no module-local error enums).
  - **Wiring:** add `pub mod acceptance;` in `src/enrich/mod.rs`. `enrich()` passes an
    accept-all filter for now and calls `select(&lookup.candidates, order)`.

  Make T004 pass, and keep every existing test green.
- [X] T006 Write a failing test, then implement `rejection_note(entries: &[(&str, bool, RejectReason)]) -> Option<String>` in `src/enrich/acceptance.rs`:
  - the format is `source[@base]:reason`, comma-separated (research R7);
  - it returns `None` for an empty list.
- [X] T007 Give `Lookups` an `areas: Arc<ExtentTable>` field in `src/enrich/mod.rs`, passed from `ExtentTable::embedded()` in `src/pipeline.rs`. Update the test helper `lookups()` in `src/enrich/mod.rs` to use the embedded table.

**Checkpoint**: areas, filtering and notes exist and are tested, with no behaviour change yet:
`cargo test` is green.

---

## Phase 3: User Story 1 - A station operating from another entity is drawn in that entity (Priority: P1) 🎯 MVP

**Goal**: an online position outside the callsign's entity (nested entities excluded) is
rejected, and the entity's reference point is used. Rejections are noted and logged.

**Independent Test**: behaviour rows 1, 2, 5 and 6 in `tests/portable_callsigns.rs`.

### Tests for User Story 1 (write first, confirm they FAIL)

- [ ] T008 [P] [US1] Write failing unit tests for `check(position, entity_prefix, online, areas) -> Verdict` in `src/enrich/acceptance.rs`:
  - a Sardinian point for `IS` gives `Verdict::Accept`; for `I` it gives
    `Verdict::Reject(OutsideEntity)`;
  - a German point for `IS` is rejected;
  - a Roman point for `I` is accepted;
  - with `online == false` (logger, cty), there is no area rejection;
  - an unknown entity is accepted.
- [ ] T009 [P] [US1] Write failing integration tests in the new `tests/portable_callsigns.rs`, with `tests/fixtures/cty-portable.dat` and fake providers:
  - **Row 1:** `IS0/DL0AAA` online in Sardinia keeps that locator.
  - **Row 2:** `IS0/DL0AAA` online at a German home gets Sardinia's reference point, precision
    "country", and `APP_LIVEQSOMAP_POS_REJECTED=qrz:outside-entity`. No `QTH`/`COUNTRY` from
    the German record; `NAME` kept. The published QSO keeps `CALL` exactly as sent
    (`IS0/DL0AAA`, FR-010), and its entity is Sardinia (`IS`, FR-011).
  - **Row 5:** `DL0AAA/IS0` behaves like row 2, with `CALL` kept as `DL0AAA/IS0` and entity
    Sardinia.
  - **Row 6:** `I/HB9AAA` with a Cagliari locator gets Italy's reference point and
    `qrz:outside-entity`. `CALL` stays `I/HB9AAA`, and the entity is Italy (`I`).
  - **Row 17:** a logger locator in France for `DL0AAA` is trusted; no lookup is made.
  - **Row 18:** for an entity without a rectangle, the online position is accepted.

### Implementation for User Story 1

- [ ] T010 [US1] Implement `check(…) -> Verdict` for the area rule in `src/enrich/acceptance.rs` (research R4 step 3), using `ExtentTable::inside`. Make T008 pass.
- [ ] T011 [US1] Wire it into `enrich()` in `src/enrich/mod.rs`:
  1. compute the logged call's entity **before** the lookups (move the existing
     `entity_info` call up);
  2. pass a filter that applies `check` with `online = (candidate source != Cty)`;
  3. write `APP_LIVEQSOMAP_POS_REJECTED` from `rejection_note` when it isn't empty;
  4. log one `tracing::warn!("position rejected", call, source, reason, round)` per rejection
     (FR-009).

  Make T009's rows 1, 2, 5, 6, 17 and 18 pass, and keep all existing tests green.

**Checkpoint**: prefixed callsigns are drawn in their prefix entity. This is the MVP.

---

## Phase 4: User Story 2 - A portable callsign with no online record uses its base callsign (Priority: P1)

**Goal**: when no online service knows the logged call, one more round runs with the base call.
The country data's answer for the base call is ignored, and `/MM` and `/AM` are never retried.

**Independent Test**: behaviour rows 3, 4 and 7–10 in `tests/portable_callsigns.rs`.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T012 [P] [US2] Write failing unit tests for `base_call` and `retry_allowed` in the new `src/enrich/callsign.rs`:
  - every row of research R2's table;
  - lowercase input, surrounding spaces, empty parts (`IS0//DL0AAA`), and `/QRPP`, `/LH`,
    `/B`;
  - a plain call gives `None`;
  - `/MM` and `/AM` aren't retried.

  Add `pub mod callsign;` in `src/enrich/mod.rs`.
- [ ] T013 [P] [US2] Write failing integration tests in `tests/portable_callsigns.rs`:
  - **Row 3:** `IS0/DL0AAA` is unknown, the base is at a German home: Sardinia reference,
    `qrz@base:outside-entity`. `CALL` stays `IS0/DL0AAA`, and the entity is Sardinia, never the
    base call's Germany.
  - **Row 4:** the base is in Sardinia: that locator is used.
  - **Row 7:** `IS0AAA/P` with its own record: no second call to the fake provider (call
    counter = 1).
  - **Row 8:** `IS0AAA/P` is unknown, `IS0AAA` is known: the base record's locator and `NAME`.
    `CALL` stays `IS0AAA/P`, and the entity comes from the logged call (Sardinia).
  - **Row 9:** `K0AAA/6` is unknown, `K0AAA` is in the US: that locator.
  - **Row 10:** `DL0AAA/MM` is unknown: no second call, no position.

### Implementation for User Story 2

- [ ] T014 [US2] Implement `base_call` and `retry_allowed` in `src/enrich/callsign.rs` (research R2). Move the suffix constants `IGNORED_SUFFIXES` and `NO_ENTITY_SUFFIXES` there as `pub(crate)`, and import them in `src/enrich/cty.rs`. Make T012 pass, and keep every `cty.rs` test green.
- [ ] T015 [US2] Add the second round in `enrich()` in `src/enrich/mod.rs` (research R3):
  1. when `!round1.known_online && retry_allowed(call)` and `base_call(call)` is `Some(base)`,
     call `resolve_remote(base, …)` with the same filter;
  2. drop the second round's candidates whose source is `Cty`, from both `candidates` and
     `rejections`;
  3. pool `round1.candidates` with `round2.candidates`, and call `select(&pooled, order)` once;
  4. note second-round rejections with `@base`.

  Make T013 pass, and keep everything else green.

**Checkpoint**: portable callsigns use the base callsign's data when theirs is unknown.

---

## Phase 5: User Story 3 - Positions outside the entity, and placeholder positions, are treated as errors (Priority: P2)

**Goal**:
- 0°/0° is rejected from every source;
- `AA00`/`AA00AA` is rejected except for Antarctica;
- a logger placeholder is ignored, and the lookups run;
- plain callsigns with an out-of-entity locator get the entity's reference point.

**Independent Test**: behaviour rows 11–16 in `tests/portable_callsigns.rs`.

### Tests for User Story 3 (write first, confirm they FAIL)

- [ ] T016 [P] [US3] Write failing unit tests for the placeholder rule in `src/enrich/acceptance.rs`:
  - exactly 0°/0° gives `Verdict::Reject(Placeholder)` for every entity and for no entity;
  - `1e-9`/`0.0` and `JJ00` (centre 0.5N 1E) are not placeholders;
  - a point inside square `AA00`, including −90°/−180°, gives `Verdict::Reject(Placeholder)`
    for `DL` and for no entity, but `Verdict::Accept` for `CE9`;
  - `-89.98`/`-179.96` (the centre of `AA00AA`) is caught;
  - the rule applies with `online == false` too.
- [ ] T017 [P] [US3] Write failing integration tests in `tests/portable_callsigns.rs`:
  - **Row 11:** plain `DL0AAA` online in France gets Germany's reference point and
    `qrz:outside-entity`.
  - **Row 12:** online 0°/0° gives `qrz:placeholder`.
  - **Row 13:** online `AA00AA` gives `qrz:placeholder`.
  - **Row 14:** `LU1ZAA` (Antarctica, prefix `LU1Z`) with `AA00AA` keeps it. `KC4AAA` is Antarctic only through a cty.dat exact-call entry, which the fixture doesn't carry.
  - **Row 15:** logger LAT/LON 0/0 gives `logger:placeholder`, the lookups run, and the online
    German locator is used.
  - **Row 16:** logger `GRIDSQUARE` `AA00` gives `logger:placeholder`; with no online answer,
    Germany's reference point.

### Implementation for User Story 3

- [ ] T018 [US3] Add the placeholder rule to `check` in `src/enrich/acceptance.rs`, before the area rule (research R4 steps 1–2). Make T016 pass.
- [ ] T019 [US3] In `enrich()` in `src/enrich/mod.rs`, check the logger position with `check(…, online = false)`. On rejection, note `logger:placeholder`, treat the position as absent, and run the lookups (FR-012). Make T017 pass (row 11 already passes with US1's filter), and keep everything else green.

**Checkpoint**: no QSO is drawn at 0°/0°, and no non-Antarctic QSO at `AA00AA`.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T020 [P] Update `docs/ham-radio.md` (research R9):
  - **HR-7:** add that a position outside the callsign's entity (nested entities excluded), or a
    placeholder (0°/0°; square `AA00` except Antarctica), is not a position.
  - **§2.2:** add a "Position looked up as" column (logged call, then the base call when no
    online service knows it; never for `/MM` and `/AM`).
  - **Changelog:** add a dated line.
- [ ] T021 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md` (`docs/releasing.md`):
  - **Fixed:** prefixed callsigns drawn in their own entity; positions at 0°/0° or `AA00AA`
    ignored.
  - **Added:** the base-callsign retry for portable callsigns, and the
    `APP_LIVEQSOMAP_POS_REJECTED` field.
- [ ] T022 [P] Add a "Rejected positions" note to the README (QSO fields section, near `APP_LIVEQSOMAP_POS_SOURCE` if documented there, otherwise in the enrichment description): what the new field means, with an example.
- [ ] T023 Run the local gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `node --test 'web/*.test.js'`, `python3 -m unittest discover -s tools -p 'test_*.py'`, and quickstart §1.
- [ ] T024 Mark the tasks done, commit per phase, push `feature/010-portable-callsigns`, and open the pull request. Merge it once the five gates are green; delete the branch afterwards (constitution, Development Workflow).

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T007)** → stories.
- **US1 (T008–T011)** first: it is the MVP, and it brings the area filter that US2's row 3 and
  US3's row 11 rely on.
- **US2 (T012–T015)** after US1. T012 and T014 (the `callsign` module) can be written in
  parallel with US1, since they touch other files.
- **US3 (T016–T019)** after US1. T016 and T018 only touch `acceptance.rs`, after T010.
- **Polish (T020–T024)** last.
- **Within each story**: tests → implementation → keep all existing tests green.
- **Shared files**:
  - `src/enrich/mod.rs` is edited by T005, T007, T011, T015 and T019, strictly in order;
  - `src/enrich/acceptance.rs` by T005, T006, T008, T010, T016 and T018;
  - `tests/portable_callsigns.rs` by T009, T013 and T017.

## Parallel Opportunities

- **Foundational**: T002–T003 (`entity_extents.rs`) and T004–T005 (`resolve.rs`) are separate
  files.
- **US1**: T008 (unit) and T009 (integration) in parallel.
- **US2**: T012 (`callsign.rs`) can start alongside US1; T013 alongside T012.
- **US3**: T016 and T017 in parallel.
- **Polish**: T020, T021 and T022 touch different files.

### Parallel example: User Story 2

```text
Task: "T012 [US2] callsign.rs unit tests"
Task: "T013 [US2] integration rows 3, 4, 7–10 in tests/portable_callsigns.rs"
```

## Implementation Strategy

1. **MVP (US1)**: Phases 1–3. Prefixed callsigns land in the right entity, with rejections noted
   and logged. Validate with rows 1, 2, 5, 6, 17 and 18.
2. **US2**: the base-callsign retry for portable callsigns.
3. **US3**: placeholders and the logger path.
4. **Polish**: domain document, changelog, README, gates, pull request.
