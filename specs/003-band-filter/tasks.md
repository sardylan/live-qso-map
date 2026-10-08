---
description: "Task list for 003-band-filter"
---

# Tasks: Band Filter

**Input**: Design documents from `/specs/003-band-filter/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Every test task comes before the implementation it
covers, and must be run and seen to fail first. Use `cargo test` for Rust and `node --test web/` for
the frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`).

**Conventions** (as in feature 001):

- **Errors**: the crate `Error`/`Result` from `src/errors.rs` (`docs/rust-guidelines.md` §1).
- **Bands**: always in frequency order (`docs/ham-radio.md` HR-2). Use `band::band_rank` to sort
  bands; never sort band names as strings.
- **Tests**: unit tests go in `#[cfg(test)] mod tests` in the same file. Every integration test file
  starts with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` and uses
  `tests/common` (`test_config()`, `send_udp`, `next_upsert`).
- **Commits**: commit after each phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US3 from spec.md

---

## Phase 1: Setup

- [ ] T001 Add `pub mod band_filter;` to `src/lib.rs` and create an empty `src/band_filter.rs` with a module doc comment (`//! Band selection: groups, exclusions and the effective band set (feature 003, research R1).`). `cargo build` must pass.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the selection model (FR-002 – FR-005) and the `bands` setting (FR-001), which every
story uses.

### Tests first (write, run, confirm they FAIL)

- [ ] T002 [P] Write unit tests in `src/band_filter.rs`:
  - **group table (SC-005)**: `GROUPS` contains exactly `lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`,
    `uhf`, `shf`, `ehf`, with the members from data-model.md; every member is an ADIF band
    (`band::is_adif_band`); every group's members are strictly increasing by `band::band_rank`.
  - **User Story 2 examples (SC-003)**: `resolve` of `["hf-contest"]` gives `160m 80m 40m 20m 15m
    10m`; `["hf","2m","-60m"]` gives `160m 80m 40m 30m 20m 17m 15m 12m 10m 2m`; `["warc","vhf"]`
    gives `30m 17m 12m 8m 6m 5m 4m 2m 1.25m`; `["-60m"]` gives every ADIF band except 60m, in
    frequency order; `[" HF-Contest ", "40M"]` equals `["hf-contest"]`; `["20m","-20m","40m"]`
    gives `40m` (exclusions win).
  - **errors**: `["hf-dx"]` and `["21m"]` give `Err(Error::Config { setting: "bands", .. })` whose
    message contains the bad entry; `["warc","-warc"]` gives an error saying no bands are left;
    `["-"]` is an error.
  - **empty input**: `parse_setting(&[])` and `parse_setting(&["", " "])` give `Ok(None)` (no
    filter).
  - **behaviour**: `BandSet::contains` ignores case; `Display` is space-separated in frequency
    order; `BandSet::bands()` returns them in frequency order.
- [ ] T003 [P] Extend the unit tests in `src/config.rs` (contracts/configuration.md, delta):
  - the default `band_filter` is `None`;
  - TOML `bands = ["hf-contest"]` gives the 6 contest bands;
  - `LQM_BANDS="hf,-60m"` overrides TOML;
  - `--bands warc --bands 2m` (repeatable) and `--bands warc,2m` both override env, replacing the
    whole list (no merge);
  - `--bands hf-dx` gives `Error::Config { setting: "bands", .. }`;
  - `LQM_BANDS=""` means no filter.

### Implementation

- [ ] T004 Implement `src/band_filter.rs` per data-model.md:
  - `pub const GROUPS: &[(&str, &[&str])]`;
  - `enum Target { Band(&'static str), Group(&'static str) }` and `struct SelectionEntry { exclude,
    target }`;
  - `pub struct BandSet { bands: Vec<&'static str> }` with `contains`, `bands()` and `Display`;
  - `pub fn resolve(entries: &[&str]) -> Result<BandSet>`;
  - `pub fn parse_setting(entries: &[String]) -> Result<Option<BandSet>>`, which splits each entry
    on commas, skips blanks, and returns `None` when nothing remains.

  Every band name comes from `band::canonical_band` (canonical `&'static str`), and the result is
  sorted by `band_rank`. T002 must pass.
- [ ] T005 Add the `bands` setting to `src/config.rs`:
  - `Cli.bands: Vec<String>` (`#[arg(long = "bands")]`, repeatable), with a help text naming
    `LQM_BANDS` and the groups;
  - `FileConfig.bands: Option<Vec<String>>`;
  - layering: CLI if non-empty, else `LQM_BANDS` if set, else the file;
  - `Config.band_filter: Option<BandSet>` (default `None`), filled with `band_filter::parse_setting`.

  T003 must pass.
