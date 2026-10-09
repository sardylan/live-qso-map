---
description: "Task list for 007-contest-score-layout"
---

# Tasks: Contest Score and Page Layout

**Input**: Design documents from `/specs/007-contest-score-layout/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation
it covers, run it, and confirm it fails first. Use `cargo test` for Rust and `node --test web/`
for the frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`).

**Conventions** (as in features 001–006):

- **Errors**: the crate `Error`/`Result` (`docs/rust-guidelines.md`). Score parsing errors are
  `Error::QarTest { element, message }`, logged with `tracing::warn!` and skipped.
- **Rules**:
  - domain rules: `docs/ham-radio.md` §1 (HR-1 UTC, HR-10 contest-agnostic);
  - interface rules: `docs/http-api.md` (API-2 typed JSON frames, API-4 `snake_case` and UTC
    with `Z`, additive messages keep protocol 3);
  - constitution Principle IV: the moved Last QSOs switch keeps the `lqm.recent` key.
- **Page code**: decisions live in pure modules with `web/*.test.js` tests; `app.js` is glue only.
  Score values reach the DOM only through `textContent` and `title`.
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

- [X] T001 Create the two new modules as stubs, so later tests compile:
  - **`src/ingest/score.rs`**: the doc comment
    `//! QARTest score messages (dynamicresults): score, QSO total and multiplier total (feature 007, research R1).`;
  - **`src/ingest/mod.rs`**: add `pub mod score;`;
  - **`src/web/assets/score.js`**: the header comment
    `// Pure score-line helpers (feature 007, research R5). No DOM.`

  `cargo build` must pass.

---

## Phase 2: User Story 1 - See the current contest score (Priority: P1) 🎯 MVP

**Goal**:
- QARTest's score message is read;
- the latest score is kept and sent to every page on connect, on update and after a lag;
- the page shows it as one line at the bottom left, above the box;
- the line greys out after 5 minutes without an update (FR-001 to FR-008, FR-012 for the score
  line, FR-013).

**Independent Test**: quickstart §2.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T002 [P] [US1] Write unit tests in `src/ingest/score.rs` for
  `parse(xml: &str) -> Result<ContestScore>` (data-model "ContestScore"):
  - **reference packets**: using `qartest::reference_packets` on the feature-001 capture,
    - packet 4 gives `{ score: 90, qsos: 6, mults: 10 }`;
    - packet 10 gives `{ score: 132, qsos: 7, mults: 11 }`;
  - **multiplier types (HR-10)**:
    - one `type="prefix"` total of 250 gives `mults: 250`;
    - three types (`country` 10, `zone` 5, `state` 7) give 22;
    - no `band="total"` multiplier gives 0;
  - **selection**:
    - per-band entries (`band="40"`) and totals with `mode="PH"` are ignored;
    - attributes match ignoring case (`band="TOTAL" mode="all"`);
  - **errors**, each `Error::QarTest` with the given `element`:
    - `<score>` missing, empty, `abc`, `-5` or `1.5` → `"score"`;
    - no total `<qso>`, or a non-numeric one → `"qso"`;
    - a non-numeric multiplier total → `"mult"`;
    - two multiplier totals of `18446744073709551615` (overflow) → `"mult"`;
    - ill-formed XML → an error, never a panic.
- [X] T003 [P] [US1] Update the tests in `src/ingest/qartest.rs`:
  - replace `score_and_delete_messages_give_no_event` with
    `score_messages_give_a_score_and_deletes_nothing`:
    - packet 4 (index 3) gives `Some(QarTestItem::Score(ContestScore { score: 90, qsos: 6, mults: 10 }))`;
    - packet 10 (index 9) gives the 132/7/11 score;
    - the `contactdelete` sample still gives `None`;
  - adapt `message(i)` to unwrap `QarTestItem::Qso`;
  - `mutated_packets_never_panic` keeps running over every packet.
- [X] T004 [P] [US1] Write unit tests in `src/ingest/mod.rs`:
  - `events_from_datagram` on packet 4 returns `Ingested { events: [], score: Some(90/6/10) }`;
  - on packet 1 it returns one event and `score: None`;
  - on an ADIF datagram it returns `score: None`;
  - a `dynamicresults` with `<score>abc</score>` returns no events and no score. It logs one WARN
    event naming `score`, checked with a `tracing` layer that counts warnings, as
    `tests/qartest_replay.rs` does.
- [X] T005 [P] [US1] Write unit tests in `src/pipeline.rs`:
  - `latest_score()` is `None` on a new pipeline;
  - after `publish_score(s)`:
    - `latest_score()` holds `s` with a `received_at` that matches `^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$`;
    - a subscriber receives `HistoryChange::Score` with the same values;
  - publishing an identical score twice broadcasts twice and refreshes `received` (R2: no
    deduplication);
  - `history_len()` is unchanged by scores (FR-004).
