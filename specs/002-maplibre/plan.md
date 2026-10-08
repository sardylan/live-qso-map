# Implementation Plan: MapLibre Map Rendering

**Branch**: `002-maplibre` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-maplibre/spec.md`

## Summary

The page renderer changes from Leaflet 1.9.4 to MapLibre GL JS 5.24.0, loaded from cdnjs with full
Subresource Integrity (research R1).

- **Markers and lines** become two GPU layers fed by GeoJSON: a circle layer sorted by `zRank` and
  a line layer of great-circle paths. Two new pure modules build that data: `layers.js` (GeoJSON)
  and `view.js` (base map style, projection preference, WebGL check).
- **Base map**: the operator chooses a vector style (default OpenFreeMap Liberty, with its required
  attribution) or the feature-001 raster tiles. The service rejects configuring both.
- **Projection**: pages open as a globe, and a per-viewer toggle switches to flat, remembered in
  `localStorage`.
- **No WebGL**: the page shows a clear message instead of a map.
- **Server**: only the configuration and `hello.basemap` change, so the protocol version becomes 2.

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules
**Primary Dependencies**: no new crates; the frontend uses MapLibre GL JS 5.24.0 (cdnjs, SRI)
instead of Leaflet 1.9.4
**Storage**: browser `localStorage` for the projection preference; no server storage
**Testing**: `cargo test` and `node --test web/` (new tests for `view.js` and `layers.js`)
**Target Platform**: unchanged; browsers with WebGL (FR-012 covers the others)
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**: at least 30 fps panning and zooming with 2,000 QSOs, globe and flat (SC-002);
live QSO within 2 s (SC-003)
**Constraints**: full SRI on the map library (FR-011); QSO text never rendered as markup (FR-004);
bands in frequency order (HR-2)
**Scale/Scope**: up to the history size on screen (default 10,000; SC-002 measured at 2,000)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - `view.js` and `layers.js` are pure and get `node --test` coverage first;
  - Rust tests cover the base-map configuration (defaults, conflict, raster) and the `hello.basemap`
    shape;
  - the feature-001 tests for `map-state.js`, `geodesic.js` and `connection.js` stay unchanged
    (FR-007);
  - visual parity, globe rendering, the no-WebGL page and fps are checked manually, as in
    quickstart §2–§6.
- [x] **II. Rust Safety & Quality**: no `unsafe`; the new configuration error is
  `Error::Config { setting: "map.style_url", .. }` per `docs/rust-guidelines.md`; fmt and clippy are
  gates.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate. The frontend library swap is
  justified below. Two small pure modules replace nothing and add no build step.
- [x] **Domain rules**: HR-2 (legend unchanged, frequency order); HR-7 (popups still show precision
  and source); the great-circle rendering follows `docs/ham-radio.md` §2.10.

### Dependency Justification

| Dependency | Why needed | Alternatives considered |
|------------|------------|-------------------------|
| MapLibre GL JS 5.24.0 (frontend, CDN with SRI), replacing Leaflet 1.9.4 | the owner asked for it; it adds GPU rendering for 2,000+ QSOs (SC-002), the globe projection (FR-010) and vector base maps (FR-008) | MapLibre 6.x: ES-module only, so the worker can't be SRI-checked (research R1); keeping Leaflet: no globe and slow at 2,000 DOM markers |

## Project Structure

### Documentation (this feature)

```text
specs/002-maplibre/
├── plan.md
├── research.md          # R1–R7
├── data-model.md        # BaseMap, Projection, view/layer helpers
├── quickstart.md
├── contracts/
│   ├── configuration.md         # style_url / style_attribution; tile_* without defaults
│   ├── websocket-protocol.md    # protocol 2, hello.basemap
│   └── http-endpoints.md        # /view.js, /layers.js, MapLibre SRI
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── config.rs            # + style_url, style_attribution; tile_url/tile_attribution → Option; conflict check  [tests]
└── web/
    ├── protocol.rs      # hello: protocol 2, `basemap` replaces `tiles`                                  [tests]
    ├── mod.rs           # + /view.js, /layers.js routes
    └── assets/
        ├── index.html   # MapLibre 5.24.0 CSS/JS with SRI; no-WebGL message container; projection toggle
        ├── app.css      # remove Leaflet-specific rules; toggle, message and popup styles
        ├── app.js       # MapLibre glue: map, two sources and layers, popups, attribution, toggle
        ├── view.js      # NEW pure: buildStyle, load/save/nextProjection, hasWebGL                       [node tests]
        └── layers.js    # NEW pure: markerFeatures, lineFeatures                                          [node tests]

web/view.test.js, web/layers.test.js         # NEW
tests/ws_protocol.rs                         # + /view.js, /layers.js served; hello.basemap shape
deploy/config.example.toml, README.md        # base map settings
```

**Structure Decision**: the feature-001 layout is unchanged. All rendering decisions stay in pure,
tested modules, and `app.js` remains glue only.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] I: every new decision (style building, projection preference, WebGL check, GeoJSON features)
  is in a pure module with tests. Visual and performance checks are explicit quickstart steps.
- [x] II: the configuration conflict is a structured error. No panics on input.
- [x] III: no new crates. One frontend library replaced, pinned and SRI-checked.
- [x] Security: QSO data reaches the DOM only through `textContent` (`setDOMContent`). Only
  operator-supplied attribution is rendered as HTML.
