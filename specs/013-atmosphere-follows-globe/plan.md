# Implementation Plan: Atmosphere that moves with the globe, and the grayline

**Branch**: `feature/013-atmosphere-follows-globe` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/013-atmosphere-follows-globe/spec.md`

## Summary

**Globe glow (User Stories 1 and 2):**
- **Cause:** the glow's bright side comes from the MapLibre style light, whose default
  `anchor: "viewport"` pins it to the window (research R1).
- **Fix:** recompute the light from the camera on every `move`, inside the animation frame. It
  points towards the Sun's on-screen direction, from behind the globe (R2, revised during
  implementation). The rim is brightest towards the Sun, and the scattering never hazes the map
  (owner's rule, FR-019).

**Grayline (User Story 3):**
- **Drawing:** raster tiles the page draws itself (a `grayline://` protocol). Each pixel's
  opacity follows the Sun's height there: nothing at 0° and above, full night from −12°, linear
  between. One raster layer works the same on the flat map, on every world copy and on the
  globe, attached to the map (R7, revised during implementation: the first design, 25 stacked
  GeoJSON caps, made the globe 3× slower to drag).
- **Layer order:** the layer goes just before the style's first boundary layer, so place names
  and borders stay above it (R9).

**Sun symbol:** an optional symbol layer at the subsolar point, under every QSO overlay (R10).

**One Sun:** `sun.js` gains `subsolarPoint(date)`, sharing its equations with `solarElevation`.
The glow, the grayline, the Sun symbol and the automatic theme all use it. Everything is
refreshed after each style load and on the existing 60 s tick. The glow's light is also
recomputed on every camera move.

**Settings:** `lqm.grayline` (on by default) and `lqm.sun` (off by default), two checkboxes in
the Map group (R11).

**Tests:**
- **Unit tests:** pure functions in `sun.js`, `geodesic.js`, the new `grayline.js`, `view.js`
  and `themes.js`.
- **Release browser test:** two new steps, `atmosphere` and `grayline`, on a test clock. They
  measure screenshots, and the flight frames' camera and light (R5, R13).

## Technical Context

**Language/Version**: browser ES modules (the change is page-only). Rust (edition 2024, latest
stable toolchain; see `docs/rust-guidelines.md` §2) changes only by one route: `src/web/mod.rs`
serves the new `grayline.js` (`include_str!` and `/grayline.js`), like every other page file. Node 22+ runs the browser test runner.

**Primary Dependencies**: none new.
- MapLibre GL JS 5.24.0, already loaded: `setLight`, `addProtocol` with a `raster` source, and a
  GeoJSON `symbol` layer for the Sun.
- OpenFreeMap styles `liberty`, `fiord` and `dark`, unchanged.

**Storage**: browser `localStorage`: `lqm.grayline`, `lqm.sun`. No server state.

**Testing**:
- **Every change:** `node --test 'web/*.test.js'`, and `cargo test` (feature 012's test fetches
  every page file found on disk, so `grayline.js` is covered).
- **Releases:** `node tests/browser/run.mjs`.

**Target Platform**: browsers with WebGL 2, as before. Development on any host. Release targets
as configured in `.circleci/config.yml` (Principle V).

**Project Type**: single binary, with embedded web assets.

**Performance Goals**:
- Per frame while the map moves: one `setLight` (a few multiplications).
- Once a minute: new grayline tiles (a few 256 px tiles at contest zooms, drawn in JavaScript),
  one `setData` for `sun`.