- [X] T006 [P] [US1] Write unit tests in `src/web/protocol.rs`:
  - `ServerMessage::score(&ScoreState)`, built from a state whose `received` is 42 s in the past
    (`Instant::now().checked_sub(...)`), serialises exactly as
    `{"type":"score","score":132,"qsos":7,"mults":11,"received_at":"2026-10-08T15:04:29Z","age_s":42}`;
  - a `received` in the future, or an `Instant` that can't be subtracted, gives `age_s: 0`.
- [X] T007 [P] [US1] Write unit tests in `src/web/session.rs`:
  - `on_connect` with no score gives `hello`, `snapshot` only;
  - `on_connect` with a score gives `hello`, `snapshot`, `score`, in that order;
  - `on_change(HistoryChange::Score(..))` gives one `score`;
  - `on_lagged` with a score gives `snapshot`, then `score`; without one, only `snapshot`;
  - in the driver test with fakes, a score published after connect reaches the sink.
- [X] T008 [P] [US1] Write integration tests in `tests/ws_protocol.rs`:
  - **live**: a connected page, then a UDP send of reference packet 4. Within 2 s the page gets
    `{"type":"score","score":90,"qsos":6,"mults":10,…}` with `age_s` 0 or 1 (SC-001);
  - **on connect**: a page that connects after the score gets `hello`, `snapshot` and that
    `score` within 1 s (SC-002);
  - **latest wins** (spec Edge Cases): after packet 4, send packet 10 with `<call>IS0MZK</call>`
    replaced by `<call>IQ0ZZZ</call>`. The page gets a second `score` with 132/7/11, and a page that
    connects afterwards gets only 132/7/11;
  - **none held**: a page on a fresh service gets no `score` message within 500 ms;
  - **malformed** (SC-006): 100 datagrams of packet 4 with `<score>90</score>` replaced by
    `<score>x</score>`, then one QSO. The page gets the QSO's `upsert` and no `score`, and
    `latest_score()` stays `None`;
  - **band filter** (R2): with a band filter that excludes 40m, packet 4 still gives a `score`.
- [X] T009 [P] [US1] Update `tests/qartest_replay.rs` `reference_capture_gives_sc_009`: after the
  replay, `service.pipeline().latest_score()` holds 132/7/11. The QSO assertions and the
  no-warnings check stay as they are.
- [X] T010 [P] [US1] Write `web/score.test.js` for `src/web/assets/score.js` (data-model
  "score.js"):
  - **`groupDigits`**: `0` → `"0"`, `999` → `"999"`, `1000` → `"1.000"`,
    `1234567` → `"1.234.567"`, `12345678` → `"12.345.678"`; `-1`, `1.5`, `NaN` and `"12"` →
    `"?"`;
  - **`scoreText`**:
    - `null` → `"Score —"`;
    - `{score: 1234567, qsos: 1523, mults: 183}` → `"1.234.567 pts · 1.523 QSO · 183 mults"`;
    - zeros → `"0 pts · 0 QSO · 0 mults"`;
  - **`isStale(localReceived, now)`**: false at 0 s and at exactly 300 s, true at 300 001 ms; the
    custom `limitMs` argument is honoured;
  - **`updatedTitle`**: `"2026-10-08T15:04:29Z"` → `"Last updated 15:04Z"`; `""`, `null` and
    `"yesterday"` → `""`.

### Implementation for User Story 1

- [X] T011 [US1] Implement `ContestScore` and `parse` in `src/ingest/score.rs`, as research R1
  describes:
  - walk with `quick_xml::Reader`, tracking the element path;
  - read `band`, `mode` and `type` attributes of `<qso>` and `<mult>` inside `<breakdown>`;
  - use `u64` with `checked_add`;
  - decode entities in text as `xml.rs` does.

  T002 must pass.
- [X] T012 [US1] In `src/ingest/qartest.rs`:
  - add `enum QarTestItem { Qso(QarTestMessage), Score(ContestScore) }`;
  - make `parse` return `Result<Option<QarTestItem>>`, with `dynamicresults` going to
    `score::parse` on the same text;
  - update the module doc (FR-005a now covers deletes only).

  T003 must pass.
- [X] T013 [US1] In `src/ingest/mod.rs`:
  - add `pub struct Ingested { pub events: Vec<QsoEvent>, pub score: Option<ContestScore> }` and
    return it from `events_from_datagram`;
  - in `run`, publish `score` with `pipeline.publish_score` before the band split, with no gate;
  - update every caller, including `qartest.rs`'s `mutated_packets_never_panic` and the
    `tests/` files.

  T004 must pass.
