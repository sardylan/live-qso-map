---
description: "Task list for 004-qth-autozoom"
---

# Tasks: QTH Marker, Auto-Zoom and Map Panels

**Input**: Design documents from `/specs/004-qth-autozoom/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write every test task before the implementation it
covers, then run it and confirm it fails first. Use `cargo test` for Rust and `node --test web/` for
the frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`). The dev-only generator uses
`python3 -m unittest tools/test_entity_extents.py`.

**Conventions** (as in features 001–003):

- **Errors**: the crate `Error`/`Result` from `src/errors.rs` (`docs/rust-guidelines.md` §1).
- **Domain rules**: `docs/ham-radio.md` §1 (UTC, HR-5 callsign comparison, HR-6 entities, HR-7
  precision and source).
- **Interface rules**: `docs/http-api.md`. JSON everywhere, including errors (API-3).
- **Page code**:
  - every decision goes in a pure module under `src/web/assets/`, with its tests in
    `web/*.test.js`; `app.js` is glue only;
  - QSO text reaches the DOM only through `textContent`.
- **Rust tests**:
  - unit tests go in `#[cfg(test)] mod tests` in the same file;
  - every integration test file starts with
    `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` and uses `tests/common`.
- **Commits**: one commit after each phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US5 from spec.md

---

## Phase 1: Setup

- [X] T001 Create the module skeletons. `cargo build` and `node --test web/` must still pass.
  - Add `pub mod entity_extents;` to `src/lib.rs` and create `src/entity_extents.rs` with the doc
    comment `//! Bundled DXCC entity extents (feature 004, research R2).`
  - Create the empty ES modules `src/web/assets/qth.js`, `autozoom-geo.js`, `autozoom.js` and
    `recent.js`, each with a header comment that says it is pure (no DOM, no MapLibre).
  - Add `/.cache/` to `.gitignore`.
- [X] T002 Add the routes `/qth.js`, `/autozoom.js`, `/autozoom-geo.js` and `/recent.js` to
  `src/web/mod.rs` (`include_str!` constants, `text/javascript`). In `tests/ws_protocol.rs`, extend
  the asset test to fetch all four and assert `200` with `text/javascript`. Write the test first.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: protocol 3 and the server-side station data (FR-025), which US1 and US2 both read.

### Tests first (write, run, confirm they FAIL)

- [X] T003 [P] Write unit tests in `src/web/protocol.rs` (contracts/websocket-protocol.md):
  - **protocol**: `hello` has `protocol: 3`;
  - **station**:
    - built from a `StationConfig` with callsign `" is0mzk "` and locator `JM49NH`, it serialises
      to `{"callsign":"IS0MZK","locator":"JM49NH","position":{"lat":…,"lon":…,"precision":"locator6","source":"config"}}`;
    - explicit latitude and longitude win and give precision `coordinates`;
    - with only a callsign, `position` is `null`;
    - with nothing configured, `station` is `null`;
  - **entity**: `entity` is `null` when `HelloStation` has no entity, and
    `{"prefix":"IS","name":"Sardinia","bounds":[w,s,e,n]}` when it has one;
  - **unchanged fields**: `bands`, `basemap`, `limits`, `default_limit` and `fallback_color` are
    unchanged (feature-003 filter included).
- [X] T004 [P] In `tests/ws_protocol.rs`, assert that the `hello` received over a real WebSocket has
  `protocol: 3`, a `station` object matching `test_config()`'s station (or `null` when it has none),
  and an `entity` key.

### Implementation

- [X] T005 Implement in `src/web/protocol.rs` (data-model "Server side"):
  - set `PROTOCOL_VERSION = 3`;
  - add `StationView`, `EntityView`, and `HelloStation { station: Option<StationView>, entity:
    Option<EntityView> }` with `HelloStation::new(&StationConfig, Option<EntityView>)`;
  - make the position follow `enrich::station::config_station`; make that function `pub(crate)`
    rather than duplicating it;
  - add `station` and `entity` to `ServerMessage::Hello`, and change the signature to
    `hello(map, filter, &HelloStation)`.
