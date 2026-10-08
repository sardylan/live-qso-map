---
description: "Task list for 002-maplibre"
---

# Tasks: MapLibre Map Rendering

**Input**: Design documents from `/specs/002-maplibre/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Every test task comes before the code it covers,
and must be run and seen to fail first. Use `cargo test` for Rust and `node --test web/` for the
frontend (Node via nvm: `~/.nvm/versions/node/v26.11.1/bin/node`). Rendering that only a browser can
show (visual parity, globe, no-WebGL page, fps) is checked manually, following the quickstart steps
named in each task.

**Conventions**:

- **Rust rules**: `docs/rust-guidelines.md` (crate `Error`/`Result`).
- **Domain rules**: `docs/ham-radio.md`. Bands always in frequency order (HR-2).
- **Frontend**: `view.js` and `layers.js` are pure ES modules with no DOM and no MapLibre, like
  `map-state.js`. `app.js` is glue only. QSO data reaches the DOM only through `textContent`
  (FR-004); only operator-configured attribution may be HTML.
- **Frozen modules**: `map-state.js`, `geodesic.js` and `connection.js` and their tests MUST NOT
  change behaviour (FR-007).
- **Commits**: commit after each phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US3 from spec.md

---

## Phase 1: Setup

- [X] T001 Create empty ES modules `src/web/assets/view.js` and `src/web/assets/layers.js`, each with a header comment stating it is pure (no DOM, no MapLibre). Add the routes `GET /view.js` and `GET /layers.js` (`text/javascript`, `include_str!`) to `src/web/mod.rs`. `cargo build` must pass.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the `hello.basemap` message (protocol 2) with the default vector style. Every story
renders through it.

### Tests first (write, run, confirm they FAIL)

- [X] T002 [P] Update the unit tests in `src/config.rs`:
  - the default `Config` has `map.tile_url == None` and `map.tile_attribution == None` (previously
    the OpenStreetMap URL and credit);
  - the default `MapConfig::basemap()` is `BaseMap::Style { url:
    "https://tiles.openfreemap.org/styles/liberty", attribution: <OpenFreeMap credit from research
    R2> }`.

  Adapt `defaults_match_the_contract` to the new defaults.
- [X] T003 [P] Update the unit tests in `src/web/protocol.rs`: `hello` serialises `"protocol": 2` and `"basemap": {"kind":"style","url":…,"attribution":…}`, and has no `tiles` key. Adapt `hello_shape` and the `map_config()` test helper.
- [X] T004 [P] Write `web/view.test.js` for `buildStyle`: `buildStyle({kind:"style", url:"https://s/style.json", attribution:"a"})` returns `"https://s/style.json"`.

### Implementation

- [X] T005 In `src/config.rs`:
  - `MapConfig.tile_url` and `tile_attribution` become `Option<String>` with no default;
  - add `MapConfig.style_url: Option<String>` and `style_attribution: Option<String>`;
  - add `pub enum BaseMap { Style { url, attribution }, Raster { url, attribution } }` with
    `#[derive(Serialize)] #[serde(tag = "kind", rename_all = "lowercase")]`;
  - add `MapConfig::basemap(&self) -> BaseMap`. In this phase it returns the configured style, or
    the OpenFreeMap default with its credit (constants `DEFAULT_STYLE_URL`,
    `DEFAULT_STYLE_ATTRIBUTION`).

  T002 must pass.
- [X] T006 In `src/web/protocol.rs`, set `PROTOCOL_VERSION = 2`, replace `Tiles`/`tiles` with `basemap: BaseMap` (from `map.basemap()`), and update the other callers. T003 must pass.
- [X] T007 [P] Implement `buildStyle(basemap)` for `kind: "style"` in `src/web/assets/view.js`. T004 must pass.
- [X] T008 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `node --test web/` (`Cargo.toml`, `src/`, `web/`). All must pass.

**Checkpoint**: the server sends `hello.basemap`, and the style can be built.

---

## Phase 3: User Story 1 - Same live map, new renderer (Priority: P1) 🎯 MVP

**Goal**: feature-001 behaviour on MapLibre 5.24.0: markers, lines, popups, legend, selector and
reconnect, on the default vector base map with its attribution. Pages without WebGL show a clear
message.

