---
description: "Task list for 016-map-autozoom-fixes"
---

# Tasks: Map Auto-Zoom Fixes

**Input**: Design documents from `/specs/016-map-autozoom-fixes/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R7), data-model.md,
contracts/configuration.md, contracts/websocket-protocol.md, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the change it covers,
run it, and confirm it fails first.

- **Frontend unit tests**: `node --test web/`.
- **Rust tests**: `cargo test`.
- **Browser test**: `cargo build --release && node tests/browser/run.mjs`. Its CI proof is a
  release rehearsal (LL-007).

**Conventions:**

- Pure framing maths lives in `src/web/assets/autozoom-geo.js`, with no DOM and no MapLibre. DOM
  and MapLibre calls stay in `src/web/assets/app.js`.
- The geometry constants already in `autozoom-geo.js` are reused: `DEFAULT_FOV_RAD`, `TILE_SIZE`,
  `MAX_LAT`, `cameraDistance`, `globeRadiusPx`.
- The new setting follows the `map.recent_header` path everywhere:
  - `src/config.rs`: the CLI arg near line 230, `FileMap` near line 570, `MapConfig` near line
    360, the default near line 451, layering near line 1154, the export near line 664;
  - `src/web/protocol.rs`: `hello`, near line 185.
- No inline lint suppressions (LL-001). `unwrap`/`expect` are fine in tests only.
- **Snapshot**: the operator's 20 QSO positions and QTH from spec Assumptions. Only callsigns and
  coordinates are copied, never names or addresses.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US3)
- **[R:TIER]**: risk tier (default GREEN)

---

## Phase 1: Setup

- [X] T001 Confirm the baseline on `feature/016-map-autozoom-fixes`: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` and `node --test web/` pass. Record
  the existing globe tests in `web/autozoom-geo.test.js` that this feature rewrites: lines
  ~145–200 (`fitsGlobe…`, `globeZoomForCap…`).

---

## Phase 2: Foundational (shared fixture and the failing regression)

**Purpose**: the snapshot data that every story's tests use, and the browser regression that
must fail on the current code (spec FR-011, SC-004).

- [X] T002 [P] Add a `SNAPSHOT` constant at the top of `web/autozoom-geo.test.js`:
  - `qth: { lat: 39.051738, lon: 8.37711 }`;
  - `points`: the 20 `[lat, lon]` pairs from spec Assumptions (VK3ATL, VK6LG, SM5VTP, SK6D, DL9ZP,
    M5S, G0MBA, DF6QV, ON6IO, M1STI, DM1SV, OK2TS, TM1FC, F4ICZ, YT4M, YO8SBQ, EA1FUM, EE7I, CT2FUN,
    CT2IOV);
  - `europe`: the same points without VK3ATL and VK6LG;
  - one shared `PADDING = { top: 60, right: 100, bottom: 240, left: 110 }` (research R1).

  Add one passing sanity test: `boundingCap([qth, ...points]).radiusDeg` is within 0.5 of 79.6.
- [X] T003 [P] Add window resizing to the browser runner. In `tests/browser/cdp.mjs`, add
  `page.resize(width, height)`: `Emulation.setDeviceMetricsOverride` with `deviceScaleFactor: 1`,
  `mobile: false`. Add `page.clearSize()` (`Emulation.clearDeviceMetricsOverride`). In
  `tests/browser/lib.mjs`:
  - add `SNAPSHOT_QSOS`: the 20 `{ call, grid }` from the snapshot's GRIDSQUARE values (QF21eu,
    OF85wb, JO88gn, JO67fw, JO53ro, IO92fs for M5S, JO01nt, JO41mq, JO21ha, IO90px, JN49id, JN89nc,
    JN18eu, JN08qb, KN04ev, KN24xn, IN82di, IN80dj, IN50sf, IM57xd);
  - add `fitsFree(view, points, free)`: true when every `[lat, lon]` projects inside `free`
    (Web Mercator at `view.zoom`, centred on `view`, with longitudes wrapped the shorter way).

  Test both helpers in `web/browser-lib.test.js`.
- [X] T004 [R:YELLOW] Add the browser step `autozoom-fit` (12) at the end of `STEPS` in
  `tests/browser/run.mjs`:
  1. Send `SNAPSHOT_QSOS` on 20m with `sendQso`. They are the newest, so the default Last 20
     shows exactly them.
  2. Choose the globe preference with the projection button, then turn auto-zoom on.
  3. For each of `[1024, 768]` and `[1920, 1080]`, call `page.resize` and wait for `moveend`
     (`data-view` changes). Then assert:
     - the projection button reads `Globe` (the flat view is on screen, FR-007);
     - the house is drawn within 2 % of the free-area centre (SC-001), the free area coming from
       the page's `data-padding`; vertically, a view held at the map's ±85° edge is accepted
       (FR-003, seen at 1024 × 768);
     - `fitsFree(view, points)` holds, and `fitsFree` at `view.zoom + 0.25` fails (SC-004, the
       zoom is the highest that fits).
  4. Take a screenshot per size as `12-autozoom-fit-<w>x<h>.png`, then `page.clearSize()`.

  Run it on the current code and confirm it fails: the globe stays on screen. Document the step
  in `tests/browser/README.md` (12 `autozoom-fit`).