- [X] T006 Wire it through:
  - in `src/pipeline.rs`, `start` builds `HelloStation` once (entity `None` for now; T021 fills it)
    and passes it to `web::serve`;
  - `src/web/mod.rs` stores it in `AppState` (`Arc<HelloStation>`);
  - `src/web/session.rs` `drive` passes it to `ServerMessage::hello`;
  - update the existing session tests.

  T003 and T004 must pass.

**Checkpoint**: pages receive protocol 3 with `station`. The existing page still works, because it
ignores the new fields.

---

## Phase 3: User Story 1 - See the QTH and the paths from it (Priority: P1) 🎯 MVP

**Goal**: a house at the QTH, with black great-circle lines from it to every marker (FR-001–FR-006).

**Independent Test**: quickstart §2.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T007 [P] [US1] Write `web/qth.test.js` for `currentQth(helloStation, state)` and
  `houseFeature(qth)`:
  - a configured position wins over QSOs (`origin: 'config'`, with callsign and locator from
    `hello`);
  - with no configured position, the result is the `station` of the newest QSO in state by
    `sort_key` (not the newest received), with `origin: 'qso'`, callsign `station_call` and
    locator `adif.MY_GRIDSQUARE`;
  - QSOs without a `station` are skipped;
  - `null` when there is nothing;
  - after `applyRemove` of the QSO that supplied the QTH, the next newest one supplies it;
  - `houseFeature(null)` has 0 features; otherwise there is one Point `[lon, lat]`;
  - `HOUSE_SVG` is a constant string that starts with `<svg` and contains no `{` placeholders.
- [X] T008 [P] [US1] Update `web/layers.test.js` for `lineFeatures(markers, qth)`:
  - every line starts at the `qth` (great circle from `[qth.lat, qth.lon]`), whatever each QSO's
    `station`;
  - no lines when `qth` is `null`;
  - the properties are `{key, zRank}`, with no `color`;
  - a path that crosses the antimeridian (QTH `JM49`, marker in Hawaii) keeps unwrapped
    longitudes;
  - remove the old "line from the station" and "no line without a station" tests, which this
    feature supersedes (FR-004).

### Implementation for User Story 1

- [X] T009 [P] [US1] Implement `src/web/assets/qth.js`: `currentQth`, `houseFeature` and
  `HOUSE_SVG`, a 28×28 house with a dark `#222` outline and a light `#fff8e7` fill (research R3).
  T007 must pass.
- [X] T010 [P] [US1] Change `lineFeatures` in `src/web/assets/layers.js` to take `qth` and drop the
  `color` property. T008 must pass.
- [X] T011 [US1] Glue in `src/web/assets/app.js` (research R3, R4):
  - load `HOUSE_SVG` into an `Image`, then `map.addImage('qth-house', …)` after `load`;
  - add a `qth` GeoJSON source and a `symbol` layer (`icon-image: 'qth-house'`,
    `icon-allow-overlap`, `icon-ignore-placement`), placed so the order is `qso-lines` → `qth` →
    `qso-markers`;
  - the line paint becomes `line-color: '#000'`, `line-opacity: 0.6`, `line-width: 1.5`;
  - keep `hello.station` from `onHello`;
  - `render()` computes `currentQth` and sets the `qth` and `qso-lines` data.

  Feature-001 to 003 behaviour must be unchanged apart from the line colour (FR-026).
- [X] T012 [US1] Add the house popup in `src/web/assets/app.js` (FR-006):
  - clicking the `qth` layer opens a popup built with `textContent`: the callsign (or "station"),
    the locator or `lat, lon` to 4 decimals, and `source · precision` (HR-7);
  - open it only when no QSO marker is under the click point
    (`map.queryRenderedFeatures(point, {layers: ['qso-markers']})` is empty); otherwise the marker's
    popup wins (FR-005);
  - track it like the marker popups (`openPopup`/`openKey = 'qth'`);
  - add a style in `src/web/assets/app.css`.
- [X] T013 [US1] Manually run quickstart §2 in a browser. Record the result in
  `specs/004-qth-autozoom/checklists/verification.md` (create the file).

**Checkpoint**: the house and the black lines work; US1 is deliverable on its own.

