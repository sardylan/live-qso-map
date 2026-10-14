---
description: "Task list for 014-adif-persistence"
---

# Tasks: QSOs Kept in a Local ADIF File

**Input**: Design documents from `/specs/014-adif-persistence/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R15), data-model.md,
contracts/history-file.md, contracts/command-line.md, contracts/websocket-protocol.md, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the code it covers,
run it, and confirm it fails first. Use `cargo test`; page modules use `node --test 'web/*.test.js'`.

**Conventions:**

- **Unit tests** go in `#[cfg(test)] mod tests` of the same file.
- **Integration tests** go in the new `tests/history_file.rs` and `tests/enrich_only.rs`. They:
  - return `Result<(), Box<dyn std::error::Error>>` and use `?`, never `unwrap`/`expect`/`panic!`;
  - have no inline lint suppressions (LL-001);
  - start the service with `live_qso_map::pipeline::start_with_country(config, Some(sources),
    Some(country))` and use the helpers in `tests/common/mod.rs` (`test_config`, `send_udp`,
    `next_upsert`, and the new ones from T002);
  - each use their own temporary folder (T002), never a shared path.
- **Fake providers** are `LookupSource` closures, as in `tests/lookup_resolution.rs`: they answer
  per callsign, count their calls, and can be told to fail (`Err(Error::Provider)`) or to hang
  until released (for the timeout and "live first" cases).
- **The test seam for the file**: `Config::history_file` is `Option<PathBuf>`. `Config::load`
  always fills it (R1); `Config::default()` leaves it `None`, which means "no history file" and is
  used only by tests that don't care about persistence. `test_config()` keeps `None`.
- **Field names**: `APP_LIVEQSOMAP_ADDED`, `APP_LIVEQSOMAP_LOOKUP_FAILED` are `pub const`s in
  `src/history_file/mod.rs`; tests use the constants.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US3)

---

## Phase 1: Setup

- [X] T001 [P] Create the fixtures in `tests/fixtures/` and list them in `tests/fixtures/README.md`:
  - `history-external.adi`: an exported log with a header and 8 records — 2 with `GRIDSQUARE`,
    1 with `LAT`/`LON`, 3 with no position, 1 on `2m` (outside `hf-contest`), 1 malformed (no
    `TIME_ON`);
  - `history-saved.adi`: the same QSOs as written by the service (header per
    contracts/history-file.md, `APP_LIVEQSOMAP_ADDED`, `APP_LIVEQSOMAP_POS_SOURCE`/`PRECISION`),
    including one record with `APP_LIVEQSOMAP_POS_REJECTED`, one with
    `APP_LIVEQSOMAP_LOOKUP_FAILED`, and one QARTest record with `APP_LIVEQSOMAP_LOGGER_ID`;
  - `history-not-adif.txt`: plain text with neither `<EOH>` nor `<EOR>`.
- [X] T002 [P] Extend `tests/common/mod.rs`:
  - `temp_dir(name) -> PathBuf`: a fresh folder under `std::env::temp_dir()` named from `name`,
    the process id and a counter; created empty;
  - `FakeProvider` builder: per-call answers (`Found(LookupCandidate)`, `NotFound`, `Fail`,
    `Hang(Arc<Notify>)`), a shared call counter, and `.source() -> LookupSource`;
  - `read_records(path) -> Vec<AdifRecord>` using `live_qso_map::adif::parse::parse_datagram`.

---

## Phase 2: Foundational (blocking prerequisites)

**Error variant, unbounded history, configuration, the added-field list, and the file module's
read/write/restore primitives. Every story needs them.**

- [X] T003 [P] Write a failing unit test in `src/errors.rs`: `Error::HistoryFile { path, message }`
  displays `history file <path>: <message>`.
- [X] T004 [P] Implement `Error::HistoryFile { path: PathBuf, message: String }` in
  `src/errors.rs` (Display, no `source`). Make T003 pass.
