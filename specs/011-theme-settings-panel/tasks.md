---
description: "Task list for 011-theme-settings-panel"
---

# Tasks: Automatic Theme, Atmosphere, Settings Panel and Legend Selection

**Input**: Design documents from `/specs/011-theme-settings-panel/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (websocket-protocol.md, page-ui.md, configuration.md), quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task first, run it, and confirm
it fails.
- **Page logic:** pure ES modules tested with `node --test 'web/*.test.js'`. Tests import from
  `../src/web/assets/<module>.js`, as `web/view.test.js` does. No DOM and no npm packages.
- **Rust:** `cargo test`. New integration tests return `Result<(), Box<dyn std::error::Error>>`,
  use `?`, and have no inline lint suppressions (LL-001).
- **DOM and map wiring** in `app.js`: checked by hand with `quickstart.md`, as in features
  002–007.

**Conventions:**
- **Storage:** saved settings use the existing helper pattern (`load…(storage)` /
  `save…(storage, value)` in `src/web/assets/view.js` and `connection.js`). Any error or unknown
  value falls back to the default (constitution IV).
- **Band order:** band lists are always in frequency order (HR-2).
- **Phase order:** the settings panel (User Story 2) is built before the theme (User Story 1).
  The theme's choice control lives in the panel's Appearance group. Both are P1.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US5)

---

## Phase 1: Setup

- [X] T001 Replace every colour in `src/web/assets/app.css` with CSS custom properties, defined once on `:root` with today's (light) values: `--bg`, `--panel`, `--text`, `--muted`, `--border`, `--accent`, `--banner`, `--banner-low`, `--highlight`, `--space`. No visual change. Check the page looks identical (quickstart §5, step 1, before the panel exists).

---

## Phase 2: Foundational (blocking prerequisites)

- [X] T002 [P] Write failing tests in the new `web/themes.test.js`, for a new `src/web/assets/themes.js`:
  - `PALETTES` has `light`, `dim` and `dark`, each with every key used by `app.css`, plus
    `sky`, `horizon`, `fog`, `basemapStyle` (`liberty`, `fiord`, `dark`) and `dimOpacity` (0,
    0.35, 0.55);
  - `contrastRatio(fg, bg)` gives 21 for black on white and 1 for equal colours;
  - for every palette, `text`/`bg`, `text`/`panel`, `muted`/`panel` and the banner text pairs
    have a contrast of at least 4.5 (SC-002).
- [X] T003 Implement `src/web/assets/themes.js` (`PALETTES`, `contrastRatio` with WCAG relative luminance, and `applyPalette(root, palette)`, which sets the CSS variables and `data-theme`). Make T002 pass. Light's values equal T001's.

**Checkpoint**: the palettes exist and their contrast is proven. The page is still light.

---

## Phase 3: User Story 2 - All viewer settings in one place (Priority: P1) 🎯 MVP

**Goal**: one gear button opens a right-side panel holding every viewer setting, grouped. The map
stays visible, and nothing else covers it.

**Independent Test**: quickstart §5.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T004 [P] [US2] Write failing tests in the new `web/panel.test.js`, for a new `src/web/assets/panel.js` (`createPanel()`, `toggle`, `close`, `activity(now)`, `tick(now)`, `isOpen`). Cover:
  - it starts closed;
  - the gear toggles it;
  - close (✕ or Escape) closes it;
  - `tick` closes it once 5 minutes pass since the last `activity`, and not at 4 m 59 s;
  - `activity` resets the timer;
  - a closed panel is untouched by `tick`;
  - each closing reports `focusGear: true` (FR-008).

### Implementation for User Story 2

- [X] T005 [US2] Implement `src/web/assets/panel.js`, a pure state machine with an injected clock. Make T004 pass.
- [X] T006 [US2] In `src/web/assets/index.html`, remove `#settings` and add the panel per `contracts/page-ui.md`:
  - the `<aside id="panel">` with a ✕ button;
  - `<fieldset>` groups Map ("Last" N `<select id="limit">`), Last QSOs box (the existing three
    controls, same ids), Appearance (empty for now) and Legend (empty for now).

  Style the panel in `src/web/assets/app.css`: right edge, 320 px wide; full width under 600 px;
  CSS variables only; a slide-in transition.