---

## Phase 4: User Story 2 - Auto-zoom keeps everything in view (Priority: P1)

**Goal**: an auto-zoom toggle that refits after every change, keeps clear of panels, frames the
entity when there are no QSOs, pauses for popups and turns off on manual moves (FR-007–FR-014,
FR-012a).

**Independent Test**: quickstart §3.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T014 [P] [US2] Write Rust unit tests in `src/enrich/cty.rs` for `CountryData::entity_of`,
  using `tests/fixtures/cty-excerpt.dat`:
  - `IS0MZK` → `("IS", "Sardinia")`;
  - `IT9ABC` → `("IT9", "Sicily")`, with the WAE `*` stripped;
  - `I2XYZ` → `("I", "Italy")`;
  - `is0mzk` is handled like `IS0MZK` (HR-5);
  - `K1ABC` → `("K", "United States")`;
  - `KH6XX` → `("KH6", "Hawaii")`;
  - a portable call (`IS0MZK/P`) resolves the same way `lookup` does;
  - an unknown call → `None`.
- [X] T015 [P] [US2] Write Rust unit tests in `src/entity_extents.rs`:
  - parse `{"source":"…","extents":{"IS":[8.13,38.86,9.83,41.31],"3D2":[176.8,-19.3,-178.2,-16.0]}}`;
  - `get("IS")` returns its box, and `get("XX")` returns `None`;
  - an antimeridian box (`w > e`) is accepted;
  - errors (`Error::Config { setting: "entity_extents", .. }`): `s ≥ n`, a latitude outside
    ±90, a longitude outside ±180, a box that isn't 4 numbers, malformed JSON;
  - `ExtentTable::embedded()` parses the committed `src/web/entity_extents.json` and contains
    `IS`, `I`, `K` and `VK` (SC-003). Its `IS` box contains Cagliari (39.22, 9.12), its `I` box
    contains Rome (41.9, 12.5) but not Cagliari, `K` contains Kansas City but not Honolulu, and
    `VK` contains Sydney.
- [X] T016 [P] [US2] Write `tools/test_entity_extents.py` (unittest, standard library only) for the
  generator's pure functions:
  - `min_lon_span([...])` returns `(w, e)` with `w > e` for points on both sides of 180° (Fiji),
    and the ordinary span otherwise;
  - `select(features, selector)` handles `admin0`, `admin1`, `exclude_admin1` and `clip`;
  - the output is deterministic: sorted keys, values rounded to 2 decimals, no timestamp;
  - `check_coverage(cty_prefixes, mapping)` fails, naming the entity, when an entity is neither
    mapped nor listed under `[unmapped]`.

  Use small inline GeoJSON fixtures; no download.
- [X] T017 [P] [US2] Write `web/autozoom-geo.test.js` (research R5, R6):
  - `framedPoints(qth, markers)` includes the QTH when there is one;
  - `fallbackFrame(qth, entity)`:
    - with an entity, the result is its bounds (unwrapped so `e > w`, e.g. Fiji `[176.8, …,
      181.8, …]`);
    - with no entity, a 500 km box around the QTH (half-height ≈ 4.5° of latitude, width widened
      by `1/cos(lat)`);
    - with no QTH, `null`;
  - `flatBounds`:
    - Europe + Japan gives the ordinary span;
    - Europe + New Zealand gives the short way, with `e > 180`;
    - a single point gives a degenerate box;
  - `paddingFor(mapRect, rects, {gap: 12, minFree: [240, 160]})`:
    - a legend at the bottom right pads the right or bottom edge by its depth plus 12;
    - a box at the bottom left pads the bottom;
    - a limit selector at the top right pads the top;
    - hidden panels (null rects) are ignored;
    - when the free area is smaller than 240×160, the result is a uniform 12 px.