- Measured (step 11, the same 3 s drag): 60 frames/s with or without the new layers on a real
  GPU; in the test browser's software renderer 19.7 against 24.7. That is recorded, not judged
  (SC-004, owner's decision).

**Constraints**:
- no new crates, npm packages or CDN libraries;
- UTC everywhere (HR-1);
- the feature 011 atmosphere rules are unchanged (FR-006).

**Scale/Scope**:
- about 250 lines of page code, with one new module, `grayline.js`;
- about 200 lines of runner code: two steps and five helpers.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: unit tests come before each pure function, and fail first because the
  function doesn't exist:
  - `subsolarPoint`, `destination`, `shade`, `elevationAt`, `tilePixels`, `graylineTiles`,
    `graylineBeforeId`,
    `lightFor`, and the new load and save functions;
  - the `getSunPos` replay, which is the regression test for A1 and A2;
  - edge cases: poles, solstices, the date line, midnight UTC, and storage that throws.

  Browser steps 10 and 11 are written before the `app.js` wiring, and fail against today's page.
- [x] **II. Rust Safety & Quality**: the only Rust change is one `include_str!` constant and one
  route in `src/web/mod.rs`, the existing pattern, with no fallible code. `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` and `cargo test` keep passing.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new dependency;
  - MapLibre's own atmosphere and a raster layer, instead of a custom WebGL layer (R1, R7,
    revised);
  - one new module, holding only pure grayline maths.
- [x] **Domain rules** (`docs/ham-radio.md` §1): Sun positions are computed from UTC (HR-1).
  "Grayline" is added to the glossary (§2.11), as the spec requires.
- [x] **IV. Persistent Viewer Settings**: both new choices are saved in `localStorage`. Reads fall
  back to their defaults on missing or unknown values, or when storage throws. Unit tests and
  browser step 11 (reload) cover them.
- [x] **V. Portable Development, Pipeline-Built Releases**: page JavaScript and Node tests only,
  native on every host. No platform-specific code.
- [x] **Interface rules** (`docs/http-api.md`): no HTTP or WebSocket change. `grayline.js` is a
  static page file, exempt from API-1.

**Post-design re-check (after Phase 1)**: still passes. The design adds:
- pure functions in five existing modules, and one new pure module;
- two checkboxes;
- two layers and sources, added in `addOverlays()`;
- one `applySun()`, called from places that already exist;
- two runner steps.

No principle is bent, so Complexity Tracking stays empty.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/013-atmosphere-follows-globe/
├── plan.md
├── research.md              # R1–R6 globe glow; R7–R13 grayline, Sun symbol, settings, tests
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── page-modules.md      # sun, geodesic, grayline, view, themes, app wiring
│   └── browser-test.md      # steps 10 `atmosphere`, 11 `grayline`
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
src/web/assets/
├── sun.js          # + subsolarPoint(date); shared declination / equation of time
├── geodesic.js     # + destination(point, bearing, distance)
├── grayline.js     # NEW: shade, elevationAt, tilePixels, graylineTiles, graylineBeforeId (pure)
├── view.js         # + lightFor; load/save Grayline and Sun position choices
├── themes.js       # + night, nightOpacity per palette
├── app.js          # + grayline and sun sources/layers in addOverlays(); applySun()
└── index.html      # + #grayline, #sun-marker checkboxes in the Map group
web/
├── sun.test.js, geodesic.test.js, view.test.js, themes.test.js   # + new cases
├── grayline.test.js                                             # NEW
└── browser-lib.test.js                                          # + runner helpers
tests/browser/
├── cdp.mjs         # + drag(from, to)
├── lib.mjs         # + limbSamples, sunSide, toScreenMercator, toScreenGlobe, luminance
├── run.mjs         # + steps 10 `atmosphere`, 11 `grayline`
└── README.md       # step list
src/web/mod.rs      # + /grayline.js route (include_str!)
docs/ham-radio.md   # + §2.11 Grayline
README.md           # settings panel: Grayline, Sun position; grayline described
CHANGELOG.md        # Fixed: glow turns with the globe; Added: grayline, Sun position
```

**Structure Decision**:
- **Grayline geometry:** a module of its own, `grayline.js`, like `geodesic.js` and
  `autozoom-geo.js`: pure, and tested with `node --test`.
- **Glue:** `app.js` keeps the wiring, as it does for the other overlays.
- **Release checks:** the feature 012 runner gets two more steps, on the release pipeline job
  that already exists.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