- [X] T005 Write failing unit tests in `src/history.rs`: `History::new()` takes no capacity and
  keeps 50 000 upserts (none evicted); `iter_in_order()` yields QSOs oldest first by `sort_ts`,
  ties by id; tombstones still reject stale revisions. Update the existing eviction tests to the
  new rule (remove the ones that assert eviction).
- [X] T006 Implement in `src/history.rs`: drop `capacity` and eviction; add `iter_in_order()`.
  In `src/ingest/gate.rs` make `ResendGate::new()` unbounded (drop the capacity argument and its
  eviction; update its tests). Update callers: `Pipeline::new(broadcast_queue)` in
  `src/pipeline.rs`, `ingest::run` (no `gate_capacity`), `tests/load_ws.rs` and
  `tests/ws_protocol.rs` (drop `config.history_size`). Make T005 and all existing tests pass.
- [X] T007 Write failing unit tests in `src/config.rs` (contracts/command-line.md, R1, R13, R14):
  - `history_file` from `--history-file`, `LQM_HISTORY_FILE`, TOML `history_file`, with the usual
    precedence;
  - not set → `Some(<folder of country_data.path>/qso-history.adi)`; with `country_data.path =
    "/var/lib/live-qso-map/cty.dat"` → `/var/lib/live-qso-map/qso-history.adi`; with
    `./cty.dat` → `./qso-history.adi`;
  - `history_size` in TOML, `LQM_HISTORY_SIZE` or `--history-size` → loads, and the warnings list
    holds `history_size is no longer used: the history is not limited; remove it`;
  - `--enrich-only` alone → `Some(None)`; `--enrich-only x.adi` → `Some(Some("x.adi"))`;
    `--enrich-only` with `--save-config` → usage error;
  - `Config::default().history_file` is `None`.
- [X] T008 Implement in `src/config.rs`: `Cli::history_file`, `Cli::enrich_only:
  Option<Option<PathBuf>>` (`num_args = 0..=1`, `conflicts_with = "save_config"`), hidden
  `Cli::history_size` kept only to warn; `Config::history_file: Option<PathBuf>`; remove
  `Config::history_size`; the file layer accepts `history_size` and warns. Make T007 pass.
- [X] T009 [P] Update `src/config_export.rs` and `tests/config_export.rs`: no `history_size`;
  `history_file` written only when set explicitly (the default depends on the country data path).
  Tests first.
- [X] T010 Update `src/main.rs`: after `country_data.path` is resolved, resolve `history_file` the
  same way (`resolve_data_path`, R1). Update `tests/cli_help.rs` for `--history-file` and
  `--enrich-only [FILE]` (and `--history-size` absent from help). Tests first.
- [X] T011 Write failing unit tests in `src/enrich/mod.rs` (R3): after `enrich`, the record has
  `APP_LIVEQSOMAP_ADDED` listing exactly the fields that were absent before (in name order,
  including `MY_GRIDSQUARE` from the configuration, merged lookup fields such as `QTH`, and the
  `APP_LIVEQSOMAP_*` fields), and no logger field is listed; a record with a logger `GRIDSQUARE`
  does not list `GRIDSQUARE`.
- [X] T012 Create `src/history_file/mod.rs` (declare `read`, `write`, `restore`; the field-name
  constants) and register it in `src/lib.rs`. Implement the `APP_LIVEQSOMAP_ADDED` computation at
  the end of `enrich()` in `src/enrich/mod.rs`. Make T011 pass.
- [X] T013 [P] Write failing unit tests in `src/history_file/write.rs` (R10,
  contracts/history-file.md):
  - header: comment line with the UTC time, `ADIF_VER 3.1.4`, `PROGRAMID live-qso-map`,
    `PROGRAMVERSION` = crate version, `CREATED_TIMESTAMP` `YYYYMMDD HHMMSS`, `<EOH>`;
  - records one per line, in the order given; the file parses back to the same records;
  - `save(path, records)` replaces an existing file and leaves no `<name>.tmp`;
  - saving into a folder that does not exist returns `Error::HistoryFile` and leaves no file;
  - a failed save leaves the previous file byte-for-byte unchanged (make the target's folder
    read-only on Unix, `#[cfg(unix)]`; on other targets, simulate with a target path that is a
    directory).