- [X] T018 [P] [US2] Write `web/autozoom.test.js` for the controller, with an injected clock
  (data-model "Auto-zoom controller"):
  - **toggle**: on → `fitNow`; off → no fit, and later `change` events produce no fit;
  - **user move**: `userMove` while on → off, plus a `saveOff` action, and 0 fits after it
    (SC-004);
  - **throttle**: 10 `change` events within 1 s → exactly one `fit` on `tick`, the next one no
    earlier than 1000 ms after it (FR-014, SC-005);
  - **popups**:
    - while `popupOpen`, `tick` never fits;
    - `popupClosed` while enabled → exactly one catch-up fit;
    - `popupClosed` while disabled → none (FR-012a);
  - **page-made moves**: no event is fed for them, so the state is unchanged (document this in the
    test name).
- [X] T019 [P] [US2] Extend `web/view.test.js`:
  - `loadAutoZoom`/`saveAutoZoom` (`lqm.autozoom`): the default is on; `off` round-trips; an
    unknown value → on; a storage that throws → on, and saving swallows the error.

### Implementation for User Story 2

- [X] T020 [US2] Implement `ExtentTable` in `src/entity_extents.rs`: `parse(&str) -> Result<Self>`,
  `embedded() -> Result<Self>` (`include_str!("web/entity_extents.json")`) and `get(prefix)`.
  T021 loads it once in `start`; a failure is a startup error. Commit a placeholder
  `src/web/entity_extents.json` with only `IS`, `I`, `K` and `VK` so that T015 can pass before
  T023. T015 must pass.
- [X] T021 [US2] Implement `Entity.prefix` (the primary prefix column without `*`) and
  `CountryData::entity_of` in `src/enrich/cty.rs`. Then, in `src/pipeline.rs`:
  - `build_sources` also returns the parsed `CountryData`;
  - `start` loads `ExtentTable::embedded()` (T020) once and resolves the entity from the
    configured station callsign through it, filling `HelloStation.entity`;
  - a missing callsign, missing country data, an unknown entity or no extent each give `None`,
    each logged at debug level.

  T014 must pass, and so must the T003 tests.
- [X] T022 [US2] Implement `tools/entity_extents.py` (research R2):
  - CLI: `--cty`, `--mapping`, `--out`, `--cache .cache/naturalearth`;
  - download the two Natural Earth v5.1.2 GeoJSON files from
    `raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/` once;
  - provide `min_lon_span` and `select`, then write the deterministic JSON;
  - print the cty.dat entities that have no mapping.

  T016 must pass.
