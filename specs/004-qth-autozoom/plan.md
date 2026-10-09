# Implementation Plan: QTH Marker, Auto-Zoom and Map Panels

**Branch**: `004-qth-autozoom` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-qth-autozoom/spec.md`

## Summary

The map gains six things:

- a house at the QTH;
- black great-circle lines from the QTH to every marker;
- an auto-zoom mode that keeps the house and every marker clear of the page's panels, choosing
  globe or flat when needed;
- a last-five-QSO box;
- a metric scale in flat mode;
- JSON bodies on every error response (constitution 1.3.0).

**Server**:

- `hello` (protocol 3) gains `station`, from the configuration, and `entity`: the station's DXCC
  entity and its extent.
- Extents come from a committed table generated from public-domain Natural Earth data by a dev-only
  script (research R2).
- A single helper turns error responses into JSON (R11).

**Page**:

- Every decision lives in four new pure modules: `qth.js`, `autozoom-geo.js`, `autozoom.js` and
  `recent.js`.
- Two existing pure modules are extended: `layers.js` and `view.js`.
- `app.js` remains glue only.
- The house is an inline SVG drawn in a symbol layer under the QSO markers (R3).

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules;
Python 3.11+ standard library for the dev-only generator
**Primary Dependencies**: no new crates, and no new frontend libraries (MapLibre GL JS 5.24.0 is
unchanged: `ScaleControl`, `addImage`, symbol layers, `cameraForBounds`)
**Storage**:

- browser `localStorage`: `lqm.autozoom` and `lqm.recent`, next to `lqm.projection`;
- an embedded `src/web/entity_extents.json`.

**Testing**: `cargo test` and `node --test web/`. New suites: `qth`, `autozoom-geo`, `autozoom` and
`recent`. Existing suites extended: `layers` and `view`.
**Target Platform**: unchanged; browsers with WebGL
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**:

- at most one refit per second under a 10 QSO/s stream with 2,000 QSOs (SC-005);
- a refit settles within 2 s (SC-002);
- the bounding-cap computation stays under 5 ms for 2,000 points.

**Constraints**:

- QSO text is plain text only (FR-022);
- interface rules: `docs/http-api.md` (FR-027);
- UTC times (HR-1);
- DXCC/WAE entities (HR-6);
- position source and precision are shown (HR-7).

**Scale/Scope**: up to the history size on screen, with the same 2,000-QSO reference as feature 002

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - every page rule is in a pure module with `node --test` coverage written first:
    - QTH choice;
    - the minimal longitude span and the antimeridian;
    - the bounding cap and whether it fits the globe;
    - padding from panels;
    - the controller's transitions (toggle, user move, popup pause, catch-up, 1 s throttle,
      automatic flat and back);
    - the box contents and order;
    - stored preferences;
  - Rust tests come first for:
    - `StationView` and `EntityView`;
    - `CountryData::entity_of`, including WAE entities, exact calls and portable calls;
    - parsing the extent table, including malformed entries;
    - the `hello` v3 shape;
    - every JSON error path;
  - visual checks are explicit quickstart steps: fit quality, pixel padding, the scale's accuracy
    and the globe camera.
- [x] **II. Rust Safety & Quality**:
  - no `unsafe`;
  - the extent table loads through `Result`, and a bad table is `Error::Config { setting:
    "entity_extents", .. }`, as `docs/rust-guidelines.md` requires;
  - no `unwrap` in runtime paths;
  - fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new crates and no new frontend libraries;
  - the generator is a dev-only, standard-library Python script, not part of the build. Its output
    is a committed data file, so the project is still one self-contained binary with no runtime
    download.
- [x] **Domain rules** (`docs/ham-radio.md` §1):
  - HR-1: UTC times in the box;
  - HR-5: callsigns compared case-insensitively;
  - HR-6: the "ham country" is the cty.dat DXCC/WAE entity;
  - HR-7: the house popup shows the source and precision;
  - §2.10: great-circle lines;
  - HR-2: the legend is unchanged.
- [x] **Interface rules** (`docs/http-api.md`):
  - API-1: `/healthz` is already JSON;
  - API-2: the new `hello` fields are in a JSON text frame;
  - API-3: JSON error bodies for 404, 405 and failed upgrades (FR-027), with contract tests;
  - API-4: `snake_case` fields, arrays for ordered data, and a protocol bump to 3;
  - API-5: the new `.js` assets are exempt.

### Dependency Justification

| Dependency | Why needed | Alternatives considered |
|------------|------------|-------------------------|
| none at runtime | — | — |
| Natural Earth v5.1.2 GeoJSON (data, dev-time only, public domain) | the source of approximate DXCC entity extents (FR-010, SC-003; owner decision R2) | a hand-curated table (accuracy can't be reviewed); a ~60-entity subset (too much radius fallback); operator configuration (rejected in clarification) |

## Project Structure

### Documentation (this feature)

```text
specs/004-qth-autozoom/
├── plan.md
├── research.md          # R1–R12
├── data-model.md        # StationView, EntityView, ExtentTable, page modules, controller
├── quickstart.md
├── contracts/
│   ├── websocket-protocol.md   # protocol 3: hello.station, hello.entity
│   └── http-endpoints.md       # new assets; JSON error responses
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── entity_extents.rs     # NEW: ExtentTable::embedded(), lookup by prefix              [tests]
├── enrich/cty.rs         # Entity.prefix; CountryData::entity_of()                     [tests]
├── pipeline.rs           # build_sources also returns the CountryData; HelloStation built at start
├── web/
│   ├── protocol.rs       # PROTOCOL_VERSION 3; StationView, EntityView; hello(.., &HelloStation) [tests]
│   ├── session.rs        # passes HelloStation through
│   ├── mod.rs            # json_error(); fallback 404; method_not_allowed_fallback 405;
│   │                     # /ws rejection → JSON; routes for the new .js files               [tests]
│   ├── entity_extents.json   # NEW, generated, committed
│   └── assets/
│       ├── index.html    # #recent box container; no new CDN resources
│       ├── app.css       # box, house popup, button states; box stacked above the scale
│       ├── app.js        # glue: qth layer and image, black lines, controls, controller wiring,
│       │                 # popup timer, ScaleControl add/remove, row clicks
│       ├── qth.js        # NEW pure: currentQth, houseFeature, HOUSE_SVG             [node tests]
│       ├── autozoom-geo.js # NEW pure: framedPoints, fallbackFrame, flatBounds,
│       │                 # boundingCap, fitsGlobe, globeZoomForCap, paddingFor       [node tests]
│       ├── autozoom.js   # NEW pure: controller state machine                         [node tests]
│       ├── recent.js     # NEW pure: recentQsos, rowView                              [node tests]
│       ├── layers.js     # lineFeatures(markers, qth): from the QTH, no colour property [node tests]
│       └── view.js       # load/saveAutoZoom, load/saveRecent, scaleVisible           [node tests]
tools/
├── entity_extents.py     # NEW dev-only generator (Python stdlib)
└── entity_extents.toml   # NEW cty prefix → Natural Earth selectors mapping
scripts/send_burst.sh     # optional delay and start-offset arguments (quickstart §3)