- [X] T014 [P] Implement `src/history_file/write.rs`: `render(records, now) -> String` and
  `save(path, records, now) -> Result<()>` (write `<name>.tmp`, `sync_all`, `rename`; remove the
  temporary file on error). Make T013 pass.
- [X] T015 [P] Write failing unit tests in `src/history_file/read.rs` (R11):
  - a missing file → `Read::Missing`;
  - an empty file → zero records, not unreadable;
  - `history-external.adi` → 7 records and 1 malformed counted;
  - `history-not-adif.txt` → `Read::Unreadable(reason)`;
  - `move_aside(path, now)` renames to `<name>.unreadable-YYYYMMDDTHHMMSSZ` (no `:`), and
    returns the new path.
- [X] T016 [P] Implement `src/history_file/read.rs`: `read(path) -> Result<Read>` with
  `Read::{Missing, Unreadable(String), Records { records, malformed }}` and `move_aside`. Make
  T015 pass.
- [X] T017 Write failing unit tests in `src/history_file/restore.rs` (R3–R8, data-model table):
  - `as_received(record)` removes the fields listed in `APP_LIVEQSOMAP_ADDED` and the list
    itself; a record without the list is unchanged;
  - `classify(record, Mode::Service, filter)` and `classify(…, Mode::EnrichOnly, …)` give every
    row of the data-model table: invalid → `Skip`; band not considered → `Park` (service) /
    not parked (enrich-only); `LOOKUP_FAILED` → `Enrich`; `POS_REJECTED` with a saved position
    → `Reuse` (service) / `Enrich` (enrich-only); saved position → `Reuse`; otherwise `Enrich`;
  - `saved_position(record)` reads `LAT`/`LON` with the saved precision and source (a `cty`
    point stays `Country`/`Cty`), and is `None` when the source or precision is unknown.
- [X] T018 Implement `src/history_file/restore.rs`: `Mode`, `RestoreAction::{Skip, Park,
  Reuse, Enrich}`, `as_received`, `classify`, `saved_position`. Make T017 pass.

**Checkpoint**: `cargo test` green; nothing loads or saves yet.

---

## Phase 3: User Story 1 - The map survives a restart (Priority: P1) 🎯 MVP

**Goal**: QSOs on the map before a graceful stop are back after the restart, at the same positions,
with no new lookups; resends and corrections after the restart merge as before.

**Independent Test**: quickstart §1; `tests/history_file.rs` US1 tests.

### Tests for User Story 1 (write first, confirm they FAIL)

- [ ] T019 [P] [US1] Write failing integration tests in `tests/history_file.rs`, each with its own
  `temp_dir` and `config.history_file = Some(…)`:
  - **round trip** (SC-001, SC-002): send 5 QSOs (2 with `GRIDSQUARE`, 3 looked up through a
    `FakeProvider`), shut down, check the file has 5 records with `APP_LIVEQSOMAP_ADDED`; start a
    new service on the same file with a fresh counter: 5 upserts with the same positions (to
    1/60 000°), precisions and entities, and **zero** provider calls;
  - **resend after restart** is dropped (no upsert, no provider call), and a **correction**
    after the restart replaces the restored QSO (same id, higher revision); a **dupe** (same call,
    other time) stays a separate QSO (HR-9);
  - **QARTest logger id**: a restored record with `APP_LIVEQSOMAP_LOGGER_ID` is replaced by a
    QARTest correction carrying that id;
  - **missing file**: the service starts empty and creates the file at shutdown;
  - **deleted while running**: delete the file after startup; shutdown writes it again (FR-003);
  - **unreadable**: start on a copy of `history-not-adif.txt`; the service starts empty, the
    original is now `<name>.unreadable-…` with the same bytes, and a warning names it;
  - **save failure**: remove the history file's folder while the service runs, so the temporary
    file can't be created; shutdown logs an error and doesn't recreate the folder. On Unix only
    (`#[cfg(unix)]`), a second variant: a read-only folder leaves the previous file byte-identical;
  - **pending at shutdown**: with a `Hang` provider, send a QSO and shut down before it is
    released; the file holds it as received (no `APP_LIVEQSOMAP_ADDED`), and the next start
    enriches it;
  - **correction pending at shutdown**: a QSO is placed, then a correction is sent while the
    provider hangs, then shutdown; the file holds the correction as received, not the old
    version.