**Independent Test**: `node --test web/` (including the new `layers.test.js` and `view.test.js`) and
`tests/ws_protocol.rs` (new assets served), plus quickstart §2 and §5 in two browsers.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T009 [P] [US1] Write `web/layers.test.js`. Build `markers` with `map-state.js`, as in `web/map-state.test.js`, and assert:
  - **`markerFeatures(markers)`**:
    - one `Point` per marker, with coordinates `[lon, lat]`, not `[lat, lon]`;
    - properties `key`, `color` and `zRank`;
    - features ordered by `zRank`, ascending.
  - **`lineFeatures(markers)`**:
    - one `LineString` per marker that has a station, with none for a null station;
    - coordinates equal to `greatCircle(station, marker, 64)` mapped to `[lon, lat]`;
    - an antimeridian path (`[39.2, 9.2]` to `[21.3, -157.8]`) has no jump above 180° between
      consecutive coordinates;
    - properties `key`, `color` and `zRank`.
  - **Empty input**: an empty `markers` gives empty FeatureCollections.
- [X] T010 [P] [US1] Extend `web/view.test.js` for `hasWebGL`, `isBasemapError` and `shouldRender`.

  `hasWebGL(createCanvas)`:
  - `true` when `getContext('webgl2')` returns an object;
  - `true` when only `'webgl'` does;
  - `false` when both return null;
  - `false` when `getContext` throws.

  `isBasemapError(event, ['qso-lines','qso-markers'])` (edge case "base map unreachable"):
  - `true` for `{ sourceId: 'openmaptiles' }`;
  - `true` for an event with no `sourceId` (style load failure);
  - `false` for `{ sourceId: 'qso-markers' }`.

  `shouldRender(loaded, diff)`:
  - `false` when not loaded;
  - `false` for an empty diff;
  - `true` when loaded and any of `add`, `update` or `remove` is non-empty.
- [X] T011 [P] [US1] Extend `tests/ws_protocol.rs::static_assets_are_served` with `/view.js` and `/layers.js` (`text/javascript`). Assert that `/` contains `maplibre-gl/5.24.0` and both SRI values from research R1, and does not contain `leaflet`.

### Implementation for User Story 1