**Checkpoint**: the snapshot fixture exists and the regression fails on today's page.

---

## Phase 3: User Story 1 - Readable globe or a switch to flat (Priority: P1) 🎯 MVP

**Goal**: the globe is kept only while every framed marker is at least `map.globe_border_deg`
inside the visible horizon at the free-area fit zoom, or the horizon is out of view. Otherwise
the page shows the flat view. The globe never shrinks to a dot.

**Independent Test**: `node --test web/` passes the border cases. With the globe preference and
auto-zoom on, the snapshot shows the flat view, and the European-only set shows the globe filling
the free area (quickstart steps 2–4).

### Tests for User Story 1 ⚠️ write first, see them fail

- [X] T005 [P] [US1] Rust tests in the `#[cfg(test)] mod tests` of `src/config.rs`:
  - the default is 10;
  - the file `[map] globe_border_deg = 20`, `LQM_GLOBE_BORDER_DEG=30` and
    `--globe-border-deg 40` each win in the usual CLI > env > file order;
  - 0 and 45 are accepted;
  - -1, 45.5, `NaN`, `inf` and an unparseable env value are refused, with a config error that
    names `map.globe_border_deg`.
- [X] T006 [P] [US1] Rust tests for `hello` in `src/web/protocol.rs` (`globe_border_deg` is
  serialised from `MapConfig`, a changed value shows up). In `tests/ws_protocol.rs`
  (`every_page_receives_hello_snapshot_and_live_upserts`), assert
  `hello["globe_border_deg"] == 10.0` and `hello["protocol"] == 4`.
- [X] T007 [P] [US1] Rust test in `src/config_export.rs` (`--save-config` section): the written
  `[map]` holds `globe_border_deg`. Add the key to the `for key in [...]` list near line 190.
  The existing `example_lists_every_exported_setting` must then fail until T012.
- [X] T008 [P] [US1] JS tests in `web/autozoom-geo.test.js` for `globeZoomForCap(cap, { width,
  height, padding, borderDeg, maxZoom })` (research R2, R3):
  - the snapshot cap at 1024 × 768 and at 1920 × 960 with `PADDING` and `borderDeg: 10` →
    `null`, and also with `borderDeg: 0` (the VK QSOs are behind the horizon);
  - the `europe` cap at 1920 × 960 → a number. At that zoom the globe radius is at least 250 px,
    never the old 30–40 px dot;
  - a 0.001° cap → 12, with `borderDeg: 10` and `borderDeg: 45` (horizon out of view);
  - the `europe` cap with `borderDeg: 45` → `null` when the cap plus 45° exceeds the horizon at
    the fit zoom;
  - the returned zoom never lies below the free-area fit (no shrinking: compare with the zoom
    where the cap edge sits at half the free area);
  - rewrite the old `fitsGlobe…` and limb tests (`no zoom when the cap reaches the limb`,
    `IS, JA, W6 and DL fit on a globe seen from the Arctic`, `the cap edge lands … in front of
    the limb`) to the new rule. Keep each case, with the expectation the new rule gives.
- [X] T009 [P] [US1] JS tests in `web/view.test.js` for `globeBorder(hello)`: it returns
  `hello.globe_border_deg` when it is a finite number in [0, 45]. Otherwise (missing, string,
  NaN, -1, 46) it returns 10.

### Implementation for User Story 1