- [ ] T006 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` over the crate (`Cargo.toml`, `src/`). All must pass.

**Checkpoint**: the selection model is ready.

---

## Phase 3: User Story 1 - Only the considered bands reach the map (Priority: P1) 🎯 MVP

**Goal**: QSOs on ignored or unknown bands are dropped at ingest and never reach lookups, history or
pages. The legend lists only considered bands, in frequency order.

**Independent Test**: `tests/band_filter.rs::only_considered_bands_reach_the_map`. With
`hf-contest`, a 20m QSO goes through and a 2m QSO does not. The counting source sees 1 call, and
`hello.bands` is `160m 80m 40m 20m 15m 10m`.

### Tests for User Story 1 (write first, confirm they FAIL)

- [ ] T007 [P] [US1] Add unit tests to the `tests` module in `src/ingest/mod.rs` for `band_allowed(record: &AdifRecord, filter: Option<&BandSet>) -> bool`:
  - `None` allows everything, including a record with no band;
  - with `hf-contest`: `BAND=20m` is allowed; `BAND=2m` and `BAND=60m` are not; a record with no
    band or an out-of-band frequency is not (unknown band while a filter is active);
  - `FREQ=14.025` with no `BAND` is allowed once `normalise` has derived the band (HR-3);
  - `events_from_datagram` with the filter returns no event for a 2m ADIF record.
- [ ] T008 [P] [US1] Extend the unit tests in `src/web/protocol.rs`: `ServerMessage::hello(&map, Some(&hf_contest))`, with a palette holding 160m, 60m, 20m and 2m, sends `bands` = `[160m, 20m]` in that order; `hello(&map, None)` keeps all four, in frequency order. Both calls MUST still serialise `"protocol": 2` and the same `basemap` as `hello(&map)` did before (feature 002); only `bands` is filtered.
- [ ] T009 [P] [US1] Write integration test `tests/band_filter.rs::only_considered_bands_reach_the_map`:
  - start the service with `test_config()`, `band_filter = hf-contest` and a counting lookup source
    (as in `tests/resend_gate.rs`);
  - connect a WebSocket client (as in `tests/ws_protocol.rs`) and assert `hello.bands` lists
    exactly 160m, 80m, 40m, 20m, 15m and 10m, in order;
  - send a callsign-only ADIF QSO on `20m` and one on `2m`, plus one with no band or frequency;
  - assert exactly one upsert (the 20m QSO) on both the broadcast and the WebSocket, 1 lookup call,
    and `history_len() == 1`.

### Implementation for User Story 1

- [ ] T010 [US1] In `src/ingest/mod.rs`:
  - add `pub fn band_allowed(record, filter)` (unknown band → `false` when a filter is set);
  - thread `filter: Option<BandSet>` through `events_from_datagram` and `run`, applied after
    `normalise` and before the gate;
  - drop rejected events with `tracing::debug!(call, band, "dropping QSO on an ignored band")`.

  Pass `config.band_filter.clone()` from `pipeline::spawn_tasks` in `src/pipeline.rs`. T007 must
  pass.
- [ ] T011 [US1] In `src/web/protocol.rs`, change `ServerMessage::hello(map)` to `hello(map, filter: Option<&BandSet>)`, keeping only palette bands that `filter` contains (all when `None`), still sorted by `band_rank`. Pass the filter through `web::serve` (`src/web/mod.rs`) and `Session::on_connect` (`src/web/session.rs`), and update the existing callers and tests. T008 must pass. Keep the `protocol` (2) and `basemap` fields from feature 002 unchanged; only `bands` is filtered.
- [ ] T012 [US1] Run `cargo test --test band_filter` and confirm T009 passes. Then run quickstart §2 with the real binary.

**Checkpoint**: User Story 1 works on its own.

---

## Phase 4: User Story 2 - Combine groups, single bands and exclusions (Priority: P2)

**Goal**: any mix of groups, bands and exclusions resolves as documented. Invalid selections stop
startup with a clear message, and the effective set is logged.

**Independent Test**: the unit tests from T002, T003 and T013, plus quickstart §3 with the real
binary.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T013 [P] [US2] Add a unit test in `src/pipeline.rs` for `pub fn bands_log_line(filter: Option<&BandSet>) -> String`: `None` gives `"bands considered: all"`; `hf,2m,-60m` gives `"bands considered: 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m"`.

### Implementation for User Story 2

- [ ] T014 [US2] Implement `bands_log_line` in `src/pipeline.rs` and log it at info level in `pipeline::start`, before `"live-qso-map started"` (FR-006). T013 must pass.
- [ ] T015 [US2] Make `src/main.rs` print configuration errors as `invalid setting bands: …` and exit non-zero (already true for `Error::Config`; verify by hand). Run quickstart §3 with the real binary: one valid selection, `hf-dx` and `warc,-warc`.

**Checkpoint**: User Stories 1 and 2 work on their own.

---

## Phase 5: User Story 3 - A correction onto an ignored band removes the QSO (Priority: P3)

**Goal**: a correction that moves a shown QSO onto an ignored band removes it from the history and
every page. A stale in-flight revision cannot bring it back (TR-002), and a later correction back
onto a considered band shows it again.

**Independent Test**: `tests/band_filter.rs::correction_onto_ignored_band_removes_the_qso`.

### Tests for User Story 3 (write first, confirm they FAIL)

- [ ] T016 [P] [US3] Extend the unit tests in `src/history.rs`:
  - `remove(id, rev)` of a present QSO with a higher rev returns `Removed` and `len` drops;
  - an upsert with `rev <= tombstone` is `Stale`;
  - an upsert with a higher rev is `Accepted` as a new entry, with a new `sort_ts`, and clears the
    tombstone;
  - `remove` of an unknown id returns `NotPresent` but records the tombstone;
  - tombstones are capped at `capacity` (oldest evicted), and unbounded when the capacity is 0.
- [ ] T017 [P] [US3] Extend the unit tests in `src/ingest/gate.rs`: `contains(&event)` is true for a known identity and for a correction whose previous identity (`oldcall`/`oldtimestamp`, no logger id) is known; false otherwise.
- [ ] T018 [P] [US3] Extend the unit tests in `src/pipeline.rs`: `Pipeline::remove(id, rev)` broadcasts `HistoryChange::Remove { id }` only when the history returned `Removed`; a later `publish` of a stale rev sends nothing.
- [ ] T019 [P] [US3] Extend the unit tests in `src/web/session.rs` and `src/web/protocol.rs`: `on_change(Remove { id })` emits `ServerMessage::Remove { id }`, which serialises as `{"type":"remove","id":"…"}`.
- [ ] T020 [P] [US3] Extend `web/map-state.test.js`:
  - `applyRemove` drops the QSO, and its marker when no QSOs are left (`diffLayers` lists it in
    `remove`);
  - `applyRemove` of an unknown id changes nothing;
  - after `applyRemove`, an upsert with the same id and any rev is added as new.

  Also extend `web/view.test.js` for `popupToClose(openKey, diff)` (FR-012):
  - `false` when no popup is open (`openKey` null);
  - `true` when `openKey` is in `diff.remove`;
  - `false` when `openKey` is only in `diff.update`, because the glue refreshes the content
    instead;
  - `false` when `openKey` is not in the diff.
- [ ] T021 [P] [US3] Write integration test `tests/band_filter.rs::correction_onto_ignored_band_removes_the_qso`:
  - filter `hf-contest`, with the country-data fixture as the only source;
  - send a QARTest `contactinfo` (adapt packet #3, K6QA, `<txfreq>1402500</txfreq>`, 20m) and expect
    an upsert on the broadcast and on a WebSocket client;
  - send a `contactreplace` with the same `<id>` and `<txfreq>535000</txfreq>` (60m): expect
    `HistoryChange::Remove`, `{"type":"remove","id":<same id>}` on the WebSocket within 2 s
    (SC-004), and `history_len() == 0`;
  - send another `contactreplace` back on 20m: expect an upsert again;
  - send a `contactreplace` on 60m for an unknown `<id>`: expect no message at all.

### Implementation for User Story 3

- [ ] T022 [US3] Implement `History::remove(id, rev) -> Removal { Removed, NotPresent }`, the tombstone map and order (capped like the history), and the tombstone check in `upsert`, in `src/history.rs`. T016 must pass.
- [ ] T023 [P] [US3] Implement `ResendGate::contains(&QsoEvent) -> bool` in `src/ingest/gate.rs`, reusing `previous_identity`. T017 must pass.
- [ ] T024 [US3] Add `HistoryChange::Remove { id: String }` and `Pipeline::remove(&self, id: &str, rev: u32) -> bool` (broadcast under the lock only when `Removed`, R3) in `src/pipeline.rs`. Make every `match` on `HistoryChange` exhaustive. T018 must pass.
- [ ] T025 [US3] Implement `ServerMessage::Remove { id }` in `src/web/protocol.rs` and the `Remove` arm of `Session::on_change` in `src/web/session.rs`. T019 must pass.
- [ ] T026 [US3] Implement the removal path in `src/ingest/mod.rs`:
  - when `band_allowed` is false and `gate.contains(&event)`, run `gate.check(&mut event)` to get
    the next rev and the history id, then call `pipeline.remove(&event.id, event.rev)`;
  - otherwise drop silently (debug log).

  `ingest::run` needs a `Pipeline` handle; pass it from `spawn_tasks`. T021 must pass.
- [ ] T027 [P] [US3] Implement `applyRemove(state, id)` in `src/web/assets/map-state.js` and `popupToClose(openKey, diff)` in `src/web/assets/view.js`. T020 must pass.
- [ ] T028 [US3] Handle `{"type":"remove"}` in `src/web/assets/app.js`. Glue only.
  - Call `applyRemove(state, message.id)`, then `render()`. In the MapLibre page, `render()` already
    computes `diffLayers(current, next)`, and `shouldRender` lets `setData` update both the
    `qso-markers` and `qso-lines` sources, so the marker and line disappear.
  - Track the open popup and its marker key. In `render()`, close it when `popupToClose(openKey,
    diff)` is true, and refresh its content (`setDOMContent`) when the key is in `diff.update`
    (FR-012: a removed QSO must not stay visible in an open popup).
  - Check by hand: open a popup, send the correction onto 60m, and the popup closes.

**Checkpoint**: all user stories work on their own.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T029 [P] Add `tests/band_filter.rs::mixed_stream_respects_the_filter` (SC-001, SC-002):
  - send 1,000 ADIF QSOs cycling over every ADIF band, plus some with no band, 50 per datagram (the
    macOS 9216-byte limit), with filter `hf`;
  - assert the history holds exactly the QSOs on `hf` bands;
  - assert a counting source was called exactly that many times;
  - assert every stored QSO's band is in `hf`.
- [ ] T030 [P] Document `bands`:
  - add a commented `bands = ["hf-contest", "-60m"]` example with the group list to
    `deploy/config.example.toml`;
  - add a configuration row and a short "Band selection" section to `README.md`;
  - add a row pointing to `specs/003-band-filter/contracts/configuration.md` in
    `specs/001-live-qso-map/contracts/configuration.md`.

  Every band list must be in frequency order (HR-2).
- [ ] T031 [P] Add a note on the `remove` message, pointing to `specs/003-band-filter/contracts/websocket-protocol.md`, to both `specs/002-maplibre/contracts/websocket-protocol.md` (the current protocol 2 delta) and `specs/001-live-qso-map/contracts/websocket-protocol.md`.
- [ ] T032 Run the complete quality gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`. Then run quickstart §1–§4 and record SC-001 to SC-005 in `specs/003-band-filter/checklists/verification.md`.