- [X] T014 [US1] In `src/pipeline.rs`:
  - add `ScoreState { score, received: Instant, received_at: String }`, the `score` field,
    `publish_score`, `latest_score`, and `HistoryChange::Score(Arc<ScoreState>)`;
  - `received_at` uses `SystemTime::now().duration_since(UNIX_EPOCH)` seconds and
    `history::format_rfc3339`. If the clock is before 1970, use 0 and log a warning, with no
    `unwrap` (Principle II);
  - `RunningService` gets `latest_score()` for tests;
  - recover a poisoned lock as `lock_history` does.

  T005 must pass.
- [X] T015 [US1] In `src/web/protocol.rs`, add `ServerMessage::Score { score, qsos, mults,
  received_at, age_s }` and `ServerMessage::score(&ScoreState)`, which computes `age_s` from
  `received.elapsed()`. T006 must pass.
- [X] T016 [US1] In `src/web/session.rs`:
  - `on_connect` and `on_lagged` take `score: Option<&ScoreState>` and append a `score` message
    when it is `Some`;
  - `on_change` maps `HistoryChange::Score`;
  - `drive` reads `pipeline.latest_score()` after subscribing.

  T007, T008 and T009 must pass.
- [X] T017 [US1] Implement `groupDigits`, `scoreText`, `isStale` and `updatedTitle` in
  `src/web/assets/score.js`. Serve it from `src/web/mod.rs` as `/score.js` (`text/javascript`),
  and add `("/score.js", "text/javascript")` to `static_assets_are_served` in
  `tests/ws_protocol.rs`. T010 must pass.
- [X] T018 [US1] Page markup and styles:
  - **`src/web/assets/index.html`**: wrap the box in `<div id="corner-bl" class="corner-bl">`,
    with `<div id="score" class="score" aria-live="polite">Score —</div>` before
    `<div id="recent" …>`;
  - **`src/web/assets/app.css`**:
    - `.corner-bl` takes `.recent`'s old position (`position: absolute; left: 10px;
      bottom: 30px; z-index: 2; max-width: 45%`), with `display: flex; flex-direction: column;
      align-items: flex-start; gap: 6px`;
    - `body.scale-on .corner-bl { bottom: 56px; }` replaces the `.recent` rule;
    - `.recent` keeps its panel look but loses `position`, `left`, `bottom`, `z-index` and
      `max-width`;
    - `.score` has the same panel look, `white-space: nowrap` and `font-weight: 600`;
    - `.score.stale` sets `color: #999; font-weight: normal`.
- [X] T019 [US1] Glue in `src/web/assets/app.js`:
  - import from `./score.js`;
  - keep `scoreState`: reset it to `null` in `onHello`; on a `score` message, set it with
    `localReceived = Date.now() - age_s * 1000`;
  - `renderScore()`:
    - computes `text = scoreText(...)`;
    - writes `textContent` only when `text` differs from what is shown, so the `aria-live` region
      announces real changes only;
    - toggles `stale` with `isStale` and sets `title = updatedTitle(received_at)` on every call;
  - render on each `score` message and from a `setInterval` every 5 s;
  - add `'#score'` to `PANELS`, and call `autozoom.change()` whenever `renderScore()` writes new
    text, because the line's width may have changed.
- [ ] T020 [US1] Manually run quickstart §2 (steps 1–7, including the 5-minute stale check and a
  service restart). Record the results in `checklists/verification.md`.

**Checkpoint**: the score line works on the current layout; the Last QSOs button is still among
the map controls.

---

## Phase 3: User Story 2 - A tidy, predictable page layout (Priority: P2)

**Goal**:
- the top left holds only zoom, Globe/Flat and Auto-zoom;
- the top-right panel holds every viewer setting, including the Last QSOs checkbox, which keeps
  the `lqm.recent` key;
- the legend stays at the bottom right (FR-009 to FR-012, FR-014, FR-015).

**Independent Test**: quickstart §3.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T021 [P] [US2] Extend `static_assets_are_served` in `tests/ws_protocol.rs`, as a regression
  guard on the markup:
  - `/` contains `id="settings"`, `id="recent-shown"`, `id="corner-bl"` and `id="score"`;
  - `/` no longer contains `class="limit-control"`;
  - `/app.js` no longer contains `class RecentControl`.
- [X] T022 [P] [US2] In `web/view.test.js`, add a case: a value saved with `saveRecent(storage,
  false)` before this feature reads back as `false` with `loadRecent`, so the moved checkbox
  starts unticked (Principle IV). Keep the existing fallback cases. This is a regression guard and
  may already pass, because the helpers don't change: it pins the key. The failing-first test for
  moving the switch is T021.

