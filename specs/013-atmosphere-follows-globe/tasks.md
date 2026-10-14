---
description: "Task list for 013-atmosphere-follows-globe"
---

# Tasks: Atmosphere that moves with the globe, and the grayline

**Input**: Design documents from `/specs/013-atmosphere-follows-globe/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R13), data-model.md, contracts/ (page-modules.md, browser-test.md), quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task first, run it, and confirm
it fails.
- **Page logic:** pure ES modules, tested with `node --test 'web/*.test.js'`.
- **Rust:** `cargo test`. Feature 012's test fetches every page file found in
  `src/web/assets/`, so adding `grayline.js` fails it until the file is served (T024).
- **The browser run:** `node tests/browser/run.mjs` is the release-time test of the wiring in
  `app.js`. Steps 10 and 11 are written before that wiring and are seen failing against the
  unfixed page.

**Conventions:**
- **Runner:** Node built-ins only, as in feature 012. No `package.json` and no npm packages in
  `tests/browser/`.
- **MapLibre's sun maths:** the copies of `getSunPos` and `sphericalToCartesian` used in tests
  are marked `// Copied from maplibre-gl 5.24.0 dist/maplibre-gl-dev.js, getSunPos/sphericalToCartesian`.
  Upgrading MapLibre means checking them again (research R1).
- **Units and order:** coordinates are `[lat, lon]` in degrees, as in `geodesic.js`. Times are
  UTC (HR-1).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US3)

---

## Phase 1: Setup

- [X] T001 Run `node --test 'web/*.test.js'`, `cargo test` and `cargo clippy --all-targets -- -D warnings` on `feature/013-atmosphere-follows-globe`, and record that all pass before any change (baseline). Check that `node tests/browser/run.mjs` passes steps 01–09 on this machine, or note why it can't run here.

---

## Phase 2: Foundational (one Sun for everything)

The glow (US1, US2), the grayline and the Sun symbol (US3) all need the subsolar point.

- [X] T002 [P] Write failing tests for `subsolarPoint(date)` in `web/sun.test.js` (contracts/page-modules.md, `sun.js`). Cover:
  - **Equinox:** on 2026-03-20 at 12:00 UTC, `lat` is within 0.5° of 0, and `lon` is within 2° of 0.
  - **June solstice:** on 2026-06-21, `lat` is within 0.1° of +23.44.
  - **December solstice:** on 2026-12-21, `lat` is within 0.1° of −23.44.
  - **Westward drift:** `lon` moves west by 15° ± 0.1° per hour.
  - **Range:** at 00:00 UTC, `lon` is within the range −180 (included) to 180 (excluded), near ±180.
  - **Consistency:** `solarElevation(date, p.lat, p.lon) ≥ 89.5` for the subsolar point `p`, at 12 dates spread over 2026.
- [X] T003 Implement `subsolarPoint(date)` in `src/web/assets/sun.js`:
  - move the declination and equation-of-time code out of `solarElevation` into a non-exported helper, and use it in both functions;
  - `lon = −(minutesUtc + equationOfTime − 720) / 4`, wrapped to the range −180 (included) to 180 (excluded) (research R3).

  T002 and the feature 011 `solarElevation` tests must pass, with no other change.

**Checkpoint**: one Sun position, unit-tested. The user stories can start.

---

## Phase 3: User Story 1 - The atmosphere turns with the Earth when moved by hand (Priority: P1) 🎯 MVP