- [X] T007 [US2] In `src/web/assets/app.js`:
  - add a `SettingsControl`, a MapLibre control in `top-right` with a gear button (`aria-label`,
    `aria-expanded`, `aria-controls="panel"`);
  - wire it to `panel.js`: gear toggles; ✕ and Escape close; a 15 s interval calls
    `tick(Date.now())`; pointer, key and focus events inside the panel call `activity`;
  - focus the first control on opening, and the gear on closing;
  - keep the existing handlers for the moved controls, which have the same ids, so their saved
    values and behaviour are unchanged (FR-009).
- [ ] T008 [US2] **Verify** quickstart §5 by hand (open/close by mouse, keyboard and Escape; auto-close after 5 minutes; full width under 600 px; existing values kept), and record the result in the task's commit message.

**Checkpoint**: the map is free of settings; the panel holds them all.

---

## Phase 4: User Story 1 - The page matches the light around it (Priority: P1)

**Goal**: five theme choices. With "Automatic (device)" as the default, the page, the base map
and every panel follow the chosen theme within 60 seconds.

**Independent Test**: quickstart §3.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T009 [P] [US1] Write failing tests in the new `web/sun.test.js`, for a new `src/web/assets/sun.js` (`solarElevation(date, lat, lon)`):
  - Cagliari (39.22N 9.12E) at 2026-03-20 11:30 UTC is between 49° and 52°;
  - the same place at 2026-03-20 23:30 UTC is below −40°;
  - Tromsø (69.65N 18.96E) at 2026-06-21 23:00 UTC is above 0° (polar day);
  - Tromsø at 2026-12-21 11:00 UTC is below 0° (polar night);
  - the equator at the equinox at local noon is above 85°.
- [X] T010 [P] [US1] Add failing tests to `web/themes.test.js` for `resolveTheme(choice, { prefersDark, sunElevation })`:
  - every row of research R2's table, including elevations 0, −0.1, −6 and −6.1, and
    `auto-sun` with `null` behaving like `auto-device`;
  - `loadTheme`/`saveTheme` (`lqm.theme`): five valid values, the default `auto-device`, and
    fallback on unknown values, `null` and throwing storage;
  - `themeStatus(choice, inputs)` gives the status text from `contracts/page-ui.md`;
  - in `web/view.test.js`, `basemapFor(basemap, palette, defaultUrl)` returns
    `{ styleUrl, dimOpacity }`:
    - for the default OpenFreeMap URL, the palette's style
      (`https://tiles.openfreemap.org/styles/<basemapStyle>`) and `dimOpacity: 0`;
    - for any other style URL or raster tiles, `styleUrl: null` (keep the style) and the
      palette's `dimOpacity`.

### Implementation for User Story 1

- [X] T011 [US1] Implement `solarElevation` in `src/web/assets/sun.js` (NOAA equations, UTC, no refraction; research R3). Make T009 pass.
- [X] T012 [US1] Implement `resolveTheme`, `loadTheme`, `saveTheme` and `themeStatus` in `src/web/assets/themes.js`, and `basemapFor` in `src/web/assets/view.js`. Make T010 pass.
- [X] T013 [US1] In `src/web/assets/app.js`:
  1. **Recompute** the theme on load, on the `matchMedia('(prefers-color-scheme: dark)')`
     `change` event, every 60 s, and when the house-marker position from `currentQth()`
     changes;
  2. **Apply** it with `applyPalette`;
  3. **Default base map:** take the decision from
     `basemapFor(hello.basemap, palette, DEFAULT_STYLE_URL)` (T012). When it gives a `styleUrl`,
     swap to it with `map.setStyle(…, { diff: false })`. On `style.load`,
     re-add the QSO, QTH and great-circle sources and layers (move their setup from
     `map.on('load')` into a reusable function), and redraw the current data;
  4. **Other base maps** (`styleUrl` is `null`): add or update a `background` layer
     `theme-dim` with the palette's colour and `basemapFor`'s `dimOpacity`, inserted below the
     first QSO layer. Only these MapLibre calls stay checked by hand;
  5. **Keep the view:** don't touch the camera, and let any animation finish.