- [X] T010 [US1] [R:YELLOW] In `src/config.rs`, add `globe_border_deg`:
  - the clap arg `--globe-border-deg` (`Option<f64>`, doc "Environment: LQM_GLOBE_BORDER_DEG.
    Default: 10.");
  - the `FileMap` field and `MapConfig.globe_border_deg: f64`;
  - `layer("map.globe_border_deg", …)` plus a range check that accepts only finite values in
    0..=45, with the error "<v> is not between 0 and 45";
  - the default 10;
  - the export in the save-config struct (`Some(config.map.globe_border_deg)`).

  T005 and T007 pass.
- [X] T011 [US1] In `src/web/protocol.rs`, add `globe_border_deg: f64` to the `hello` struct,
  filled from `map.globe_border_deg`, and update the test fixtures near line 295. T006 passes.
- [X] T012 [P] [US1] In `deploy/config.example.toml` `[map]`, add the commented-out
  `globe_border_deg = 10` with the two-line comment from contracts/configuration.md. In
  `README.md`, add the row "Globe border (degrees inside the horizon) |
  `--globe-border-deg` / `LQM_GLOBE_BORDER_DEG` | `10`" next to the recent-header row (line ~203).
  `example_lists_every_exported_setting` passes again.
- [X] T013 [US1] [R:YELLOW] In `src/web/assets/autozoom-geo.js`:
  - delete `fitsGlobe` and `LIMB_SHARE`;
  - in `globeZoomForCap`, keep the free-area radius (`half·c / denominator`, unbounded when
    `denominator ≤ 0`), capped at `maxZoom`;
  - at that radius R, compute `H = acos(R / (c + R))` and the horizon's on-screen radius (the
    `capScreenRadius` formula at θ = H);
  - when that radius is below half the map's diagonal and `cap.radiusDeg + borderDeg > H`
    (degrees), return `null`; never lower the zoom to make it pass;
  - keep the `minZoom` floor (`null` below it).

  Update the header comment to cite feature 016 FR-006/FR-007. T008 passes.
- [X] T014 [P] [US1] Add `globeBorder(hello)` to `src/web/assets/view.js` (default constant
  `GLOBE_BORDER_DEG = 10`). T009 passes.
- [X] T015 [US1] In `src/web/assets/app.js`:
  - remove `GLOBE_MARGIN_DEG` and the `fitsGlobe` import;
  - add `let globeBorderDeg = 10`, set from `globeBorder(hello)` in `onHello`;
  - in `currentFrame`, call `globeZoomForCap(cap, { width, height, padding, borderDeg:
    globeBorderDeg, maxZoom: MAX_ZOOM })` and keep `fitsGlobe: !cap || globeZoom !== null`.

  The tooltip text for the automatic flat view (line ~357) stays.

**Checkpoint**: on the snapshot, the globe preference with auto-zoom on shows the flat view. T004
still fails on the centring and zoom asserts until US2.

---

## Phase 4: User Story 2 - Flat view centred on the station (Priority: P1)

**Goal**: with a known QTH, every flat refit under auto-zoom centres the house in the free area
at the highest zoom that holds every framed point (FR-001 to FR-004).

**Independent Test**: `node --test web/` passes the centred-frame cases. With the flat
preference and auto-zoom on, the house is in the middle of the free area and every marker is in
view (quickstart step 3).

### Tests for User Story 2 ⚠️ write first, see them fail

- [X] T016 [P] [US2] JS tests in `web/autozoom-geo.test.js` for `flatFrameAround(qth, points, {
  width, height, padding, maxZoom })`, which returns `{ center: [lon, lat], zoom }`
  (research R4):
  - snapshot at 1920 × 1080 with `PADDING`: `center` equals the QTH, and every point projected
    at `zoom` lies inside the free area (with a 1 px tolerance), while one of them leaves it at
    `zoom + 0.05`;
  - longitudes are measured the shorter way: a ZL QTH (−41, 174.8) with a W6 QSO (37, −122) uses
    dLon ≈ 63°, not 297°;
  - every point at the QTH (or the QTH alone) → `maxZoom`;
  - a QSO 180° of longitude away → the zoom shows the full world width in the free width;
  - a QTH at 80 N with a QSO at −40: the result keeps every point inside the free area once
    MapLibre's ±85.05° clamping moves the centre (FR-003; model the clamp in the test);
  - free area of zero or negative size → the same result as `paddingFor`'s small-screen case
    (uniform gap) gives, never `NaN`.
- [X] T017 [P] [US2] JS test in `web/autozoom-geo.test.js`: with no displayed markers, the
  `fallbackFrame` corners passed as points to `flatFrameAround` still centre on the QTH (spec
  edge case "No QSOs displayed").

### Implementation for User Story 2

- [X] T018 [US2] [R:YELLOW] Implement `flatFrameAround` in `src/web/assets/autozoom-geo.js` as
  research R4 states:
  - dLon wrapped to [−180, 180];
  - Mercator y with latitudes clamped to ±`MAX_LAT`;
  - zoom = min(the width term, the height term, `maxZoom`);
  - a pole correction that lowers the zoom only when the clamped centre would push a point out.

  T016 and T017 pass.
- [X] T019 [US2] In `src/web/assets/app.js` `fitNow`, on the flat branch: when `qth` is known,
  `map.easeTo({ center, zoom, padding, duration: 800 })` with `flatFrameAround(qth,
  framePoints(bounds), …)`. Keep `fitBounds` only when the QTH is unknown (FR-004). The globe
  branch and the world-view branch are unchanged. T004 passes locally.

