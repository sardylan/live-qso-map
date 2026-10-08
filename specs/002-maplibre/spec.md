# Feature Specification: MapLibre Map Rendering

**Feature Branch**: `002-maplibre`
**Created**: 2026-10-09
**Status**: Draft. Open decisions are listed under Clarifications; complete them with
`/speckit-clarify` before `/speckit-plan`.
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
every item of FR-001 to FR-010 by hand in two browsers. Every frontend unit test from feature 001
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

1. **Given** no base-map setting, **When** a page opens, **Then** the default base map is shown, with
   its attribution [NEEDS CLARIFICATION: which default base map: a free vector style (e.g.
   OpenFreeMap, no key), the current OpenStreetMap raster tiles, or a setting that accepts either?].
2. **Given** an operator-supplied base map, **When** a page opens, **Then** that base map is shown.

---

### User Story 3 - Globe view (Priority: P3)

At low zoom the viewer sees the Earth as a globe. Long-distance paths look like the real
great-circle arcs, and polar paths are no longer distorted.

**Why this priority**: This is the main visual gain of the new renderer for a DX/contest map. It is
not required for parity.

**Independent Test**: Open a page, zoom out, and check the projection and the path rendering
against acceptance scenario 1.

**Acceptance Scenarios**:

1. **Given** a page at world zoom, **When** QSOs on several continents are shown, **Then** the
   projection is [NEEDS CLARIFICATION: globe only, flat only, or globe with a per-viewer toggle to
   flat?].

---

### Edge Cases

- **Browser without WebGL**: [the behaviour is to be decided during clarification. A suggested
  default is a clear message that the map needs WebGL. Falling back to the old renderer is not
  planned.]
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
  line, then environment, then file).
- **FR-009**: The base map's attribution MUST always be visible, as its usage terms require.
- **FR-010**: Projection: see User Story 3 (pending clarification).
- **FR-011**: The map library MUST be pinned to an exact version and integrity-checked [NEEDS
  CLARIFICATION: delivered from a CDN with Subresource Integrity as today, or embedded in the
  service binary?].

### Key Entities

- **Base map**: the background map (style or tile source plus attribution), chosen by the operator.
- **Projection**: how the Earth is drawn (globe or flat).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every feature-001 frontend behaviour (FR-001 – FR-007) is confirmed in two browsers,
  and every existing frontend unit test passes unchanged.
- **SC-002**: With 2,000 QSOs on the map and "All" selected, the page stays interactive (pan and
  zoom without visible stalls) on a mid-range laptop. [The exact frame-rate target is to be set
  during clarification.]
- **SC-003**: A live QSO appears on every open page within 2 s, as in feature 001.

## Assumptions

- **Server side**: unchanged apart from the configuration of FR-008 and whatever FR-011 requires.
  The WebSocket protocol is unchanged.
- **Branch**: created from `main` after feature 001 was merged. Feature 003 (band filter) is planned
  separately; whichever of 002 and 003 lands second adapts to the other (FR-005, FR-006).

## Clarifications

Open questions, to settle before planning. Each has a suggested default.

| # | Question | Suggested default |
|---|----------|-------------------|
| 1 | Default base map | A free vector style without an API key (OpenFreeMap), configurable to any style URL |
| 2 | Projection | Globe at low zoom, with a per-viewer toggle to flat |
| 3 | Library delivery | CDN, pinned exact version, Subresource Integrity |
| 4 | No-WebGL behaviour | A clear message; no fallback renderer |
| 5 | SC-002 frame-rate target | 30 fps while panning with 2,000 QSOs |
| 6 | Keep the `tile_url`/`tile_attribution` settings? | Keep them as a raster alternative, or deprecate them in favour of a style URL |