- [X] T012 [P] [US1] Implement `markerFeatures` and `lineFeatures` in `src/web/assets/layers.js`, importing `greatCircle` from `./geodesic.js`. T009 must pass.
- [X] T013 [P] [US1] Implement `hasWebGL(createCanvas)`, `isBasemapError(event, overlaySources)` and `shouldRender(loaded, diff)` in `src/web/assets/view.js`. T010 must pass.
- [X] T014 [US1] Update `src/web/assets/index.html`:
  - replace the Leaflet CSS and JS with MapLibre 5.24.0 from cdnjs, with the SRI values from
    research R1 (`crossorigin="anonymous"`, `referrerpolicy="no-referrer"`);
  - add a hidden `<div id="no-webgl">` holding the FR-012 message ("This map needs WebGL. Enable
    hardware acceleration in your browser settings, or try another browser.").

  Update `src/web/assets/app.css`: remove the Leaflet-only rules (`.qso-dot`) and keep the legend,
  selector, status and popup rules, adapted to `.maplibregl-popup-content`. T011 must pass.
- [X] T015 [US1] Rewrite the map glue in `src/web/assets/app.js` with MapLibre. Keep the WebSocket, state and selector flow unchanged.
  - **Startup**: if `!hasWebGL(() => document.createElement('canvas'))`, show `#no-webgl`, hide
    `#map`, and do not connect.
  - **On `hello`**: create `new maplibregl.Map({ container: 'map', style: buildStyle(hello.basemap),
    center: [0, 30], zoom: 1.5, attributionControl: false, renderWorldCopies: true })`. Add
    `new maplibregl.AttributionControl({ compact: false, customAttribution:
    hello.basemap.attribution })` (FR-009). Add a `NavigationControl`. Keep the DOM legend in
    `hello.bands` order.
  - **On map `load`**:
    - add a GeoJSON source `qso-lines` and a line layer `qso-lines` (`line-color ['get','color']`,
      `line-width 1.5`, `line-opacity 0.7`);
    - add a GeoJSON source `qso-markers` and a circle layer `qso-markers` (`circle-radius 6`,
      `circle-color ['get','color']`, `circle-stroke-color '#fff'`, `circle-stroke-width 1`,
      `circle-sort-key ['get','zRank']`);
    - then render.
  - **`render()`**: `next = markers(state)`. When `shouldRender(loaded, diffLayers(current, next))`,
    call `setData(markerFeatures(next))` and `setData(lineFeatures(next))`, then set
    `current = next`. Before `load`, keep the state; on `load`, render with
    `current = new Map()`.
  - **Popups**: `map.on('click', 'qso-markers', e => new maplibregl.Popup().setLngLat(...)
    .setDOMContent(popupContent(current.get(e.features[0].properties.key))).addTo(map))`. Keep
    `popupContent` building text-only DOM. Show a pointer cursor on `mouseenter`/`mouseleave`.
  - **Errors**: `map.on('error', e => …)`. Log every error to the console. When
    `isBasemapError(e, ['qso-lines','qso-markers'])`, show the existing `#status` banner with "Base
    map unavailable; QSOs are still shown." (edge case "base map unreachable"). QSO layers keep
    rendering on the empty background.
- [X] T016 [US1] Run quickstart §2 (parity, including the antimeridian path), §5 (no WebGL) and §2b (base map unreachable: block `tiles.openfreemap.org` in the browser's DevTools network request blocking, reload, and check that the banner shows while markers and lines still render) in two browsers. Record the results in `specs/002-maplibre/checklists/verification.md`.

**Checkpoint**: User Story 1 works on its own. This is the MVP.

---

## Phase 4: User Story 2 - Choose the base map (Priority: P2)

**Goal**: the operator can choose a vector style URL or raster tiles, with their attribution;
configuring both is rejected.

**Independent Test**: unit tests from T017–T019, plus quickstart §3.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T017 [P] [US2] Extend the unit tests in `src/config.rs`:
  - `--style-url`/`LQM_STYLE_URL`/TOML `map.style_url` and the matching `*_style_attribution`
    follow the CLI > env > file precedence;
  - only `tile_url` set (and `tile_attribution`) gives `BaseMap::Raster`;
  - only `style_url` set gives `BaseMap::Style` with `style_attribution`, or `""` when it is unset;
    the OpenFreeMap credit applies only to the default style;
  - `style_url` together with `tile_url`, from any layers, gives `Error::Config { setting:
    "map.style_url", .. }` with a message saying they are mutually exclusive;
  - `tile_url` with a missing or blank `tile_attribution` gives `Error::Config { setting:
    "map.tile_attribution", .. }` (FR-009).
- [X] T018 [P] [US2] Extend `web/view.test.js`: `buildStyle({kind:"raster", url:"https://t/{z}/{x}/{y}.png", attribution:"a"})` returns `{version: 8, sources: {basemap: {type: "raster", tiles: ["https://t/{z}/{x}/{y}.png"], tileSize: 256}}, layers: [{id: "basemap", type: "raster", source: "basemap"}]}`.
- [X] T019 [P] [US2] Extend the unit tests in `src/web/protocol.rs`: a raster `MapConfig` serialises `basemap` as `{"kind":"raster","url":…,"attribution":…}`.

### Implementation for User Story 2

- [X] T020 [US2] In `src/config.rs`:
  - add the `Cli` flags `--style-url` and `--style-attribution`, the env vars `LQM_STYLE_URL` and
    `LQM_STYLE_ATTRIBUTION`, and the `FileMap` keys `style_url` and `style_attribution`;
  - add the conflict validation and the raster-attribution validation;
  - make `MapConfig::basemap()` return `Raster` when `tile_url` is set.

  T017 and T019 must pass.
- [X] T021 [P] [US2] Implement the raster branch of `buildStyle` in `src/web/assets/view.js`. T018 must pass.
- [X] T022 [US2] Run quickstart §3 with the real binary: the default style, raster tiles, and the conflict error.

**Checkpoint**: User Stories 1 and 2 work on their own.

---

## Phase 5: User Story 3 - Globe view (Priority: P3)

**Goal**: pages open as a globe, and a per-viewer toggle switches to flat, remembered in
`localStorage`.

**Independent Test**: unit tests from T023, plus quickstart §4.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T023 [P] [US3] Extend `web/view.test.js`:
  - `loadProjection(storage)` gives `"globe"` for empty storage, `null` storage, an unknown value,
    or a throwing storage; it gives `"mercator"` when stored;
  - `saveProjection(storage, p)` writes the key `lqm.projection` and swallows errors;
  - `nextProjection('globe')` is `'mercator'` and the reverse.

### Implementation for User Story 3

- [X] T024 [US3] Implement `loadProjection`, `saveProjection` and `nextProjection` in `src/web/assets/view.js`. T023 must pass.
- [X] T025 [US3] Wire the projection in `src/web/assets/app.js`:
  - on `load`, call `map.setProjection({ type: loadProjection(localStore()) })`, where `localStore()`
    returns `window.localStorage` inside a try/catch;
  - add a small custom control (top-left, under the navigation control): a button labelled
    "Flat"/"Globe" for the other projection. A click sets `nextProjection`, calls `setProjection`
    and `saveProjection`, and updates the label.

  Add the control style to `src/web/assets/app.css`.
- [X] T026 [US3] Run quickstart §4 in two browsers. Record the result in `specs/002-maplibre/checklists/verification.md`.

**Checkpoint**: all user stories work on their own.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T027 [P] Document the base map:
  - in `deploy/config.example.toml`, add commented `style_url`/`style_attribution`, mark `tile_*`
    as the raster alternative, and explain the conflict;
  - in `README.md`, replace Leaflet with MapLibre, describe the globe toggle, list the base-map
    settings and the WebGL requirement;
  - in `specs/001-live-qso-map/contracts/configuration.md` and `websocket-protocol.md`, add a note
    pointing to the 002 deltas (protocol 2, `basemap`).
- [X] T028 [P] Extend `tests/ws_protocol.rs::every_page_receives_hello_snapshot_and_live_upserts` to assert that `hello.protocol == 2` and that `hello.basemap.kind == "style"` with the default configuration.
- [X] T029 **Accepted as observed by the owner (2026-10-09): no problems seen; no fps figure recorded.** Run quickstart §6 (SC-002: at least 30 fps with 2,000 QSOs in globe and flat) using `scripts/send_burst.sh 2000` and the browser Performance panel. Record the measured fps, plus the machine (CPU, GPU, OS) and browser version, in `specs/002-maplibre/checklists/verification.md`.
- [X] T030 Run the complete quality gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`. Grep `src/` and `README.md` for leftover `leaflet`/`L.` references. Finalise `specs/002-maplibre/checklists/verification.md` with SC-001 to SC-003. SC-003 (live QSO within 2 s) is evidenced by the unchanged test `tests/ws_protocol.rs::every_page_receives_hello_snapshot_and_live_upserts`.

---

## Dependencies & Execution Order

- **Setup (T001)**, then **Foundational (T002–T008)**, which blocks every story.
- **US1 (T009–T016)** depends on Foundational. This is the MVP: the page renders only once the
  MapLibre glue exists.
- **US2 (T017–T022)** depends on Foundational. Its server and `view.js` parts are independent of
  US1; the manual check T022 needs US1's page.
- **US3 (T023–T026)** depends on US1, because it uses the map created by the T015 glue.
- **Polish (T027–T030)** comes after the stories. T029 needs US1 and US3.

```text
T001 → T002–T008 → US1 (MVP) → US3 ─┐
                  └→ US2 ───────────┴→ Polish
```

### Within each story

Tests come first and must fail. Then the pure modules (`view.js`, `layers.js`), then config and
protocol, then the glue (`app.js`, `index.html`), then the manual quickstart check.

---

## Parallel Opportunities

- **Foundational**: T002, T003 and T004 (different files). T007 runs alongside T005 and T006.
- **US1**: T009, T010 and T011, then T012 and T013.
- **US2**: T017, T018 and T019. T021 runs alongside T020.
- **Across stories**: US2's server work (T017–T021) can run alongside US1.
- **Polish**: T027 and T028.

### Parallel example: User Story 1

```text
Together:  T009 layers tests · T010 webgl tests · T011 assets/SRI test
Together:  T012 layers.js · T013 hasWebGL
Then:      T014 index.html/app.css → T015 app.js glue → T016 manual check
```

---

## Implementation Strategy

1. **MVP**: Setup, Foundational and US1. The map renders on MapLibre with the default vector base
   map, full feature-001 parity, and the no-WebGL message.
2. **US2** adds the operator's choice of base map (vector or raster).
3. **US3** adds the globe default and the flat toggle.
4. **Polish** adds documentation, the 30 fps check, the gates and the verification note.