- [ ] T020 [P] [US1] Write a failing integration test in `tests/history_file.rs` for **live
  first** (FR-008, R2), with `workers = 1`: restore 20 records that need lookups from a `Hang` provider; send a live
  QSO with a `GRIDSQUARE`; its upsert arrives while the restored lookups are still hanging, and
  `/healthz` answers.
- [ ] T021 [P] [US1] Write a failing integration test in `tests/history_file.rs` for **scale**
  (SC-003): a generated file of 10 000 saved records (all `Reuse`) — the service answers
  `/healthz` within 5 s of `start_with_country`, all 10 000 are in the history within 5 s, and
  shutdown (including the save) completes within 5 s.

### Implementation for User Story 1

- [ ] T022 [US1] Extend `QsoEvent` in `src/ingest/mod.rs` with `saved: Option<AdifRecord>`
  (data-model). Add `restored_events(records, station, mode, filter) -> (Vec<QsoEvent>,
  Vec<(String, AdifRecord)>, LoadReport)`: `normalise` with `from` = the file (log the file
  path, not an address), logger id from `APP_LIVEQSOMAP_LOGGER_ID` (R5), `record` =
  `as_received`, `saved` = the full record for `Reuse`, parked records returned separately.
  Unit tests first in the same file (identity with and without logger id; parked by band).
- [ ] T023 [US1] Change `ingest::run` in `src/ingest/mod.rs` to take the restored events, run them
  through the gate in file order **before** the UDP loop (R2), and hand the forwarded ones to a
  feeder that sends them to a second channel (`restore_tx`); parked records also pass through the
  gate. Keep the live path unchanged.
- [ ] T024 [US1] In `src/enrich/mod.rs`: `run_worker` is unchanged for live events; in
  `spawn_tasks` (`src/pipeline.rs`) spawn **one more** `run_worker` on the restore receiver (R2: no
  worker reads both queues); add `reuse(event, config, lookups) ->
  EnrichedQso` for events with `saved` (R4: remote from `saved_position`, station from
  `apply_station` on the as-received record and the current configuration, entity from the
  country data, record = saved). Unit tests first (reuse keeps every saved field, keeps `cty`
  precision, recomputes the entity, makes no lookup).
- [ ] T025 [US1] Add the pending map to `Pipeline` in `src/pipeline.rs` (R9): insert when an event
  is queued, as `id → (rev, record)` (live and restored; the saved record when present, else the
  record as received); remove when a worker publishes or drops the result as stale. Add
  `Pipeline::records_to_save(parked) -> Vec<AdifRecord>`: history `iter_in_order()` records; a
  pending record replaces or joins them when its id is not in the history or its rev is higher
  (R9); parked records whose id is not in the history; all merged by `QSO_DATE` + `TIME_ON`. Unit
  tests first, including "pending rev 2 beats history rev 1".
- [ ] T026 [US1] Wire startup in `start_with_country` (`src/pipeline.rs`): when
  `config.history_file` is `Some`, `read` it (`Mode::Service`); `Missing` → empty;
  `Unreadable` → `move_aside` and warn, or return `Error::HistoryFile` with both reasons when the
  move fails (FR-011); log the load line of contracts/history-file.md; pass restored events and
  parked records to the tasks. Keep `RunningService` holding the parked records and the path.