**Goal**: the globe's glow is attached to the Earth, with its bright side over the subsolar point,
under every hand move (FR-001 to FR-005, FR-007).
**Independent Test**: quickstart items 1–3. Browser step 10, items 1–6.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T004 [P] [US1] Write failing tests for `lightFor(subsolar)` in `web/view.test.js` (contracts/page-modules.md, `view.js`):
  - add the marked copies of MapLibre's `getSunPos` (anchor `map` and anchor `viewport`) and `sphericalToCartesian`;
  - **Shape:** the result is `{ anchor: 'viewport', position: [1.15, az, polar] }`. Revised during implementation (research R1–R2): `lightFor(subsolar, camera)`, lit from behind the globe, rim towards the Sun, no haze (FR-019).
  - **Facing the Sun:** with the view centred on the subsolar point (bearing 0, pitch 0), the replayed sun direction is within 1e-9 of `(0, 0, 1)`. Check this for (0, 0), (−8.5, −30), (23.44, 120), (−23.44, −170) and (10, 179).
  - **West limb:** with the view centred 90° east of the subsolar point, `x < −0.9`.
  - **Regression for A1 and A2:** with the default light (`viewport`, `[1.15, 210, 30]`), the replayed direction is the same for the view centres (0, 0) and (0, 180).
- [X] T005 [P] [US1] Write failing tests for the runner helpers in `web/browser-lib.test.js`. They are pure, and live in `tests/browser/lib.mjs`:
  - **`luminance([r, g, b, a])`:** Rec. 709 luma, from 0 to 255.
  - **`globeRadiusPx(view)`:** `512 · 2^zoom / (2π cos lat)`. `view` is `#map`'s `data-view`, parsed to `{ lat, lon, zoom }`.
  - **`limbSamples(view, width, height)`:** two pixel boxes at the globe centre's height, 2–6 % of the radius outside the limb, on the left and on the right. Neither box runs outside the image.
  - **`sunSide(view, subsolar)`:** the sign of the replayed sun direction's `x`, plus its value. It reuses `lightFor` and the copied `getSunPos`.
  - **`testInstant(house, year)`:** the UTC instant on the June solstice of `year` when the subsolar longitude is the house longitude − 90° (contracts/browser-test.md, test clock). `subsolarPoint(testInstant(h, y)).lon` is within 0.1° of `h.lon − 90`, wrapped, for houses at lon −170, 0, 9.12 and 179.

### Implementation for User Story 1

- [X] T006 [US1] Implement `lightFor(subsolar)` in `src/web/assets/view.js`, next to `skyFor`, with a doc comment pointing to research R2. T004 must pass.
- [X] T007 [US1] Implement `luminance`, `globeRadiusPx`, `limbSamples`, `sunSide` and `testInstant` in `tests/browser/lib.mjs`, importing `lightFor` from `../../src/web/assets/view.js`. T005 must pass.
- [X] T008 [US1] Add `drag(from, to, steps = 20)` to the page object in `tests/browser/cdp.mjs`. It sends `Input.dispatchMouseEvent`: `mousePressed`, then `steps` × `mouseMoved`, then `mouseReleased`, with the left button. Also add `pixels(boxes)`:
  1. take `Page.captureScreenshot` as a PNG;
  2. evaluate in the page: load it as a `data:` URL `Image`, draw it on an `OffscreenCanvas`, and return the average RGBA of each box (research R5).

  Also add `setClock(isoInstant)`. It installs, with `Page.addScriptToEvaluateOnNewDocument`, a `Date` subclass and a `Date.now` shifted by a constant offset, so "now" starts at `isoInstant` and keeps running (contracts/browser-test.md, test clock).
- [X] T009 [US1] Write step 10 `atmosphere` in `tests/browser/run.mjs`, items 1–6 of contracts/browser-test.md:
  - **Test clock:** first `setClock(testInstant(house, year))` and reload, with the Sun 90° west of the house. Use the same instant, plus the elapsed time, for every expected side computed in Node.
  - **Set up:** Dark theme, globe, auto-zoom off, zoomed out.
  - **Drag:** drag until `|sunSide.x| ≥ 0.5`.
  - **Check:** the brighter limb box matches the sign, with a 10 % margin.
  - **Turn:** drag about 180°, and check again.
  - **Screenshots:** `10-atmosphere-drag`, `10-atmosphere-turned`.
  - **Failures:** messages in the contract's format.

  Add the step to the steps list and to `tests/browser/README.md`. Run it against the current page, and confirm it FAILS: one of the two checks sees the bright limb on the wrong side.