- [X] T014 [US1] Add the Appearance group to the panel in `src/web/assets/index.html`: a `<select id="theme">` with the five choices, and a status line `<p id="theme-status" aria-live="polite">`. Wire both in `src/web/assets/app.js` (save on change; update the status on every recompute).
- [ ] T015 [US1] **Verify** quickstart §3 by hand (device switch, sun mode with a faked time, fixed themes kept after reload, dimming of an operator-configured map), and record the result in the commit message.

**Checkpoint**: the page follows the light, and every theme is readable.

---

## Phase 5: User Story 3 - Atmosphere around the globe (Priority: P2)

**Goal**: the globe has an atmosphere glow by default, in the theme's colours, with a switch in
the Map group that is saved.

**Independent Test**: quickstart §4.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T016 [P] [US3] Add failing tests to `web/view.test.js`:
  - `loadAtmosphere`/`saveAtmosphere` (`lqm.atmosphere`): default `on`, `off` kept, unknown or
    throwing storage gives `on`;
  - `skyFor(palette, { atmosphere, projection })` returns the MapLibre sky object, with
    `atmosphere-blend` 1 only for `atmosphere` on and `projection` `globe`, else 0, and the
    palette's `sky`, `horizon` and `fog` colours.

### Implementation for User Story 3

- [X] T017 [US3] Implement `loadAtmosphere`, `saveAtmosphere` and `skyFor` in `src/web/assets/view.js`. Make T016 pass.
- [X] T018 [US3] Add the checkbox "Atmosphere on the globe" (`id="atmosphere"`) to the Map group in `src/web/assets/index.html`. In `src/web/assets/app.js`, call `map.setSky(skyFor(…))` on load, on theme change, on projection change, on `style.load` and on checkbox change; save on change. Set the map container's background to `var(--space)` in `src/web/assets/app.css`.
- [ ] T019 [US3] **Verify** quickstart §4 by hand.

**Checkpoint**: the globe looks like a planet; the flat map is unchanged.

---

## Phase 6: User Story 4 - Show only chosen band groups (Priority: P2)

**Goal**: the Legend group offers the server's band groups. The choice filters the legend, the
map and the box, and "last N" counts only shown QSOs, computed by the server (protocol 4).

**Independent Test**: quickstart §6, plus the automated tests below.

### Tests for User Story 4 (write first, confirm they FAIL)

- [X] T020 [P] [US4] Write failing Rust tests:
  - **`src/band_filter.rs`:** `group_label(name)` for every group, and `considered_groups(filter)`
    for a filter of `hf`. It keeps `hf`, `hf-contest` and `warc` with their considered bands
    only, in frequency order, and drops `vhf`.
  - **`src/history.rs`:** `latest_matching(n, bands)` gives the newest `n` QSOs whose `BAND` is
    in `bands` (case-insensitive), skips QSOs without a `BAND`, and gives the same result as
    `latest` for `None`.
- [X] T021 [P] [US4] Write failing Rust tests in `src/web/protocol.rs` and `src/web/session.rs`:
  - `hello` has `protocol: 4` and `band_groups` (shape per `contracts/websocket-protocol.md`);
  - `ClientMessage::parse` accepts `set_limit` with and without `bands`, and with `bands: null`;
  - after `set_limit { limit: 10, bands: ["40m"] }`, the session's snapshot holds only 40 m QSOs,
    at most 10;
  - unknown band names are dropped, and an empty list behaves like `null`;
  - upserts are still forwarded for every band.