- [ ] T027 [US1] Save on stop in `RunningService::shutdown` (`src/pipeline.rs`): after the tasks
  stop, `save(path, records_to_save(parked), now)`; log `history saved to <path>: N QSOs` or the
  error (the service still exits normally). Make T019–T021 pass.

**Checkpoint**: User Story 1 works on its own (quickstart §1).

---

## Phase 4: User Story 2 - An incomplete log is completed at startup and saved back (Priority: P2)

**Goal**: an external or partly enriched file is completed with the same rules as live QSOs;
logger fields never change; failed lookups are retried, rejected ones are not (normal run);
records on other bands are kept untouched.

**Independent Test**: quickstart §2; `tests/history_file.rs` US2 tests.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T028 [P] [US2] Write failing unit tests in `src/enrich/resolve.rs` (R7): `resolve_remote`
  reports each online source as answered (`Ok(Some)`, `Ok(None)`) or failed (`error`,
  `timeout`); the country data never counts as failed.
- [ ] T029 [P] [US2] Write failing unit tests in `src/enrich/mod.rs` (R7, FR-029):
  - every online source fails on the logged call → `APP_LIVEQSOMAP_LOOKUP_FAILED =
    "qrz:timeout,hamqth:error"` (configured order), the cty fallback is used, and the base-call
    round is not run;
  - one source answers "not found" and another fails → no note;
  - no online source configured → no note.
- [ ] T030 [US2] Write failing integration tests in `tests/history_file.rs` (`history-external.adi`
  and `history-saved.adi`, with `band_filter = hf-contest`):
  - records with `GRIDSQUARE` or `LAT`/`LON` are placed with source `logger` and no provider call
    (US2 scenario 1);
  - records with no position are looked up (scenario 2);
  - after shutdown, every original field of every record is unchanged, and the added fields are
    exactly those listed in `APP_LIVEQSOMAP_ADDED` (scenario 3, HR-11, SC-005);
  - the `2m` record never reaches the page, causes no provider call, and is written back
    byte-identical (FR-026);
  - the malformed record is skipped, and the load line counts it (FR-010);
  - the `POS_REJECTED` record is reused with its note and no provider call (FR-028);
  - the `LOOKUP_FAILED` record is looked up again; with a provider that now answers, the saved
    record has the new position and no note; with a provider that still fails, the note stays
    (FR-029).

### Implementation for User Story 2

- [ ] T031 [US2] Implement failure reporting in `resolve_remote` (`src/enrich/resolve.rs`): a
  `failed: Vec<(PositionSource, FailKind)>` and an `answered_online` count on `Lookup`. Make T028
  pass.
- [ ] T032 [US2] Write `APP_LIVEQSOMAP_LOOKUP_FAILED` in `enrich()` (`src/enrich/mod.rs`) when at
  least one online source is configured and none answered on the logged call; skip the base-call
  round in that case; log a warning per failed source as today. Make T029 and T030 pass.

**Checkpoint**: User Stories 1 and 2 work independently.

---

## Phase 5: User Story 3 - Enrich a file without running the service (Priority: P3)

**Goal**: `--enrich-only [FILE]` loads, completes every record, saves and exits, with no listener.

**Independent Test**: quickstart §3; `tests/enrich_only.rs`.

### Tests for User Story 3 (write first, confirm they FAIL)