### Implementation for User Story 2

- [X] T023 [US2] In `src/web/assets/index.html`, turn `.limit-control` into
  `<div id="settings" class="settings">`, with:
  1. "Map: last [N] QSOs";
  2. the separated box section:
     - `<label><input type="checkbox" id="recent-shown"> Last QSOs</label>`;
     - "Box: last [N]";
     - "Newest: [bottom, top]".

  In `src/web/assets/app.css`, rename `.limit-control` to `.settings` (and
  `.limit-control .box-settings` to `.settings .box-settings`), and lay the box section out on
  one row.
- [X] T024 [US2] Glue in `src/web/assets/app.js`:
  - remove `class RecentControl` and its `addControl`, so the top left keeps the Navigation,
    Projection and Auto-zoom controls, in that order (FR-009);
  - wire `#recent-shown`:
    - initial `checked = recentShown`;
    - on `change`: set `recentShown`, `saveRecent(localStore(), recentShown)`, `renderRecent()`,
      `autozoom.change()`;
  - in `PANELS`, replace `'.limit-control'` with `'#settings'`;
  - remove any `.toggle-control` CSS left unused (the Auto-zoom button still uses it: keep what
    it needs).

  T021 and T022 must pass.
- [ ] T025 [US2] Manually run quickstart §3 at 1024 px and 1920 px, on the globe and flat, with
  the box shown and hidden, including a browser restart and a viewer who had the box hidden before
  the upgrade (SC-004, SC-005). Record the results in `checklists/verification.md`.

---

## Phase 4: Polish & Cross-Cutting Concerns

- [X] T026 [P] Documentation:
  - **`README.md`**: the score line (what it shows, `Score —`, grey after 5 minutes), and the new
    layout (which corner holds what; the Last QSOs checkbox is in the settings panel);
  - **`specs/001-live-qso-map/spec.md`**: a note at FR-005a saying score messages are read since
    feature 007;
  - **`specs/004-qth-autozoom/spec.md`**: a note at FR-021 saying the box switch moved to the
    settings panel in feature 007;
  - **`specs/001-live-qso-map/contracts/websocket-protocol.md`**: a note pointing to
    `specs/007-contest-score-layout/contracts/websocket-protocol.md`.
- [X] T027 Run the complete gates: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` and `node --test web/`. Complete
  `checklists/verification.md` with the automated results and SC-001 to SC-006:
  - SC-001 from T002, T008 and T020;
  - SC-002 from T008;
  - SC-003 from T010 and T020;
  - SC-004 and SC-005 from T025;
  - SC-006 from T008.

---

## Dependencies & Execution Order

- **Setup (T001)** comes first.
- **US1 (T002–T020)** depends on Setup. This is the MVP.
  - The implementation follows the data flow: T011 → T012 → T013 → T014 → T015 → T016.
  - T013 changes the signature that `tests/` use, so T008 and T009 are written against the new
    `Ingested` shape.
  - T017 is independent of the Rust chain. T018 and T019 need T017.
- **US2 (T021–T025)** depends on US1 for `#corner-bl` and the `PANELS` changes in `app.js`.
  T021 and T022 may be written in parallel with US1, because they touch test files only.
- **Polish (T026–T027)** comes last.

```text
T001 → US1 (T002–T020) → US2 (T021–T025) → Polish (T026–T027)
         Rust: T011 → T012 → T013 → T014 → T015 → T016
         Page: T017 → T018 → T019
```

---

## Parallel Opportunities

- **US1 tests**: T002–T010 are in different files and can all be written in parallel (T008 and
  T021 share `tests/ws_protocol.rs`: write them one after the other).
- **US1 implementation**: the page chain (T017–T019) runs alongside the Rust chain (T011–T016).
- **US2**: T021 and T022 together; then T023 → T024.
- **Polish**: T026 runs alongside the manual checks.

### Parallel example: User Story 1

```text
Together:  T002 score.rs · T003 qartest.rs · T004 ingest · T005 pipeline · T006 protocol
           · T007 session · T008 ws_protocol · T009 qartest_replay · T010 score.test.js
Then:      Rust T011 → … → T016   alongside   page T017 → T018 → T019
Then:      T020 manual check
```

---

## Implementation Strategy

1. **MVP** (Setup and US1): the score line, live on every page, with the empty and stale states.
   It already sits at the bottom left above the box.
2. **US2**: the corners rearranged; the Last QSOs switch moves into the settings panel with its
   saved state.
3. **Polish**: documentation, gates and the verification note.
