# Feature Specification: MapLibre Map Rendering

**Feature Branch**: `002-maplibre`
**Created**: 2026-10-09
**Status**: Complete (2026-10-09). SC-002 was accepted as observed; see
`checklists/verification.md`.
**Input**: User description: "Replace Leaflet with MapLibre GL JS"

Terms follow `docs/ham-radio.md` §2. Band lists follow HR-2: always in frequency order.

## Context

Feature 001 (closed 2026-10-09) renders the map with Leaflet 1.9.4 from a CDN, using raster
OpenStreetMap tiles in a flat projection. All map decisions are already isolated in pure modules:
`map-state.js` (window, markers, colours, z-order, `diffLayers`), `geodesic.js` (great-circle points)
and `connection.js`. Only `app.js` talks to the map library. This feature replaces that library with
MapLibre GL JS and keeps every behaviour of feature 001.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Same live map, new renderer (Priority: P1)

A viewer opens the map during a contest and sees exactly what feature 001 shows:

- band-coloured markers, merged by position and stacked by QSO time;
- great-circle lines from the station;
- popups listing the QSOs at a marker, with position source and precision;
- the band legend in frequency order;
- the last-N selector;
- automatic reconnection.

The map now renders smoothly with many markers, and the base map stays crisp at every zoom level.

**Why this priority**: Replacing the renderer must not lose any feature-001 behaviour. Parity is the
minimum deliverable.

**Independent Test**: Run the feature-001 quickstart §2, §3 and §6 against the new page, and check
every parity item, FR-001 to FR-007, by hand in two browsers. Every frontend unit test from feature 001
still passes unchanged.

**Acceptance Scenarios**:

1. **Given** two open pages, **When** an ADIF QSO with a locator arrives, **Then** both show the
   marker and its great-circle line within 2 s (feature 001, SC-001).
2. **Given** QSOs at the same position on different bands, **When** they are shown, **Then** there is
   one marker, in the colour of the most recent QSO's band, and its popup lists all of them.
3. **Given** a path that crosses the 180° meridian, **When** it is drawn, **Then** the line is
   continuous (feature 001, research R15).

---

### User Story 2 - Choose the base map (Priority: P2)

The operator chooses the base map shown to viewers through configuration.

**Why this priority**: The base map determines the look, the cost (free vs. keyed providers) and the
usage policy. It must be under the operator's control.

**Independent Test**: Start the service with two different base-map settings and check that pages
render each one, with its attribution visible.

**Acceptance Scenarios**:

1. **Given** no base-map setting, **When** a page opens, **Then** the OpenFreeMap vector style is
   shown, with its attribution.
2. **Given** an operator-supplied vector style URL, **When** a page opens, **Then** that style is
   shown.
3. **Given** an operator-supplied raster tile URL template and attribution (the feature-001
   `tile_url`/`tile_attribution` settings), **When** a page opens, **Then** those raster tiles are
   shown instead of a vector style.

---

### User Story 3 - Globe view (Priority: P3)

At low zoom the viewer sees the Earth as a globe. Long-distance paths look like the real
great-circle arcs, and polar paths are no longer distorted.

**Why this priority**: This is the main visual gain of the new renderer for a DX/contest map. It is
not required for parity.

**Independent Test**: Open a page, zoom out, and check the projection and the path rendering
against acceptance scenario 1.

**Acceptance Scenarios**:

1. **Given** a page opened for the first time, **When** it is at world zoom, **Then** the Earth is
   shown as a globe, and long-distance paths follow the globe's surface.
2. **Given** a viewer who switches to flat, **When** they toggle, **Then** that page switches to the
   flat Mercator projection, and other pages are unaffected.
3. **Given** a viewer who chose flat, **When** they reload the page, **Then** it opens in flat again
   (remembered in that browser only, like the last-N selector).

---

### Edge Cases

- **Browser without WebGL**: the page shows a clear message that the map needs WebGL, with a hint
  to enable hardware acceleration. No map is rendered, and there is no fallback renderer (FR-012).
- **Base map unreachable**: QSOs, markers and lines still render on an empty background, and a notice
  explains why.
- **Very many markers** (All selected, thousands of QSOs): the page stays responsive (SC-002).