- [ ] T033 [P] [US3] Write failing integration tests in `tests/enrich_only.rs`, calling
  `live_qso_map::pipeline::enrich_only(config, path, Some(sources), Some(country), stop)` where
  `stop` is a future (pending for a full run):
  - **no listeners**: hold the configured UDP and HTTP ports with sockets bound by the test; the
    run still succeeds (FR-017);
  - **every band**: with `band_filter = hf-contest`, the `2m` record is completed (FR-026);
  - **retries**: the `POS_REJECTED` and `LOOKUP_FAILED` records are looked up again (FR-028,
    FR-029); reused records cause no call;
  - **report**: the returned `LoadReport` counts placed from the record, by lookup, failed, and
    without a position (FR-020);
  - **missing file** → `Err(Error::HistoryFile)`, and no file is created; **unreadable file** →
    `Err`, the file is unchanged and not moved (FR-018);
  - **duplicates in the file**: two records with the same identity, the second changed → one
    record saved, the second version; an exact repeat → one record and one lookup;
  - **interrupt**: with a `Hang` provider and `stop` resolved after the first upsert, the file is
    written with the completed records and the others as loaded (FR-019).
- [ ] T034 [P] [US3] Write a failing test in `tests/cli_help.rs` (or a new `tests/enrich_only_cli.rs`
  running the built binary): `--enrich-only <missing file>` exits with code 1 and prints the
  error; `--enrich-only --save-config x` exits with code 2.

### Implementation for User Story 3

- [ ] T035 [US3] Implement `pub async fn enrich_only(config, path, sources, country, stop) ->
  Result<LoadReport>` in `src/pipeline.rs` (R12): read with `Mode::EnrichOnly` (missing or
  unreadable → `Error::HistoryFile`, no write); run the events through a fresh `ResendGate` in
  file order, as `ingest::run` does; build lookups as `start_with_country` does; no
  sockets; spawn `config.workers` workers on the restore queue only; wait until the queue and the
  pending map are empty, or `stop`; stop the workers; save; log the final line of
  contracts/history-file.md; return the report. Make T033 pass.
- [ ] T036 [US3] Dispatch in `src/main.rs`: with `cli.enrich_only`, take the path (or the
  configured `history_file`), call `enrich_only` with `stop` = `wait_for_signal()` (make it
  `pub(crate)` or move it to a shared place), and map `Ok` → exit 0, `Err` → log and exit 1. Make
  T034 pass.

**Checkpoint**: all three stories work independently.

---

## Phase 6: The page's count menu tops out at 1000 (FR-024, FR-025)

- [ ] T037 [P] Write failing tests in `src/web/protocol.rs` and `src/web/session.rs`
  (contracts/websocket-protocol.md): `hello.limits` is `[10, 20, 50, 100, 200, 500, 1000]`;
  `set_limit` with `limit: null` answers a snapshot with `limit: 1000` and at most 1000 QSOs out of
  50 000, and `snapshot` returns within 100 ms (measured with `Instant`, SC-007); `set_limit` with 7 is still ignored. Update the existing `tests/ws_protocol.rs` case that
  expected all QSOs for `null`.
- [ ] T038 Implement in `src/web/protocol.rs` (`LIMITS`, `is_allowed_limit`) and
  `src/web/session.rs` (`null` → `Some(1000)`). Make T037 pass.
- [ ] T039 [P] Write failing tests in `web/connection.test.js` and `web/map-state.test.js`:
  `LIMITS` is `[10, 20, 50, 100, 200, 500, 1000]`; `loadLimit` reads `"all"` as 1000 and the page
  saves `"1000"` back (constitution IV); `saveLimit(1000)` stores `"1000"`; `serverLimit(1000)` is
  1000; replace the old `null` expectations.
- [ ] T040 Implement in `src/web/assets/connection.js` (`LIMITS`, `loadLimit` migration and
  save-back), `src/web/assets/map-state.js` (`serverLimit` without the `null` branch),
  `src/web/assets/index.html` (option `1000` replaces `All`) and `src/web/assets/app.js` (the
  select no longer maps `all`). Make T039 pass; check `tests/browser/run.mjs` has no step that
  selects `All`.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T041 [P] Update `README.md`: a "History file" section (where it lives per install, what is
  saved, only graceful stops save, reset = stop, delete, start; the unreadable-file rename);
  `--enrich-only [FILE]` with an example and its exit codes; the configuration table (add
  `--history-file`/`LQM_HISTORY_FILE`, remove `--history-size`, note it is ignored with a
  warning); the page's count menu (10–1000); replace "The history lives in memory only and is lost
  on restart."
