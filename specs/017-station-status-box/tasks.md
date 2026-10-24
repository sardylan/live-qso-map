---
description: "Task list for 017-station-status-box"
---

# Tasks: Station Status Box

**Input**: Design documents from `/specs/017-station-status-box/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation
it covers, run it, and confirm it fails first. Use `cargo test` for Rust and `node --test web/`
for the frontend.

**Conventions** (as in features 001–015):

- **Errors**: the crate `Error`/`Result` (`docs/rust-guidelines.md`); nothing in this feature is
  fallible, so no new error variant.
- **Rules**: `docs/ham-radio.md` §1 (HR-1 UTC); `docs/http-api.md` (API-2 typed JSON frames, API-4
  `snake_case`, UTC with `Z`, `null` for missing values; an additive message keeps protocol 4).
- **Page code**: decisions live in pure modules with `web/*.test.js` tests; `app.js` is glue only.
  Status values reach the DOM only through `textContent` and `title`.
- **Rust tests**: unit tests in `#[cfg(test)] mod tests` in the same file; integration tests use
  `tests/common`.
- **Commits**: one commit per phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US2 from spec.md

---

## Phase 1: Setup

- [X] T001 Create `src/web/assets/station.js` with the header comment
  `// Pure station-status helpers (feature 017, research R5, R6). No DOM.` and an empty export, so
  later tests import it. `node --test web/` must still pass.
- [X] T002 [P] Add `--operator CALL` (default empty) to `scripts/simulate_qartest_qso.py`: it fills
  the `<operator>` element of the `contactinfo` payload, which is empty today. Update the usage line
  in the module docstring.

---

## Phase 2: Foundational

**Purpose**: existing WebSocket integration tests read messages one by one and would break when a
`station_status` follows every `snapshot`, `upsert` and `remove`.

- [X] T003 Make the `next_json` helpers skip messages whose `type` is `station_status`, with a
  one-line comment saying why (feature 017, contracts/websocket-protocol.md), in
  `tests/ws_protocol.rs` and `tests/band_filter.rs`. Add a separate helper
  `next_json_any` in `tests/ws_protocol.rs` that does not skip, for the US1 contract test.
  `cargo test` must pass unchanged.

**Checkpoint**: the suite tolerates the new message before it exists.

---

## Phase 3: User Story 1 - See the band and operator now on the air (Priority: P1) 🎯 MVP

**Goal**: the service sends `station_status` (band and operator of the most recent QSO by QSO time)
on connect, after every `upsert` and `remove`, and after a lag; the page shows `40m · IK0ABC` above
the score line (FR-001 to FR-006, FR-009 to FR-011).

**Independent Test**: quickstart §2 steps 1–6 and 8.

### Tests for User Story 1 ⚠️ write first, see them fail

- [X] T004 [P] [US1] Unit tests in `src/web/protocol.rs` for
  `ServerMessage::station_status(history: &History, fallback: Option<&str>, now: i64)`:
  - empty history → every field `null`
    (`{"type":"station_status","band":null,"operator":null,"qso_at":null,"age_s":null}`);
  - two QSOs, the later by QSO time inserted first → band and operator of the later one;
  - `qso_at` is the QSO's RFC 3339 UTC time (`…Z`) and `age_s = now − QSO time`;
  - a QSO with no `BAND` → `band` null, operator still set;
  - `OPERATOR` `" ik0abc "` → `"IK0ABC"` (trimmed, upper-case).
- [X] T005 [P] [US1] Unit tests in `src/web/session.rs` (use the existing `kind` helper):
  - `on_connect` → `["hello", "snapshot", "station_status"]` without a score and
    `["hello", "snapshot", "station_status", "score"]` with one (update the two existing
    expectations at the `["hello", "snapshot", "score"]` and `["hello", "snapshot"]` asserts);
  - `on_change(Upsert)` → `["upsert", "station_status"]`; `on_change(Remove)` →
    `["remove", "station_status"]`; `on_change(Score)` → `["score"]` only;
  - `on_lagged` → `["snapshot", "station_status", "score"]` (update the existing
    `["snapshot", "score"]` assert);
  - a session showing only 20m, history whose latest QSO is 40m → the status says `40m` (FR-009).
- [X] T006 [P] [US1] Contract test in `tests/ws_protocol.rs`,
  `a_page_gets_the_station_status_of_the_latest_qso`, using `next_json_any`:
  - a fresh page gets `hello`, `snapshot`, then `station_status` with every field `null`;
  - send QARTest QSO A (40m, operator `IK0ABC`, 15:04:04) then QSO B (20m, operator `IS0XYZ`,
    15:04:10): after B's `upsert` the next message is `station_status` with `"20m"` and `"IS0XYZ"`,
    within 2 s;
  - send a `contactreplace` of A changing its operator: the following `station_status` still says
    `20m`/`IS0XYZ`;
  - send a `contactreplace` of B changing its operator to `IS0QQQ`: the following `station_status`
    says `20m`/`IS0QQQ` (correction of the latest QSO);
  - a page opened afterwards gets `20m`/`IS0QQQ` right after its `snapshot` (FR-004, SC-002);
  - the message has exactly the keys `type`, `band`, `operator`, `qso_at`, `age_s`.
- [X] T007 [P] [US1] Node tests in `web/station.test.js` for `stationText(status)`:
  `null` → `'— · —'`; `{band:'40m', operator:'IK0ABC'}` → `'40m · IK0ABC'`; a null band or a
  null operator → `—` in its place.
- [X] T008 [P] [US1] Restart test in `tests/history_file.rs`: start with a history file whose latest
  QSO is 15m by `IK0ABC`; a page connecting before any new QSO gets `station_status` `15m`/`IK0ABC`
  (FR-005, SC-003). Reuse the file's `start_on` and `adif` helpers.

### Implementation for User Story 1

- [X] T009 [US1] In `src/web/protocol.rs`: add `ServerMessage::StationStatus { band, operator,
  qso_at, age_s }` (all `Option`, serialised as `station_status`) and the builder
  `ServerMessage::station_status(history, fallback, now)` from `History::latest(Some(1))`: band from
  `BAND`, operator from `OPERATOR` (fallback handled in US2, here `OPERATOR` only), `qso_at` via the
  existing `datetime_of`, `age_s` = `now.saturating_sub(qso_ts).max(0)` from the QSO's
  `QSO_DATE`/`TIME_ON` via `qso_timestamp`. Doc comment cites feature 017 and the contract. T004
  passes.
- [X] T010 [US1] Make `now_unix` in `src/pipeline.rs` `pub(crate)` so the session can stamp the
  status, keeping its doc comment.
- [X] T011 [US1] In `src/web/session.rs`:
  - `on_connect`, `on_change` and `on_lagged` take `station: &HelloStation` (and `on_change` the
    `&History`) and append the status as in T005; the fallback is
    `station.station.as_ref().and_then(|s| s.callsign.as_deref())`;
  - `drive` locks the history for `Event::Change` too and passes `station`;
  - update the doc comments. T005 and T006 pass.
- [X] T012 [P] [US1] In `src/web/assets/station.js` implement `stationText` (T007 passes).
- [X] T013 [US1] Page glue:
  - **`src/web/assets/index.html`**: `<div id="station" class="score station" aria-live="polite">— · —</div>`
    in `#corner-bl`, placed so it renders directly above `#score`; update the corner comment;
  - **`src/web/assets/app.js`**: `stationState` (null), reset on `hello`, `onStationStatus(message)`
    for `type === 'station_status'`, `renderStation()` writing `textContent` only on change and
    then calling `autozoom.change()`, as `renderScore` does;
  - **`src/web/assets/app.css`**: only what `.score` does not already cover, light and dark themes.
- [X] T014 [US1] Run quickstart §2 steps 1–6 and 8, and §4 (layout at 1024 px and 1920 px, globe and
  flat, both themes).

**Checkpoint**: US1 works and is testable on its own (MVP).

---

## Phase 4: User Story 2 - Know when the status is old or unknown (Priority: P2)

**Goal**: the operator falls back to the station callsign; the box greys out 15 minutes after the
latest QSO's time, with a `Last QSO HH:MMZ` tooltip (FR-006 to FR-008).

**Independent Test**: quickstart §2 step 7 and §3.

### Tests for User Story 2 ⚠️ write first, see them fail

- [X] T015 [P] [US2] Unit tests in `src/web/protocol.rs` for the operator fallback (FR-007, R4):
  empty `OPERATOR` → `STATION_CALLSIGN`; both empty → `fallback`; all three empty → `null`;
  whitespace-only counts as empty. And: a QSO time 60 s in the future → `age_s` 0.
- [X] T016 [P] [US2] Node tests in `web/station.test.js`:
  - `STATION_STALE_MS === 900_000`;
  - `lastQsoTitle('2026-10-08T15:04:29Z')` → `'Last QSO 15:04Z'`; `null` or unreadable → `''`;
  - `localQsoTime(age_s, now)` → `now − age_s × 1000`, and `null` when `age_s` is null;
  - staleness uses `isStale` from `score.js` with `STATION_STALE_MS`: 900 s → not stale,
    900.001 s → stale.

### Implementation for User Story 2

- [X] T017 [US2] Operator fallback chain in the builder in `src/web/protocol.rs` (T015 passes).
- [X] T018 [P] [US2] `STATION_STALE_MS`, `lastQsoTitle` and `localQsoTime` in
  `src/web/assets/station.js` (T016 passes).
- [X] T019 [US2] In `src/web/assets/app.js`: store `localQso` from `age_s`, set
  `#station.title` from `qso_at`, toggle `stale` on `#station` with
  `isStale(localQso, Date.now(), STATION_STALE_MS)`, and call `renderStation` from the existing
  `SCORE_TICK_MS` interval.
- [X] T020 [US2] Run quickstart §2 step 7 and §3.

**Checkpoint**: US1 and US2 both work.

---

## Phase 5: Polish & Cross-Cutting Concerns

- [X] T021 Browser step `station` in `tests/browser/run.mjs` (its entry in `STEPS`, and the step
  lists in `tests/browser/README.md` and `specs/012-page-fixes-browser-tests/contracts/browser-test.md`):

  - send a QSO timed 20 minutes ago: `#station` shows its band and operator and has `stale` at once
    (`age_s` > 900; on a fresh history, after a restart);
  - send two current QSOs with different bands and operators: `#station` reads the second and has
    no `stale`;
  - the bounding box of `#station` overlaps neither `#score` nor `#recent` (FR-011).

  Run `node tests/browser/run.mjs` locally.
- [X] T022 [P] Documentation:
  - **`docs/ham-radio.md`** §2.1: one sentence saying the current operator is that of the latest
    logged QSO, since OPON is not broadcast on its own;
  - **`README.md`**: the station status line (what it shows, the fallback, grey after 15 minutes);
  - **`specs/001-live-qso-map/contracts/websocket-protocol.md`**: a note pointing to
    `specs/017-station-status-box/contracts/websocket-protocol.md`.
- [X] T023 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md` (`docs/releasing.md`).
- [X] T024 Run the complete gates: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`.
- [X] T025 Per LL-007, run a manual rehearsal pipeline on the branch and tick this task with its
  pipeline number once the `station` browser step is green there. Record SC-001 to SC-005 in
  `checklists/verification.md`:
  - SC-001 from T006 and T021;
  - SC-002 from T006;
  - SC-003 from T008;
  - SC-004 from T016, T020 and T021;
  - SC-005 from T014.

---

## Dependencies & Execution Order

- **Setup (T001–T002)** comes first; T002 is only needed by the manual quickstart runs.
- **Foundational (T003)** blocks every Rust change in US1.
- **US1 (T004–T014)** is the MVP.
  - Rust: T009 → T010 → T011.
  - Page: T012 → T013, independent of the Rust chain.
- **US2 (T015–T020)** builds on US1's builder and page glue (T009, T013).
- **Polish (T021–T025)** comes last; T025 needs T021.

```text
T001, T002 → T003 → US1 (T004–T014) → US2 (T015–T020) → Polish (T021–T025)
                     Rust: T009 → T010 → T011
                     Page: T012 → T013
```

---

## Parallel Opportunities

- **US1 tests**: T004, T005, T006, T007 and T008 are in different files and can be written together.
- **US1 implementation**: the page chain (T012–T013) runs alongside the Rust chain (T009–T011).
- **US2**: T015 with T016; then T017 alongside T018.
- **Polish**: T022 with T023.

## Implementation Strategy

1. Setup and Foundational, then US1 alone: the box already shows band and operator for stations
   that use OPON. Stop and validate with quickstart §2.
2. US2 adds the fallback for single-op stations and the staleness look.
3. Polish, then the LL-007 rehearsal before merging.