- [X] T010 [US1] In `src/web/assets/app.js`:
  - import `subsolarPoint` and `lightFor`;
  - add `applyLight()`, which does `if (!loaded) return; map.setLight(lightFor(subsolarPoint(new Date())));`;
  - call it from `onStyleLoad()`, after `applySky()`, and from the 60 s theme tick (`setInterval(updateTheme, THEME_TICK_MS)` becomes a function that calls both).

  Revised: `applyLight()` also runs on `move`, with the camera (research R2, revised). No other handler. Run step 10 items 1–6: they must PASS.
- [X] T011 [US1] Check quickstart items 1–3 by eye in Chrome:
  - **Drag, rotate and tilt:** the glow turns with the Earth.
  - **Zoom:** zoom in until the limb leaves the window, then out; the glow comes back in place.
  - **Edge cases:** a theme switch keeps the bright side in place, and so does toggling the atmosphere and resizing the window.

  Note the result in this task's line.
  **Result:** checked in the browser by the owner on 2026-10-14: OK. The night colours were kept as planned.

**Checkpoint**: A1 is fixed and unit-tested, with a release check.

---

## Phase 4: User Story 2 - The atmosphere turns with the Earth during auto-zoom (Priority: P1)

**Goal**: the same glow behaviour during and after auto-zoom moves, and after projection switches
(US2 1–3, FR-002, FR-003).
**Independent Test**: quickstart item 4. Browser step 10, item 7.

### Tests for User Story 2 (write first)

- [X] T012 [US2] Extend step 10 in `tests/browser/run.mjs` with item 7 of contracts/browser-test.md:
  1. drag the globe so its centre is about 40° east of the house;
  2. switch auto-zoom on, and send a QSO with `sendQso` from a locator about 10° from the house;
  3. during the flight, every 200 ms while `data-view` is still changing, take a screenshot and check that the glow is on the west (left) limb. At least 3 mid-flight frames (FR-003, SC-003); screenshots `10-atmosphere-flight-1..n`;
  4. wait for a stable view, check the glow side as in item 5, and take the screenshot `10-atmosphere-autozoom`.

  Then switch to Flat and back to Globe, and check the glow side again (US2 3). Confirm it fails on a checkout of `main`, where the page has no `applyLight`: either revert T010 in a scratch worktree, or run the step against a 3.0.1 build.

### Implementation for User Story 2

- [X] T013 [US2] With T010 in place, run step 10 in full (items 1–7): it must PASS. If the auto-zoom or projection-switch check fails, fix the cause in `src/web/assets/app.js`. For example, if a style or projection change drops the light, call `applyLight()` where `shown` changes, next to `applySky()` (line ~503).
- [X] T014 [US2] Check quickstart item 4 by eye: while auto-zoom flies the globe to a far QSO, the glow moves with the Earth at every frame. Note the result in this task's line.
  **Result:** checked in the browser by the owner on 2026-10-14: OK. The night colours were kept as planned.

**Checkpoint**: A2 is fixed. The globe part of the feature is complete and releasable alone.

---

## Phase 5: User Story 3 - The grayline on the flat map and on the globe (Priority: P2)

**Goal**: a smooth night fade (0° to −12°) on both views, under labels, attached to the map,
following the Sun. A Grayline choice (on by default) and an optional Sun symbol (off by default),
both saved (FR-009 to FR-018).
**Independent Test**: quickstart items 5–11. Browser step 11.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T015 [P] [US3] Write failing tests for `destination(point, bearingDeg, distanceDeg)` in `web/geodesic.test.js`:
  - **Zero distance:** gives the start point.
  - **Due north:** from (0, 0), 90° gives (90, any lon).
  - **Due east:** from (0, 0), 90° gives (0, 90).
  - **Antipode:** from (45, 10), 180° gives (−45, −170).
  - **Round trip:** a point 30° away has a great-circle distance of 30° ± 1e-9, using the existing vector helpers through `greatCircle`.