- [X] T023 [US2] Write `tools/entity_extents.toml`:
  - map every entity in the current `cty.dat` that Natural Earth represents (admin-0 subunits,
    admin-1 units, exclusions, clip boxes);
  - add a `note` for each non-obvious choice (for example `UA` European Russia clipped at 60°E,
    `K` without `US-AK` and `US-HI`);
  - list every entity that cannot be mapped under `[unmapped]`, with a reason (for example "not
    in Natural Earth at 1:10m");
  - **acceptance**: the generator's coverage check passes, so every entity in the current
    `cty.dat` is mapped or listed, and at least 95 % of entities are mapped;
  - regenerate `src/web/entity_extents.json`;
  - spot-check 10 boxes on a map (the IS, I, IT9, K, VK, JA, PY, ZS, UA0 and 3D2 boxes).

  T015 must still pass.
- [X] T024 [P] [US2] Implement `framedPoints`, `fallbackFrame`, `flatBounds` and `paddingFor` in
  `src/web/assets/autozoom-geo.js`. T017 must pass.
- [X] T025 [P] [US2] Implement the controller in `src/web/assets/autozoom.js` (`createAutoZoom`,
  `toggle`, `change`, `userMove`, `popupOpened`, `popupClosed`, `tick`). T018 must pass.
- [X] T026 [P] [US2] Implement `loadAutoZoom` and `saveAutoZoom` in `src/web/assets/view.js`.
  T019 must pass.
- [X] T027 [US2] Glue in `src/web/assets/app.js` (research R6, R7, R10):
  - **button**: an `AutoZoomControl` button (`aria-pressed`, `title`, `aria-label`) in the top-left
    group, with its state from `loadAutoZoom`;
  - **change events**: call `change()` after every `render()` that changed data, and on the limit
    selector, `hello`/snapshot, a QTH change, panel toggles and `resize`;
  - **ticks**: a 250 ms interval calls `tick(Date.now())`;
  - **user moves**: `movestart` with `event.originalEvent` → `userMove()`. `movestart` fires for
    every camera change, including rotation and pitch, so this covers them (FR-011);
  - **fit**:
    - measure the panels (`#legend`, `#recent`, `.limit-control`, `.maplibregl-ctrl-top-left`,
      `.maplibregl-ctrl-bottom-left` (the scale bar in flat mode), `.maplibregl-ctrl-bottom-right`,
      the visible `.banner`s) with `getBoundingClientRect()`;
    - compute `paddingFor`;
    - call `map.fitBounds(flatBounds(points) or fallbackFrame, {padding, maxZoom: 12, duration:
      800})`, or the world view when both are null;
  - **popup timer**: wire popup open and close to `popupOpened`/`popupClosed`. While auto-zoom is
    on, add the 30 s inactivity timer, reset on `pointerdown`, `pointermove`, `wheel` and `keydown`
    inside the popup.
- [X] T028 [US2] Manually run quickstart §3 (steps 1–4, 6 and 7, plus a rotate with right-drag
  that must turn auto-zoom off), and step 5 after T049. Record the
  results for SC-002, SC-003 and SC-004 in `checklists/verification.md`.

**Checkpoint**: auto-zoom works in flat mode and on the globe, with the camera's default globe
fitting. US3 refines the globe behaviour.

---

## Phase 5: User Story 3 - Globe or flat, chosen sensibly (Priority: P2)

**Goal**: keep the globe whenever everything fits, centred on the best point; switch to flat only
with auto-zoom on, and switch back; never save automatic switches (FR-015–FR-018).

**Independent Test**: quickstart §4.

### Spike (before the glue that depends on it)

- [X] T029 [US3] Spike in a scratch page, not committed: does MapLibre 5.24.0
  `cameraForBounds`/`fitBounds` with `padding` produce a sensible centre and zoom under
  `projection: globe` for (a) Europe, (b) Europe + JA, (c) a box across the antimeridian? Record
  the outcome in `research.md` R5, under "Spike result". If not, `zoomForCap` (T032) is used for
  the globe.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T030 [P] [US3] Extend `web/autozoom-geo.test.js`:
  - **`boundingCap`**:
    - for Europe-only points the radius is under 30° and the centre is in Europe;
    - IS + JA + W6 fit (radius under 70°, centred in the far north); IS + ZL + PY + JA have a
      radius over 90° (research R5, spike result);
    - for points across the antimeridian (ZL, KH6, 3D2) the centre is in the Pacific, not at
      longitude 0;
    - one point gives radius 0;
    - 2,000 random points complete in under 50 ms (median of 20 runs). This is a loose limit
      against slow machines; the 5 ms plan target is measured manually in T051;
  - **`fitsGlobe(cap, marginDeg)`**: true at 80° with a margin of 5°, false at 88° with a margin
    of 5°;
  - **`zoomForCap(cap, viewportPx)`**: monotonic, so a smaller cap gives a larger zoom, capped at
    12.
- [X] T031 [P] [US3] Extend `web/autozoom.test.js` for `fitted(fitsGlobe, preference)`:
  - **globe preference**:
    - with a fit that doesn't fit the globe → `projection: 'mercator'` and `autoFlat`;
    - when it fits again → `projection: 'globe'` and `autoFlat` cleared;
  - **flat preference**: never `globe` (FR-017);
  - **switching off while `autoFlat`** (FR-017):
    - `toggle` off while `autoFlat` → no projection action, and `autoFlat` is kept;
    - `userMove` while `autoFlat` → off, with no projection action;
    - `toggle` back on while `autoFlat` → the next `fitted(true, 'globe')` returns `globe`;
    - the projection button (a `preferenceChanged` event) clears `autoFlat`;
  - **no save**: no action ever asks to save the projection (FR-018);
  - **scripted run**: a 50-change script (SC-006) checks every switch against the rule.

### Implementation for User Story 3

- [X] T032 [US3] Implement `boundingCap`, `fitsGlobe` and `zoomForCap` in
  `src/web/assets/autozoom-geo.js`. T030 must pass.
- [X] T033 [US3] Implement `fitted` and `autoFlat` handling in `src/web/assets/autozoom.js`.
  T031 must pass.
- [X] T034 [US3] Glue in `src/web/assets/app.js`:
  - **when the preference is the globe**: compute `boundingCap(points)` and `fitsGlobe` with a
    margin derived from the padding;
    - when it fits, apply `globe` and centre on the cap centre, with the zoom from
      `cameraForBounds` or `zoomForCap` according to T029;
    - otherwise apply `mercator` and `fitBounds` the flat bounds;
    - every globe camera uses `maxZoom: 12` too (FR-013);
  - **projection button**: it keeps setting and saving the preference (feature 002), and resets
    `autoFlat`;
  - **ProjectionControl label**: it reflects the preference, not the automatic switch. While
    `autoFlat`, its tooltip reads "Auto-zoom switched to flat; click to make it your preference".
- [X] T052 [US3] Fix found in the owner's browser test: the globe kept a frame it did not show
  (a polar cap zoomed into the Arctic). Replace the `cameraForBounds`/`zoomForCap` globe camera
  with `globeZoomForCap` and `capScreenRadius` in `src/web/assets/autozoom-geo.js`, modelling
  MapLibre's perspective and the 1/cos(latitude) globe scale, with tests in
  `web/autozoom-geo.test.js`. Auto-zoom keeps the globe only when that zoom exists (research R5,
  "Browser finding").
- [X] T053 [US3] Fix found in the owner's browser test: after an automatic switch to flat, the
  projection button still offered "Flat". The button now follows the view on screen
  (`src/web/assets/app.js`). The new `projectionChosen` in `src/web/assets/autozoom.js` switches
  auto-zoom off when the viewer picks a globe that cannot show the frame, so the globe is not
  taken back. Tests are in `web/autozoom.test.js`; FR-018 is updated.
- [X] T054 [US3] Owner request after the browser test: every Globe/Flat switch turns auto-zoom
  off. `projectionChosen()` in `src/web/assets/autozoom.js` now always switches it off, with
  tests in `web/autozoom.test.js`; FR-018 is updated. Page files are also served with
  `Cache-Control: no-cache` (`src/web/mod.rs`, test in `tests/ws_protocol.rs`), so browsers do
  not keep stale modules after a rebuild.
- [X] T035 [US3] Manually run quickstart §4 and record SC-006 in `checklists/verification.md`.

---

## Phase 6: User Story 4 - Last-QSO box (Priority: P2)

**Goal**: the box at the bottom left with the five newest QSOs, clickable rows and a hide button
(FR-019–FR-022a).

**Independent Test**: quickstart §5.

### Tests for User Story 4 (write first, confirm they FAIL)

- [X] T036 [P] [US4] Write `web/recent.test.js`:
  - **`recentQsos(state, 5)`**:
    - after seven QSOs, the five newest by `sort_key`, newest first;
    - a late QSO with an older time doesn't enter;
    - a correction that changes the time re-orders;
    - `applyRemove` moves the next one up;
    - fewer than five gives all of them;
    - none gives `[]`;
    - limit 10 with 20 QSOs → still the five newest of the window;
    - QSOs without a remote position are included;
  - **`rowView(qso)`**:
    - `mode` is `CW`, or `SSB/USB` when there is a submode;
    - `time` is `HH:MMZ` in UTC (HR-1);
    - `markerKey` is `"lat,lon"`, matching `markers()` keys, or `null` with no remote;
    - `call` is returned verbatim (`<b>X</b>` stays a string; escaping is the glue's job, via
      `textContent`).
- [X] T037 [P] [US4] Extend `web/view.test.js`: `loadRecent` and `saveRecent` (`lqm.recent`): the
  default is shown; `hidden` round-trips; an unknown value → shown; a storage that throws → shown.

### Implementation for User Story 4

- [X] T038 [P] [US4] Implement `recentQsos` and `rowView` in `src/web/assets/recent.js`. T036 must
  pass.
- [X] T039 [P] [US4] Implement `loadRecent` and `saveRecent` in `src/web/assets/view.js`. T037 must
  pass.
- [X] T040 [US4] Add `<div id="recent" class="recent" hidden>` to `src/web/assets/index.html`. In
  `src/web/assets/app.css`:
  - bottom-left, stacked above the scale control;
  - the same panel look as the legend;
  - rows with a `markerKey` get `cursor: pointer` and a hover state;
  - an empty-state style;
  - it must never cover the bottom-right attribution.
- [X] T041 [US4] Glue in `src/web/assets/app.js`:
  - **box contents**:
    - `renderRecent()` after every `render()` builds the rows with `textContent` only. That
      includes the render after a snapshot, so the box refills after a reconnection;
    - the empty state is "No QSOs received yet";
  - **hide button**: a `RecentControl` button in the top-left group, using
    `loadRecent`/`saveRecent`; toggling calls `autozoom.change()`;
  - **row click** (FR-022a), when the row has a `markerKey`:
    - open that marker's popup, through the existing popup path;
    - when the marker's projected point is outside the free area from `paddingFor`, `panBy` just
      enough. A page-made move has no `originalEvent`, so auto-zoom stays on;
    - `popupOpened()` pauses auto-zoom;
  - **keyboard**: rows with a marker are `role="button"` and `tabindex="0"`, and Enter or Space
    opens them.
- [X] T042 [US4] Manually run quickstart §5 and record SC-007 in `checklists/verification.md`.

---

## Phase 7: User Story 5 - Distance scale in flat mode (Priority: P3)

**Goal**: a metric scale at the bottom, shown only in flat mode (FR-023, FR-024).

**Independent Test**: quickstart §6.

### Tests for User Story 5 (write first, confirm they FAIL)

- [X] T043 [P] [US5] Extend `web/view.test.js`: `scaleVisible('mercator') === true`,
  `scaleVisible('globe') === false`.

### Implementation for User Story 5

- [X] T044 [US5] Implement `scaleVisible` in `src/web/assets/view.js` (T043 must pass). Then glue
  in `src/web/assets/app.js`:
  - create one `new maplibregl.ScaleControl({unit: 'metric', maxWidth: 120})`;
  - add it at `bottom-left` whenever the effective projection becomes `mercator`, by preference or
    automatic switch, and remove it on `globe`;
  - the `#recent` box stays stacked above it (T040).
- [X] T045 [US5] Manually run quickstart §6 and record SC-008 in `checklists/verification.md`.

---

## Phase 8: Polish & Cross-Cutting Concerns

### JSON error responses (FR-027, `docs/http-api.md` API-3)

- [X] T046 [P] Write tests in `tests/ws_protocol.rs` (contracts/http-endpoints.md). For each case,
  check the status, `Content-Type: application/json`, the `error` code, and a `message` that is a
  non-empty string:
  - `GET /nope` → 404, `{"error":"not_found",…}`;
  - `POST /healthz` → 405 with an `Allow` header, `method_not_allowed`;
  - plain `GET /ws` with no upgrade headers → 4xx, `websocket_upgrade_required`;
  - every body parses as JSON, and no `message` contains the configured cty path.
- [X] T047 Implement `json_error(status, code, message)` in `src/web/mod.rs`, then:
  - `Router::fallback` → 404;
  - `method_not_allowed_fallback` → 405, keeping `Allow`;
  - `ws_handler` takes `Result<WebSocketUpgrade, WebSocketUpgradeRejection>` and maps the
    rejection's `status()` to `websocket_upgrade_required`.

  T046 must pass.

### Tooling, docs and gates

- [X] T048 Add an end-to-end test to `tests/ws_protocol.rs` for the page's inputs. With a
  configured station, send 7 ADIF QSOs over UDP, then connect a page. Assert:
  - `hello` v3 carries the station;
  - the snapshot holds the 7 QSOs in `sort_key` order;
  - each QSO's `station` equals the configured position.

  Run it after T046, because both edit the same file.
- [X] T049 [P] Extend `scripts/send_burst.sh` with optional `delay` (seconds, default 0) and
  `offset` (default 0) arguments, so that calls and QSO times continue from `offset`. Update its
  usage comment (quickstart §3 step 5).
- [X] T050 [P] Documentation:
  - **`README.md`**:
    - a section on the QTH house and auto-zoom: where the QTH comes from, and that the station
      callsign also picks the entity framed with no QSOs;
    - the buttons and their remembered states;
    - the entity-extents regeneration command (quickstart §8), credited to Natural Earth;
  - **`deploy/config.example.toml`**: a comment on `[station]` saying it drives the house;
  - **contract cross-references**: a protocol-3 note pointing to this feature's
    `contracts/websocket-protocol.md`, added to `specs/002-maplibre/contracts/websocket-protocol.md`
    and `specs/001-live-qso-map/contracts/websocket-protocol.md`;
  - **`specs/001-live-qso-map/contracts/http-endpoints.md`**: a note on JSON errors.
- [X] T051 Run the complete gates:
  - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
    `node --test web/` and `python3 -m unittest tools/test_entity_extents.py`;
  - quickstart §7 (the JSON errors);
  - the remaining quickstart steps (§3 step 5 and SC-005);
  - complete `checklists/verification.md` (SC-001 to SC-008).

---

## Dependencies & Execution Order

- **Setup (T001–T002)** comes first, then **Foundational (T003–T006)**, which blocks US1 and US2.
- **US1 (T007–T013)** depends on Foundational. This is the MVP.
- **US2 (T014–T028)** depends on Foundational and on US1 (the QTH is part of the framed set).
  T021 depends on T020 (the extent table), and T023 depends on T022 (the generator).
- **US3 (T029–T035)** depends on US2 (the controller and fit glue).
- **US4 (T036–T042)** depends on US1 (markers and popups):
  - the box and its button work without auto-zoom;
  - the row-click pan and the auto-zoom interplay (T041) need US2.
- **US5 (T043–T045)** depends only on Foundational. It can run in parallel with US2 to US4, but it
  touches `app.js`, so serialise its glue (T044) with the other `app.js` tasks.
- **Polish**:
  - T046 and T047 (JSON errors) are independent of every story and can run any time after Setup;
  - T048–T051 come last.

```text
T001–T002 → T003–T006 → US1 (MVP) → US2 → US3
                    │            └──→ US4 (T041 after US2)
                    └→ US5 (glue serialised with app.js tasks)
T046–T047 (JSON errors): any time after Setup
```

### Within each story

1. Tests come first and must fail.
2. Then the pure modules: `qth.js`, `layers.js`, `autozoom-geo.js`, `autozoom.js`, `recent.js`,
   `view.js`, and the Rust units.
3. Then the wiring: pipeline and web.
4. Then the `app.js` glue, which is serialised because it is one file.
5. Then the manual quickstart check.

---

## Parallel Opportunities

- **Foundational**: T003 and T004.
- **US1**: T007 ∥ T008, then T009 ∥ T010.
- **US2**: T014–T019 are six test files and can all run in parallel; then T024 ∥ T025 ∥ T026, with
  T020 → T021 and T022 → T023 alongside.
- **US3**: T030 ∥ T031, then T032 ∥ T033.
- **US4**: T036 ∥ T037, then T038 ∥ T039.
- **Polish**: T046 ∥ T049 ∥ T050.

### Parallel example: User Story 2

```text
Together:  T014 cty · T015 extents.rs · T016 generator py · T017 autozoom-geo · T018 autozoom · T019 view
Then:      T020 → T021   ∥   T022 → T023   ∥   T024 ∥ T025 ∥ T026
Then:      T027 app.js glue → T028 manual check
```

---

## Implementation Strategy

1. **MVP** (Setup, Foundational, US1): protocol 3 with the station, the house, black lines from the
   QTH, and the house popup.
2. **US2**: auto-zoom with panel-aware padding, entity framing from the generated Natural Earth
   table, popup pauses and manual-move off. This is the main wall-screen value.
3. **US3**: the globe kept when possible, an automatic flat switch and back, preferences never
   overwritten.
4. **US4**: the last-QSO box with clickable rows.
5. **US5**: the metric scale in flat mode.
6. **Polish**: JSON error responses (constitution 1.3.0), burst-script options, documentation,
   gates and the verification note.