**Checkpoint**: US1 + US2 together turn the browser step `autozoom-fit` green at both sizes.

---

## Phase 5: User Story 3 - Correct auto-zoom level (Priority: P2)

**Goal**: every refit uses the zoom for the projection on screen after that refit's own
automatic switch (FR-009, FR-010), and the result is pinned by tests (FR-011).

**Independent Test**: `node --test web/autozoom.test.js` passes the sequence case, and the browser
step `autozoom-fit` is green in a rehearsal.

### Tests for User Story 3 ⚠️ write first, see them fail

- [X] T020 [P] [US3] JS test in `web/autozoom.test.js`: `fitted(false, 'globe')` → `'mercator'`,
  then `fitted(true, 'globe')` → `'globe'`, then `fitted(false, 'globe')` → `'mercator'` on one
  instance. `projectionChosen()` clears `autoFlat`. If this already passes, keep it as a
  regression guard and note so in the commit.
- [X] T021 [P] [US3] JS test in `web/autozoom-geo.test.js` (SC-005): for the snapshot, `europe`
  and a 3-point Pacific set at 1024 × 768 and 1920 × 1080, the flat zoom from `flatFrameAround`
  is within 0.25 of a brute-force search (step 0.01) for the highest zoom whose projected points
  all fit. Do the same for `globeZoomForCap` against a search over `capScreenRadius ≤ half`.

### Implementation for User Story 3

- [X] T022 [US3] In `src/web/assets/app.js` `fitNow`, make sure `showProjection(...)` runs before
  the camera is chosen, and that the branch tests `shown` after it (FR-009). Change nothing if T020
  and T004 already prove this. Otherwise move the call, with a one-line comment citing FR-009.
- [X] T023 [US3] If T004 still shows a flat zoom below the `fitsFree` maximum after T019 (the
  0.85 symptom, research R5), find the cause with `data-view` and the step's screenshots (padding,
  projection timing). Fix it in `app.js` and add a unit test at the level where the cause sits.
  Record "no further cause" in research R5 if the step is already green.

**Checkpoint**: all three stories are done. Every local test is green.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T024 [P] Add the 016 entries under `## [Unreleased]` in `CHANGELOG.md`:
  - Fixed: the globe no longer shrinks to a dot under auto-zoom, it switches to flat; the flat
    view under auto-zoom centres on the station and uses the closest zoom;
  - Added: `map.globe_border_deg` (default 10°).
- [ ] T025 [P] Update `specs/004-qth-autozoom/spec.md`: under FR-016, add a one-line pointer
  "Superseded in part by feature 016 (globe border, flat centring)" so readers find the new rule.
- [ ] T026 Run every gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `node --test web/`, then `cargo build --release && node tests/browser/run.mjs`.
  All green.
- [ ] T027 Walk through quickstart.md steps 1–6 by hand at 1920 × 1080 in Firefox (the
  operator's browser), and note the result in the PR description.
- [ ] T028 [R:YELLOW] Release rehearsal (LL-007, `docs/ci.md`): start the manual rehearsal
  pipeline on this branch, and confirm the browser step `autozoom-fit` runs and is green. Record
  the pipeline number here when ticking.

---

## Dependencies & Execution Order

- **Phase 1** → **Phase 2** (T002, T003 in parallel; T004 after T003) → stories.
- **US1 (Phase 3)** and **US2 (Phase 4)** are independent in code. They share
  `web/autozoom-geo.test.js` and `src/web/assets/autozoom-geo.js`, so edits to those files are
  sequential, but each story's Rust and `view.js` work runs in parallel with the other story.
- **T004 turns green only after T015 and T019** (both stories).
- **US3 (Phase 5)** after US1 and US2: T022 and T023 judge the result of both.
- **Polish** last. T028 needs everything committed and pushed.

Within US1: T005–T009 (tests, all [P]) → T010 → T011 → T012 [P] / T013 → T014 [P] → T015.
Within US2: T016, T017 → T018 → T019.

## Parallel Examples

```text
# US1 tests, together:
T005 src/config.rs tests · T006 src/web/protocol.rs + tests/ws_protocol.rs · T007 src/config_export.rs
T008 web/autozoom-geo.test.js · T009 web/view.test.js

# Across stories, once Phase 2 is done:
T010–T012 (Rust + example config, US1)  ‖  T016–T018 (flat frame maths, US2)
```

## Implementation Strategy

- **MVP**: Phases 1–3 (US1). The globe no longer shrinks to a dot, and the snapshot shows the
  flat view: the bug visible on the live page is gone.
- **Next**: US2 centres the flat view, which is now also where the globe falls back to. US1 + US2
  turn T004 green.
- **Then**: US3 pins the zoom level and closes the unexplained 0.85 symptom. Polish, then the
  rehearsal before merging.