- [X] T016 [P] [US3] Write failing tests in `web/grayline.test.js` for `graylineCaps`, `capOpacity` and `graylineBeforeId` (contracts/page-modules.md, `grayline.js`). **Revised during implementation:** the caps were too slow on the globe (research R7, revised). The tests now cover `shade`, `elevationAt`, `tileLatLon`, `tilePixels`, `graylineTiles`, `parseGraylineUrl`, `rgbOf` and `graylineBeforeId`. Original list:
  - **Shape:** 25 polygon features, `properties.h` from 0 to −12 in 0.5° steps.
  - **Rings:** each ring is closed, and no two consecutive longitudes differ by more than 180°.
  - **Poles:** for the June solstice subsolar point, the caps with radius > 90° − 23.44° include points at latitude −90 (the antisolar side is south).
  - **Sampled check:** on a 5° lat/lon grid, at 3 dates × 4 times (contract), each point lies inside cap `k` exactly when `solarElevation ≤ −0.5 k`. Use a spherical point-in-cap test (angular distance from the antisolar point ≤ radius), and also a planar point-in-polygon test on the unwrapped ring, with points shifted by ±360°. Leave out points within 0.1° of the boundary.
  - **`capOpacity`:** `1 − (1 − capOpacity(x))^25 = x` for x = 0.2, 0.4 and 0.7.
  - **`graylineBeforeId`:** with fixtures for `liberty`, `fiord` and `dark` (arrays of `{ id, type, 'source-layer' }` copied from research R9, the layers around the first boundary), it gives `boundary_3`, `boundary_state` and `boundary_state`. With no boundary layer it gives the first symbol layer. With neither it gives `undefined`.
- [X] T017 [P] [US3] Write failing tests in `web/view.test.js` for `loadGrayline`/`saveGrayline` and `loadSunMarker`/`saveSunMarker`:
  - **Defaults:** the grayline is on and the Sun symbol is off.
  - **Round trip:** a saved choice reads back the same.
  - **Unknown value:** reads as the default.
  - **Throwing storage:** reads as the default and doesn't throw.

  Copy the `loadAtmosphere` tests' pattern.
- [X] T018 [P] [US3] Write failing tests in `web/themes.test.js`: every palette in `PALETTES` has `night` as a 7-character hex colour, and `nightOpacity` between 0.2 and 0.7.
- [X] T019 [P] [US3] Write failing tests in `web/browser-lib.test.js` for:
  - **`toScreenMercator(view, { width, height }, [lat, lon])`:** the view centre maps to the screen centre, and a point 1 world-width east maps to the same x plus `512 · 2^zoom`;
  - **`toScreenGlobe(view, size, [lat, lon])`:** the centre maps to the screen centre, a point 90° away maps to the limb (`globeRadiusPx` from the centre), and a point more than 90° away gives `null`;
  - **`nightAndDayPoints(subsolar, view, size, project)`:** two on-screen points where the Sun is below −12° and above +10°, or `null` when none fits (contracts/browser-test.md, step 11 item 2).