## Requirements *(mandatory)*

### Functional Requirements

**Parity with feature 001** (behaviour unchanged, renderer replaced)

- **FR-001**: One marker per distinct remote position, coloured by the band of its most recent QSO
  (feature 001, FR-024 and FR-025).
- **FR-002**: Markers stacked by QSO time, the most recent on top (feature 001, FR-028).
- **FR-003**: A great-circle line from the station position to each marker, continuous across the
  180° meridian (feature 001, FR-027).
- **FR-004**: A popup per marker listing call, band, mode, UTC time, and position source and
  precision. All text comes from untrusted input and MUST be rendered as text, never as markup
  (feature 001, FR-026 and FR-035).
- **FR-005**: A band legend in frequency order (HR-2). It includes the band filter of feature 003
  when that feature is present.
- **FR-006**: The last-N selector, live updates, snapshots, removals (feature 003) and automatic
  reconnection, unchanged (feature 001, FR-029 – FR-031).
- **FR-007**: The pure frontend modules (`map-state.js`, `geodesic.js`, `connection.js`) and their
  tests MUST keep their behaviour. Only the rendering glue changes.

**New**

- **FR-008**: The base map is configurable by the operator, with the existing precedence (command
  line, then environment, then file). It is either a vector style URL or a raster tile URL template
  with its attribution (the feature-001 `tile_url`/`tile_attribution` settings, kept for
  compatibility). The default is the OpenFreeMap vector style. The service MUST refuse to start when
  both a style and raster tiles are configured, naming the conflict.
- **FR-009**: The base map's attribution MUST always be visible, as its usage terms require. Raster
  tiles have no built-in credit, so the service MUST refuse to start when `tile_url` is set without
  `tile_attribution`. A vector style may carry its own attribution in its sources.
- **FR-010**: Pages MUST open in globe projection by default. Each page MUST offer a toggle between
  globe and flat Mercator, which affects only that page and is remembered in that browser across
  reloads. If storage is unavailable, the choice lasts for the page only.
- **FR-011**: The map library MUST be loaded from a public CDN, pinned to an exact version, with
  Subresource Integrity hashes that are verified independently against the served files, as for
  Leaflet in feature 001. The service binary MUST NOT embed the library.
- **FR-012**: When the browser cannot provide WebGL, the page MUST show a readable message saying
  the map needs WebGL, with a hint to enable hardware acceleration, instead of a blank or broken
  page. No other renderer is provided.

### Key Entities

- **Base map**: the background map (style or tile source plus attribution), chosen by the operator.
- **Projection**: how the Earth is drawn (globe or flat).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every feature-001 frontend behaviour (FR-001 – FR-007) is confirmed in two browsers,
  and every existing frontend unit test passes unchanged.
- **SC-002**: With 2,000 QSOs on the map and "All" selected, panning and zooming run at 30 frames
  per second or more on a mid-range laptop, measured with the browser's performance tools, in both
  globe and flat projection.
- **SC-003**: A live QSO appears on every open page within 2 s, as in feature 001.

## Assumptions

- **Server side**: unchanged apart from the configuration of FR-008. FR-011 needs no server change.
  The WebSocket protocol is unchanged.
- **Branch**: created from `main` after feature 001 was merged. Feature 003 (band filter) is planned
  separately; whichever of 002 and 003 lands second adapts to the other (FR-005, FR-006).

## Clarifications

### Session 2026-10-09

- Q: Default base map? → A: Either, by configuration: a vector style URL or raster tiles. The
  default is the OpenFreeMap vector style. The feature-001 raster settings stay as the raster option
  (this also settles open question 6).
- Q: Projection? → A: Globe by default, with a per-viewer toggle to flat Mercator, remembered in
  the browser like the last-N selector.
- Q: How does the map library reach the browser? → A: From a CDN, pinned to an exact version, with
  independently verified Subresource Integrity hashes, as today.
- Q: What happens in a browser without WebGL? → A: A clear message with a hint to enable hardware
  acceleration; no fallback renderer.
- Q: What is the SC-002 performance target? → A: At least 30 fps while panning and zooming with
  2,000 QSOs, on a mid-range laptop.