- [X] T022 [P] [US4] Write failing tests in the new `web/legend.test.js`, for a new `src/web/assets/legend.js`:
  - `offeredGroups(hello.band_groups)` returns them as given, each with
    `text: "<label> (<n> bands)"`, singular for one band, for example "WARC (1 band)" (FR-012);
  - `chosenBands(choice, bandGroups, allBands)` gives the union in frequency order, or `null`
    for `all`;
  - `loadLegend`/`saveLegend` (`lqm.legend`) drop unknown names, turn an empty or stale array
    into `all`, and survive throwing storage;
  - `toggleGroup(choice, name)` makes "all" exclusive: ticking a group leaves `all`, and
    unticking the last group returns to `all`.
- [X] T023 [P] [US4] Add failing tests to `web/map-state.test.js` for `setShownBands(state, bands)` and the drawing helpers:
  - with bands set, only QSOs on those bands, case-insensitive, are drawn and listed;
  - QSOs without a `BAND` are hidden;
  - a correction moving a QSO onto a hidden band removes it from what is drawn;
  - "last N" counts only shown QSOs;
  - `null` shows everything.
- [X] T024 [US4] Update `tests/ws_protocol.rs` (a failing test first):
  - `protocol: 4` and `band_groups` in `hello`;
  - `set_limit` with `bands` gives a filtered snapshot;
  - a page sending no `bands` still gets every QSO.

### Implementation for User Story 4

- [X] T025 [US4] Implement `group_label`, `considered_groups` (`src/band_filter.rs`) and `History::latest_matching` (`src/history.rs`). Make T020 pass.
- [X] T026 [US4] In `src/web/protocol.rs` and `src/web/session.rs`: `PROTOCOL_VERSION = 4`; `hello.band_groups` from `considered_groups`; `ClientMessage::SetLimit { limit, bands: Option<Vec<String>> }` with `#[serde(default)]`; the session stores the cleaned band list and builds snapshots with `latest_matching`. Make T021 and T024 pass.
- [X] T027 [US4] Implement `src/web/assets/legend.js`. Make T022 pass.
- [X] T028 [US4] Implement `setShownBands` and the band filtering of the drawing helpers in `src/web/assets/map-state.js`. Make T023 pass, and keep the existing map-state tests green.
- [X] T029 [US4] Wire it up:
  - **Panel:** add the Legend group to `src/web/assets/index.html` ("All considered bands" plus
    one checkbox per group, labelled with `offeredGroups(…).text` and built from
    `hello.band_groups` in `src/web/assets/app.js`).
  - **On a change:** save it, call `setShownBands`, redraw the legend (`showLegend` with only the
    chosen bands), and send `set_limit` with `bands` (`connection.js`'s `set_limit` message gains
    `bands`).
  - **On reconnect:** re-send the same `bands`.
- [ ] T030 [US4] **Verify** quickstart §6 by hand.

**Checkpoint**: the legend describes exactly what is drawn.

---

## Phase 7: User Story 5 - Contest bands by default on the server (Priority: P3)

**Goal**: with no band setting, the server considers the six contest HF bands. `all` gives every
band, and the export always writes `bands`.

**Independent Test**: quickstart §2.

### Tests for User Story 5 (write first, confirm they FAIL)

- [ ] T031 [P] [US5] Write failing tests in `src/band_filter.rs` for `parse_setting`:
  - empty gives `hf-contest`;
  - `["all"]` gives `None`;
  - `["all", "2m"]` and `"all,-60m"` give `Error::Config` naming `bands`;
  - other values behave as before.

  Update the existing "empty means every band" test to the new rule.
- [ ] T032 [P] [US5] Write failing tests:
  - **`src/config.rs`:** with no setting, `band_filter` is `hf-contest`; `--bands all` and
    `LQM_BANDS=all` give every band (`None`).
  - **`src/config_export.rs`:** the export writes `bands = ["all"]` for every band and the six
    bands for the default, and a reload gives the same set. Replace the test asserting "no
    filter means no bands key".
  - **`src/pipeline.rs`:** `bands_log_line` prints `bands considered: all` for `None` (unchanged)
    and the six bands for the default.

