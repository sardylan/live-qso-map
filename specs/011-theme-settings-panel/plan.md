# Implementation Plan: Automatic Theme, Atmosphere, Settings Panel and Legend Selection

**Branch**: `feature/011-theme-settings-panel` | **Date**: 2026-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/011-theme-settings-panel/spec.md`

## Summary

**Page:**
- **Themes:** three palettes (Light, Dim, Dark) in a pure `themes.js`, with a contrast test.
  `resolveTheme` picks one of them from the viewer's choice: the device preference by default,
  or the sun's elevation at the house marker (pure `sun.js`, NOAA equations).
- **Base map:** for the default OpenFreeMap map, the style switches between `liberty`, `fiord`
  and `dark`. Other maps get a dimming layer under the markers.
- **Atmosphere:** the globe uses MapLibre's built-in sky (`setSky`, `atmosphere-blend`), on by
  default and saved.
- **Settings panel:** every viewer setting moves into a right-side panel opened by a gear
  button, with its open/close and 5-minute auto-close logic in a pure `panel.js`.
- **Band groups:** a Legend group offers the server's band groups (`hello.band_groups`). The
  choice filters the legend, the map and the recent box.

**Server:**
- **Band groups:** `hello` lists the band groups with their considered bands.
- **Filtered snapshot:** `set_limit` takes an optional `bands`, so the snapshot is the newest N
  QSOs **on those bands** (protocol 4).
- **Default bands:** the band setting defaults to `hf-contest`; `all` restores every band. The
  export always writes `bands`.

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md`
§2); browser ES modules

**Primary Dependencies**: none new. The page uses MapLibre GL JS 5.24.0 (pinned, SRI), whose
`setSky` it now uses. OpenFreeMap styles `fiord` and `dark` are added next to `liberty`.

**Storage**: browser storage, with the new keys `lqm.theme`, `lqm.atmosphere` and `lqm.legend`;
no server storage.

**Testing**:
- `cargo test`: band setting, `band_groups`, filtered snapshot, export;
- `node --test 'web/*.test.js'`: `themes.js`, `sun.js`, `panel.js`, `legend.js`, `map-state.js`;
- manual quickstart §3–§6 for the DOM and map wiring.

**Target Platform**: development on any host; release targets as configured in
`.circleci/config.yml` (Principle V); any current browser with WebGL.

**Project Type**: single binary (it serves the page)

**Performance Goals**:
- a theme switch within 60 s of the condition changing (FR-003);
- the sun is recomputed once a minute (negligible);
- a style swap redraws within normal map loading time.

**Constraints**:
- contrast ≥ 4.5:1 in every theme (SC-002);
- no new libraries (Principle III);
- every viewer setting persisted, and tolerant of bad values (Principle IV);
- protocol messages stay JSON with `type` (`docs/http-api.md`).

**Scale/Scope**: 3 themes, 5 theme choices, 8 band groups, 7 viewer settings in 4 panel groups.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests come before code.
  - **Pure page modules,** with `node --test`:
    - palette contrast;
    - `resolveTheme`, every row;
    - sun elevation at known dates and places, including polar day and night;
    - the panel's toggle, Escape and 5-minute auto-close, with an injected clock;
    - the legend choice: what is offered, the union, the fallback for stale values, the band
      list sent to the server;
    - the map-state band filter and "last N" over the shown QSOs.
  - **Rust:**
    - the `parse_setting` default and `all`, plus errors;
    - `band_groups` content and order;
    - `History::latest_matching`;
    - the session snapshot with `bands`;
    - the export round-trip;
    - the `ws_protocol` integration test for version 4.
  - **Map wiring:** the style swap, the dimming layer and `setSky` are checked by hand
    (quickstart), as in features 002 and 004.
- [x] **II. Rust Safety & Quality**:
  - no `unsafe`, no `unwrap`/`expect` in runtime code;
  - a bad `bands` value from a page is ignored and logged at debug level (Principle II: invalid
    input is skipped);
  - the new configuration error uses the existing `Error::Config`.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new crate or frontend library;
  - the sun calculation is about 40 lines;
  - the panel is plain HTML/CSS;
  - the band groups are reused from the server's table.
- [x] **Domain rules**:
  - **HR-1:** sun computations are in UTC.
  - **HR-2:** every band list (`band_groups`, legend union, startup log, export) is in frequency
    order.
  - **HR-3:** band names follow the ADIF band plan.
- [x] **IV. Persistent Viewer Settings**:
  - theme, atmosphere and legend choice are saved in browser storage and survive a restart;
  - missing, unknown or unreadable values fall back to the defaults;
  - the existing settings keep their keys and values;
  - the panel's open state is UI state, not a setting.
- [x] **V. Portable Development**: pure JS and Rust, tested on any host; no platform-specific
  code.
- [x] **Interface rules** (`docs/http-api.md`):
  - `hello.band_groups` and `set_limit.bands` are JSON with `snake_case` fields, and arrays where
    order matters;
  - the protocol version is bumped to 4;
  - an unknown or bad `bands` is ignored, never answered with an error frame (as for
    `set_limit`);
  - the contract tests are updated.

**Post-design re-check**: unchanged after Phase 1. No violations.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/011-theme-settings-panel/
├── spec.md
├── plan.md
├── research.md            # R1–R11
├── data-model.md          # settings keys, themes, sun, panel, band groups, band setting
├── quickstart.md
├── contracts/
│   ├── websocket-protocol.md   # protocol 4: hello.band_groups, set_limit.bands
│   ├── page-ui.md              # layout, panel groups, closing, theme application
│   └── configuration.md        # band default, "all", export
├── checklists/requirements.md
└── tasks.md               # /speckit-tasks
```

### Source Code (repository root)

```text
src/band_filter.rs            # default hf-contest, "all", group labels
src/config.rs                 # band setting default and "all" through CLI/env/file
src/config_export.rs          # always writes bands (list or ["all"])
src/history.rs                # latest_matching(limit, bands)
src/web/protocol.rs           # protocol 4, hello.band_groups, set_limit.bands
src/web/session.rs            # remembers bands; filtered snapshot
src/web/assets/index.html     # gear button, panel markup; old settings box removed
src/web/assets/app.css        # var(--…) everywhere, panel styles, data-theme
src/web/assets/app.js         # wiring: theme, style swap/dim layer, setSky, panel, legend choice
src/web/assets/themes.js      # new: palettes, resolveTheme, contrast helper
src/web/assets/sun.js         # new: solarElevation
src/web/assets/panel.js       # new: open/close/auto-close state
src/web/assets/legend.js      # new: offered groups, chosen bands, stored-choice validation
src/web/assets/map-state.js   # band filter for what is drawn
src/web/assets/view.js        # basemapFor (style URL / dim opacity per theme), loadAtmosphere/saveAtmosphere, skyFor
web/themes.test.js, web/sun.test.js, web/panel.test.js, web/legend.test.js   # new
web/map-state.test.js         # extended
tests/ws_protocol.rs          # protocol 4
deploy/config.example.toml, README.md, CHANGELOG.md   # default bands, settings panel, themes
```

**Structure Decision**: existing layout. The new pure page modules sit next to the existing ones
(`qth.js`, `recent.js`, …), with tests in `web/`.

## Complexity Tracking

No violations.