web/qth.test.js, web/autozoom-geo.test.js, web/autozoom.test.js, web/recent.test.js   # NEW
web/layers.test.js, web/view.test.js                                                   # extended
tests/ws_protocol.rs      # hello v3 shape; new assets served; JSON errors (404/405/ws)
.gitignore                # + /.cache/ (generator downloads)
README.md, deploy/config.example.toml   # QTH and auto-zoom notes; station settings now also drive the house
```

**Structure Decision**: the layout of features 001–003 is unchanged.

- Rendering decisions stay in pure, tested modules, and `app.js` remains glue only.
- The generated extent table sits next to the other embedded web assets, and the code that reads
  it sits next to the country data code.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] **I**: every new rule maps to a pure function or a Rust unit with planned tests. The controller
  is a state machine with an injected clock, so throttling and the popup timer are tested without
  real time. The camera's behaviour on the globe in MapLibre has a spike task first (R5).
- [x] **II**: the extent table and the startup station resolution return `Result`, and nothing
  panics on input. Error responses go through one tested helper.
- [x] **III**: no crates, libraries or runtime services added. The generator and its download stay
  outside the build.
- [x] **Domain rules**: HR-1, HR-5, HR-6 and HR-7 are explicit in the data model. Lines follow
  §2.10. The legend is untouched (HR-2).
- [x] **Interface rules**: API-1 to API-5 are covered by `contracts/` and the `ws_protocol.rs`
  tests.
- [x] **Security**: QSO data reaches the DOM only through `textContent`, in the box, house popup and
  marker popups. The house SVG is a constant. Error messages contain no configuration values or
  paths.