### Implementation for User Story 5

- [ ] T033 [US5] First, make `test_config()` in `tests/common/mod.rs` pin every band (`config.band_filter = None`), with a comment: "integration tests don't depend on the default bands; feature 011 tests the default in T031–T032". Existing tests then keep their meaning whatever the default is. Then implement the default and `all` in `parse_setting` (`src/band_filter.rs`), the configuration plumbing in `src/config.rs` (any place that treats "no setting" as `None`), and the export in `src/config_export.rs`. Make T031 and T032 pass, and fix any unit test that relied on the old default (search `band_filter: None` and `bands_considered`).
- [ ] T034 [US5] Update `deploy/config.example.toml`: the commented `bands` line explains the default (the six contest HF bands) and `bands = ["all"]`.

**Checkpoint**: a fresh installation's legend shows six colours (SC-006).

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T035 [P] Update `README.md`:
  - **"The map page":** the settings panel and its groups, the themes and the automatic modes,
    the atmosphere, the band-group choice;
  - **"Band selection":** the new default and `all`, with the upgrade note (FR-018).
- [ ] T036 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md`:
  - **Added:** themes (Light/Dim/Dark, automatic by device or by the sun at the station), the
    atmosphere, the settings panel, the band-group choice;
  - **Changed:** the default bands are now the six contest HF bands, with `bands = ["all"]` for
    every band; protocol 4.
- [ ] T037 Run the local gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `node --test 'web/*.test.js'`, `python3 -m unittest discover -s tools -p 'test_*.py'`, the docs build and the audit (`docs/ci.md` section 11).
- [ ] T038 Mark the tasks done, commit per phase, push `feature/011-theme-settings-panel`, and open the pull request. Merge it once the five gates are green; delete the branch afterwards (constitution, Development Workflow).

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T003)** → stories.
- **US2 (T004–T008)** first: the panel hosts the controls of every other story.
- **US1 (T009–T015)** after US2: T014 adds to the panel. T009–T012 (pure modules) can be
  written in parallel with US2.
- **US3 (T016–T019)** after US1: the sky uses the theme's palette.
- **US4 (T020–T030):**
  - the Rust side (T020, T021, T024–T026) is independent of US1–US3 and can run any time after
    Phase 2;
  - the page side (T027–T029) needs the panel (US2).
- **US5 (T031–T034)** is independent of the page. Do it **after T026**, because its new default
  changes which bands `hello` and the tests see.
- **Polish (T035–T038)** last.
- **Shared files:**
  - `src/web/assets/app.js` is edited by T007, T013, T014, T018 and T029, in order;
  - `index.html` by T006, T014, T018 and T029;
  - `src/band_filter.rs` by T020, T025, T031 and T033.

## Parallel Opportunities

- **Phase 2:** T002 alongside T001.
- **Pure modules:** T004 (panel), T009–T010 (sun, themes), T016 (view), T022 (legend) and T023
  (map-state) are all separate test files.
- **Rust:** T020 and T021 are separate modules; T031 and T032 too.
- **Polish:** T035 and T036 are separate files.

### Parallel example: pure modules

```text
Task: "T004 [US2] web/panel.test.js"
Task: "T009 [US1] web/sun.test.js"
Task: "T022 [US4] web/legend.test.js"
```

## Implementation Strategy

1. **MVP (US2):** Phases 1–3. The settings move into a panel, and the map is free of controls.
2. **US1:** themes that follow the device or the sun. This is the most visible change.
3. **US3:** the atmosphere, a small visual addition on top of the themes.
4. **US4:** band groups. The protocol bump, server and page.
5. **US5:** the server default. Last, because it changes what other tests see.
6. **Polish:** docs, changelog, gates, pull request.