- [X] T020 [US3] Write step 11 `grayline` in `tests/browser/run.mjs`, following contracts/browser-test.md items 1–9 (on the test clock installed in step 10):
  - **Flat map:** on/off screenshot pairs through `#grayline`, at the night and day points; again after a 300 px drag.
  - **Globe:** the view centre in deep night, then in full day.
  - **Sun symbol:** a box at the subsolar point, with `#sun-marker` on and off.
  - **Smoothness:** the `requestAnimationFrame` rate during a 3 s scripted drag on the globe, with nothing on, each part alone and all on, recorded in the report only (SC-004, owner's decision: software rendering in the test browser).
  - **Reload:** the choices are kept, and so are `lqm.grayline` and `lqm.sun`.
  - **Screenshots:** `11-grayline-flat`, `11-grayline-panned`, `11-grayline-globe-night`, `11-grayline-sun`.

  Add it to `tests/browser/README.md`. Run it, and confirm it FAILS: `#grayline` doesn't exist yet.

### Implementation for User Story 3

- [X] T021 [P] [US3] Implement `destination` in `src/web/assets/geodesic.js`, with spherical formulas in degrees and `[lat, lon]` order. T015 must pass.
- [X] T022 [US3] Create `src/web/assets/grayline.js` with `CAP_STEP`, `CAP_COUNT`, `graylineCaps`, `capOpacity` and `graylineBeforeId`. **Revised:** built as below and then replaced by raster tiles drawn per pixel (research R7, revised). Original steps:
  1. Start from the antisolar point `(−lat, lon + 180)`, wrapped.
  2. For each `k`, build a ring of 361 `destination` points at radius `90 − 0.5 k`, for bearings 0–360 in 1° steps.
  3. Unwrap the longitudes so consecutive points differ by at most 180°.
  4. If `|last lon − first lon| ≥ 359°`, the ring goes round a pole. Append `[lastLon, pole]` and `[firstLon, pole]`, with `pole = −90` if the antisolar latitude is negative and `+90` otherwise, then close the ring.
  5. Emit GeoJSON `[lon, lat]` coordinates (research R7, R8).

  T016 must pass. (T022 depends on T021.)
- [X] T023 [P] [US3] Implement `loadGrayline`/`saveGrayline` (key `lqm.grayline`; anything but `off` is on) and `loadSunMarker`/`saveSunMarker` (key `lqm.sun`; only `on` is on) in `src/web/assets/view.js`, next to the atmosphere pair. T017 must pass.
- [X] T024 [US3] Serve the new module in `src/web/mod.rs`: add `const GRAYLINE_JS: &str = include_str!("assets/grayline.js");` and a `/grayline.js` route with `text/javascript`, like `/sun.js`. Run `cargo test` before this task, and confirm that feature 012's every-page-file test fails on `grayline.js`; it must pass after. (Depends on T022.)
- [X] T025 [P] [US3] Add `night` and `nightOpacity` to each palette in `src/web/assets/themes.js`, and extend the palette doc comment. Values: light `#0b1a3c`/0.40, dim `#000814`/0.45, dark `#000000`/0.55 (research R12). T018 must pass.
- [X] T026 [P] [US3] Implement `toScreenMercator`, `toScreenGlobe` and `nightAndDayPoints` in `tests/browser/lib.mjs`, importing `solarElevation` from `../../src/web/assets/sun.js`. T019 must pass.
- [X] T027 [US3] Add two checkboxes to the Map group in `src/web/assets/index.html`, right after the atmosphere label:
  - `<label><input type="checkbox" id="grayline"> Grayline (day and night)</label>`
  - `<label><input type="checkbox" id="sun-marker"> Sun position</label>`

  Keep the panel's existing focus order and styles.
- [X] T028 [US3] In `src/web/assets/app.js`, `addOverlays()` (contracts/page-modules.md, app wiring):
  - **Grayline:** add source `grayline` (an empty FeatureCollection) and a `fill` layer `grayline`, with `fill-antialias: false`, before `graylineBeforeId(map.getStyle().layers)`.
  - **Sun icon:** draw it once on a 32×32 canvas: a `#ffcc00` disc of radius 7, 8 rays to radius 14, and a 1.5 px `#5a3d00` outline. Add it with `map.addImage('sun-icon', imageData)` if `!map.hasImage('sun-icon')`.
  - **Sun layer:** add source `sun` (an empty Point collection) and a `symbol` layer `sun`, with `icon-image: 'sun-icon'`, `icon-allow-overlap: true` and `icon-ignore-placement: true`, inserted before `qso-lines`. It goes after the `theme-dim` layer, so the dim layer doesn't darken it: add it after `applyOverlayTheme()` in `onStyleLoad()`, or move it there.

  Bind no events to either layer.
- [X] T029 [US3] In `src/web/assets/app.js`, replace `applyLight()` with `applySun()` (contracts/page-modules.md, app wiring). **Revised:** the grayline is a `raster` source fed by the `grayline://` protocol. `applySun()` keeps `applyLight()` for moves, and calls `setTiles` when the theme or minute changes. Original steps:
  - compute `subsolarPoint(new Date())` once;
  - call `setLight(lightFor(p))`;
  - `getSource('grayline').setData(graylineCaps(p))`;
  - set `fill-color` to `PALETTES[currentTheme].night` and `fill-opacity` to `capOpacity(...nightOpacity)`;
  - `getSource('sun').setData(point p)`;
  - set the layers' `visibility` from `grayline` and `sunMarker`.

  Read `grayline` and `sunMarker` once at start-up with `loadGrayline` and `loadSunMarker` from `localStore()`. Call `applySun()` from `onStyleLoad()`, the 60 s tick and the theme change (colours). In `setupSelector()`, wire `#grayline` and `#sun-marker` like `#atmosphere`: set `checked`, then on `change` save and call `applySun()`. Run step 11: it must PASS. Run step 10 again: it must still PASS.
- [X] T030 [US3] Check quickstart items 5–11 by eye in Light, Dim and Dark, and tune `night`/`nightOpacity` in `src/web/assets/themes.js` if needed, keeping T018's range:
  - **Fade:** smooth, no hard edge.
  - **Date line:** no seam, and two world copies are both shaded.
  - **Polar night:** near a solstice, shaded up to the map edge (use DevTools to override `Date` if needed).
  - **Night side:** labels, borders and QSO markers are readable, and a marker there opens its popup.
  - **Globe:** shaded under the glow, with the bright side over day.
  - **Sun symbol:** under QSO markers, not clickable, and hidden on the far side.
  - **Time:** an hour later, everything has moved about 15° west with no jump.

  Note the chosen values and the result in this task's line.
  **Result:** checked in the browser by the owner on 2026-10-14: OK. The night colours were kept as planned.

**Checkpoint**: all three stories work, each checked by unit tests, a browser step and by eye.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T031 [P] Add §2.11 "Grayline" to `docs/ham-radio.md`. It is the twilight zone along the day/night terminator (civil twilight: the Sun 0°–6° below the horizon), where low-band paths often open between stations that are both near sunrise or sunset. The map shades the fade to −12°.
- [X] T032 [P] Update `README.md`:
  - **Map group list** (line ~92): add Grayline and Sun position.
  - **Feature description** (near line ~108): describe the grayline, the Sun symbol, and the glow facing the real Sun.
  - **Saved settings sentence** (line ~152): add both choices.
- [X] T033 [P] Add `CHANGELOG.md` entries under Unreleased:
  - **Fixed:** the globe's atmosphere turns with the Earth on hand and auto-zoom moves, and faces the real Sun.
  - **Added:** the grayline on the flat map and the globe (Grayline choice, on by default), and the Sun position symbol (off by default).

  Check that `cargo test` (the changelog test in `tests/changelog.rs`) still passes.
- [X] T034 Record the decisions in `docs/tradeoffs.md`, using the in-file entry template, if the owner has confirmed them:
  - **TR-019:** a map-anchored MapLibre light over the subsolar point;
  - **TR-020:** the grayline as raster tiles drawn in the page (25 stacked GeoJSON caps were tried first and dropped: 3× slower on the globe);
  - **TR-021:** the atmosphere lit from behind the globe, recomputed on every move (an Earth-fixed light hazed the day side; owner's rule FR-019).

  Otherwise leave them out, and note that in this line.
  **Result:** owner confirmed. TR-020 and TR-021 accepted, TR-019 archived as superseded by TR-021; LL-005 added (and the missing TR-018 and LL-004 rows in the two indexes).
- [X] T035 Check that `specs/013-atmosphere-follows-globe/plan.md` matches what was built: the one `/grayline.js` route in `src/web/mod.rs`, the layer ids, and the palette values tuned in T030. Update it where it doesn't.
- [X] T036 Run the quality gates and record the results here:
  - `cargo fmt --check`;
  - `cargo clippy --all-targets -- -D warnings`;
  - `cargo test`;
  - `node --test 'web/*.test.js'`;
  - `python3 -m unittest discover -s tools -p 'test_*.py'`;
  - `node tests/browser/run.mjs` (all 11 steps).
  **Result (2026-10-14):** fmt clean; clippy with no warnings; `cargo test` 389 passed; `node --test` 220 passed; tool tests OK; browser run 11/11 passed in 107 s of steps (feature 012's 2-minute budget).
- [X] T037 Review: every new test was seen failing before its implementation (T002, T004, T005, T009, T012, T015–T020, T024). No new dependency. The only new map handler is `move` → `applyLight` (FR-007, FR-016; smoothness measured by step 11). No click handler on `sun` or `grayline`.
  **Result:** seen failing first: T002, T004, T005 (and the apparent-radius and padded-centre helper tests added later), T009 (on the unfixed page, the bright limb stayed put after a half turn), T015–T020, T024 (served-files test on `grayline.js`). T012's flight check was not run against the old page; it fails there by construction (no `data-light`). No new dependency. The only new map handler is `move` → `applyLight`. No click handler on `sun` or `grayline`.

---

## Dependencies & Execution Order

- **Setup:** T001 comes first.
- **Foundational:** T002 → T003. These block every user story.
- **US1:** T004 and T005 run in parallel, then T006 → T007. T008 runs alongside them. T009 needs
  T007 and T008. Then T010 → T011.
- **US2:** T012 needs T009 and T010. Then T013 → T014.
- **US3:** needs T003. It reuses `lightFor` (T006) and the step-10 helpers (T007, T008).
  - **Tests:** T015–T019 run in parallel, then T020.
  - **Implementation:** T021 → T022 → T024. T023, T025 and T026 run in parallel. Then T027 →
    T028 → T029 → T030.
- **Polish:** T031–T033 run in parallel, then T034 → T035 → T036 → T037.

The stories in order: US1 → US2 (it reuses step 10 and `applyLight`) → US3 (it replaces
`applyLight` with `applySun`).

## Parallel Opportunities

- **US1 tests:** T004 (`web/view.test.js`) and T005 (`web/browser-lib.test.js`). T008
  (`tests/browser/cdp.mjs`) runs alongside them.
- **US3 tests:** T015 (`web/geodesic.test.js`), T016 (`web/grayline.test.js`), T017
  (`web/view.test.js`), T018 (`web/themes.test.js`) and T019 (`web/browser-lib.test.js`), five
  different files.
- **US3 implementation:** T021 (`geodesic.js`), T023 (`view.js`), T025 (`themes.js`) and T026
  (`tests/browser/lib.mjs`).
- **Polish:** T031 (`docs/ham-radio.md`), T032 (`README.md`) and T033 (`CHANGELOG.md`).

### Parallel example: User Story 3 tests

```text
T015 web/geodesic.test.js   destination
T016 web/grayline.test.js   shade, elevationAt, tilePixels, graylineTiles, graylineBeforeId
T017 web/view.test.js       load/save Grayline and Sun position
T018 web/themes.test.js     night, nightOpacity
T019 web/browser-lib.test.js toScreenMercator, toScreenGlobe, nightAndDayPoints
```

## Implementation Strategy

1. **MVP (US1, T001–T011):** the glow turns with the Earth on hand moves and faces the Sun. It
   is releasable alone as a patch fix.
2. **US2 (T012–T014):** proves the same for auto-zoom and projection switches. It is mostly a
   test, since the same light fixes it.
3. **US3 (T015–T030):** the grayline and the Sun symbol, a new feature. Ship it as a minor release
   with US1 and US2.
4. **Polish (T031–T037):** glossary, README, changelog, tradeoffs and the quality gates.