- [ ] T042 [P] Update `deploy/config.example.toml`: replace the `history_size` line with
  `# history_file = "/var/lib/live-qso-map/qso-history.adi"   # default: next to the country data`.
- [ ] T043 [P] Update `docs/ham-radio.md`: HR-11 names `APP_LIVEQSOMAP_ADDED` (fields the service
  added; everything else is the logger's) and `APP_LIVEQSOMAP_LOOKUP_FAILED`; §2 gains a short
  "History file" entry; add a dated changelog line.
- [ ] T044 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md` (`docs/releasing.md`):
  **Added** history file, `--history-file`, `--enrich-only`, the two new ADIF fields; **Changed**
  no history limit, page count menu tops out at 1000; **Removed** `history_size` (ignored with a
  warning).
- [ ] T045 Run the local gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --locked`, `node --test 'web/*.test.js'`, and quickstart §1–§6.
- [ ] T046 Mark the tasks done, commit per phase, push `feature/014-adif-persistence`, and open the
  pull request. Merge it once the gates are green; delete the branch afterwards (constitution,
  Development Workflow).

---

## Dependencies & Execution Order

- **Setup (T001–T002)** → **Foundational (T003–T018)** → stories.
- **Foundational order**: T003–T004 and T005–T006 first (other code uses the error and the
  unbounded history); T007–T010 (configuration) after T006; T011–T012 after T004; T013–T016 after
  T012 (the module exists); T017–T018 after T012.
- **US1 (T019–T027)** first: it is the MVP and brings loading, the second queue, the pending map
  and the save. T022 → T023 → T024 → T025 → T026 → T027.
- **US2 (T028–T032)** after US1 (its integration tests restart the service). T028/T029 can be
  written alongside US1.
- **US3 (T033–T036)** after US1 (reuses loading, workers and the save); independent of US2 except
  for the `LOOKUP_FAILED` retry case of T033, which needs T032.
- **Phase 6 (T037–T040)** depends only on Foundational; it can run in parallel with any story.
- **Polish (T041–T046)** last.
- **Shared files** (edit strictly in task order):
  - `src/enrich/mod.rs`: T011–T012, T024, T029, T032;
  - `src/pipeline.rs`: T006, T024, T025, T026, T027, T035;
  - `src/ingest/mod.rs`: T006, T022, T023;
  - `src/config.rs`: T007–T008;
  - `tests/history_file.rs`: T019, T020, T021, T030;
  - `tests/cli_help.rs`: T010, T034.

## Parallel Opportunities

- **Setup**: T001 and T002.
- **Foundational**: T003–T004 (`errors.rs`), T005–T006 (`history.rs`, `gate.rs`), and, once T012
  exists, T013–T014 (`write.rs`), T015–T016 (`read.rs`) and T017–T018 (`restore.rs`).
- **US1**: T019, T020 and T021 are separate tests in one new file — write them together.
- **US2**: T028 (`resolve.rs`) and T029 (`enrich/mod.rs`) in parallel.
- **US3**: T033 and T034.
- **Phase 6**: T037 (Rust) and T039 (page) in parallel, alongside any story.
- **Polish**: T041–T044 touch different files.

### Parallel example: Foundational file module

```text
Task: "T013 write.rs tests" → "T014 write.rs"
Task: "T015 read.rs tests" → "T016 read.rs"
Task: "T017 restore.rs tests" → "T018 restore.rs"
```

## Implementation Strategy

1. **MVP (US1)**: Phases 1–3. The map survives a restart with no new lookups; validate with
   quickstart §1.
2. **US2**: external logs completed; failed lookups noted and retried.
3. **US3**: `--enrich-only [FILE]`.
4. **Phase 6** whenever convenient: the page tops out at 1000.
5. **Polish**: README, example config, domain document, changelog, gates, pull request.