---

## Dependencies & Execution Order

- **Setup (T001)** comes first, then **Foundational (T002–T006)**, which blocks every story.
- **US1 (T007–T012)** depends on Foundational. This is the MVP.
- **US2 (T013–T015)** depends on Foundational. It is independent of US1, but T014 touches
  `src/pipeline.rs`, which T010 also edits, so run it after T010.
- **US3 (T016–T028)** depends on US1: it uses the ingest filter path (T010) and the hello filter
  (T011).
- **Polish (T029–T032)** comes after US1 and US3. T029 needs US1.

```text
T001 → T002–T006 → US1 (MVP) ─┬→ US3 → Polish
                    └→ US2 ───┘
```

### Within each story

Tests come first and must fail. Then the pure modules (`band_filter`, history, gate, protocol,
`map-state.js`), then the wiring (ingest, pipeline, web), then the glue (`app.js`).

---

## Parallel Opportunities

- **Foundational**: T002 and T003 (different files).
- **US1**: T007, T008 and T009.
- **US3**: T016–T021 are all different files, so all six can run in parallel. T023 and T027 also run
  in parallel with T022.
- **Polish**: T029, T030 and T031.

### Parallel example: User Story 3

```text
Together:  T016 history · T017 gate · T018 pipeline · T019 session/protocol · T020 map-state.js · T021 integration
Then:      T022 history ∥ T023 gate ∥ T027 map-state.js
Then:      T024 pipeline → T025 protocol/session → T026 ingest → T028 app.js
```

---

## Implementation Strategy

1. **MVP**: Setup, Foundational and US1. Configure `bands`; ignored QSOs vanish and cost no lookups;
   the legend is trimmed.
2. **US2** adds the startup log and validated selections, so the operator can confirm what is
   active.
3. **US3** adds removal on corrections, so the map stays truthful when the operator fixes a band.
4. **Polish** adds the 1,000-QSO mixed-stream check (SC-001/SC-002), documentation, the gates and
   the verification note.
