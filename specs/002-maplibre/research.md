# Research: MapLibre Map Rendering

**Feature**: `002-maplibre` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items: the spec was clarified on 2026-10-09 (5 questions). One more
owner decision was taken during planning (R1).

## R1. MapLibre version and delivery (FR-011)

- **Decision** (owner, 2026-10-09): **MapLibre GL JS 5.24.0**, the last 5.x release (2026-04-23),
  from cdnjs. It is a single UMD bundle (`maplibre-gl.js`, 1,056,837 bytes, plus `maplibre-gl.css`,
  70,024 bytes) with SHA-512 Subresource Integrity:

  | File | SRI |
  |------|-----|
  | `maplibre-gl.js` | `sha512-rOFgoUgWaVi69JW1dzBu6PWF9xKW9rxYfY5u2SexFABHhWk0qUWkJoWNz4UuZRMvSXsNZG66+tU5B+HlwWF5kA==` |
  | `maplibre-gl.css` | `sha512-KIMsMWIdnoG9OwZa+oaIafmbDonqck1UCmq+/zcUi2aaZ97N9QE6TqpTM+n36EO40HGh64hU/375zxtjx7WTyQ==` |

  Both hashes were published by cdnjs **and** recomputed from the downloaded files on 2026-10-09;
  they match.
- **API check**: the 5.24.0 bundle contains every API the design uses: `setProjection` with
  `"globe"`, `Popup.setDOMContent`, `circle-sort-key`, `renderWorldCopies` and `AttributionControl`.
- **Rationale**: MapLibre 6.x (current: 6.13.0, released 2026-10-06) is ES-module only. Its entry
  module imports a shared chunk and starts a **module worker**, and Subresource Integrity cannot
  cover a worker. Version 5.24.0 keeps full SRI, as the clarified FR-011 requires.
- **Cost**: the 5.x line no longer receives updates. Moving to 6.x later only touches `index.html`
  and the small glue in `app.js`, because all decisions live in pure modules.
- **Alternatives**: 6.11.2 with partial SRI through an import map (the worker stays unverified);
  6.11.2 embedded in the binary (reverses the clarified CDN decision).

## R2. Base map: vector style or raster (FR-008, FR-009)

- **Decision**: two mutually exclusive configurations.
  - **Vector style**: `map.style_url` plus `map.style_attribution`. The default is OpenFreeMap
    *Liberty*, `https://tiles.openfreemap.org/styles/liberty` (checked 2026-10-09: style version 8,
    an `openmaptiles` vector source and a `ne2_shaded` raster source).
  - **Raster tiles**: `map.tile_url` plus `map.tile_attribution`, the feature-001 settings, now
    without a default.

  When neither is set, the OpenFreeMap style applies. When both a style URL and a tile URL are set,
  the service refuses to start, naming the conflict.
- **Attribution**: the OpenFreeMap style has **no** attribution in its sources, so the page would
  show none (FR-009). The default `style_attribution` is therefore OpenFreeMap's required credit:
  `<a href="https://openfreemap.org" target="_blank">OpenFreeMap</a> © <a
  href="https://www.openmaptiles.org/" target="_blank">OpenMapTiles</a> Data from <a
  href="https://www.openstreetmap.org/copyright" target="_blank">OpenStreetMap</a>`. The attribution
  control is created non-compact, so it is always visible.
- **Raster attribution is mandatory** (analyze C1): raster tiles carry no credit of their own, so
  `tile_url` without `tile_attribution` is a startup error. A custom vector style may carry its
  credit in its sources, which MapLibre shows automatically.
- **Trust**: attribution strings are HTML from the **operator's** configuration, not from UDP input,
  so they are rendered as HTML, as Leaflet did. QSO data stays text-only (FR-004).

## R3. Rendering markers and lines (FR-001 – FR-004, SC-002)

- **Decision**: render with GPU layers fed by GeoJSON, not with HTML markers.
  - Source `qso-lines` (LineStrings) drawn by a `line` layer, coloured by feature property.
  - Source `qso-markers` (Points) drawn by a `circle` layer with a white stroke, coloured by
    feature property, and `circle-sort-key: zRank` so the newest is on top (FR-002).
  - A new pure module, `layers.js`, builds both FeatureCollections from `markers(state)`. Line
    coordinates come from `greatCircle`, with longitudes unwrapped.
  - `app.js` calls `setData` only when `diffLayers` reports a change.
- **Popups**: a click on the circle layer looks up the marker by its `key` property and opens a
  `maplibregl.Popup` with `setDOMContent(popupContent(marker))`. The DOM is built with `textContent`
  only (FR-004).
- **Antimeridian**: unwrapped longitudes beyond ±180° draw continuously in Mercator with world copies
  and on the globe. This is checked manually in the quickstart.
- **Rationale**: 2,000 HTML markers means 2,000 DOM nodes moved on every frame, which won't hold
  30 fps (SC-002). Circle layers render in one draw call.
- **Alternatives**: `maplibregl.Marker` (DOM) per QSO, which is simplest but slow at 2,000; a symbol
  layer with icons, which needs generated sprites for every band colour.

## R4. Projection (FR-010)

- **Decision**: the page calls `map.setProjection({ type })` after `load`. The default is `globe`.
  A small custom control toggles between `globe` and `mercator`. The choice is stored in
  `localStorage` (`lqm.projection`), with the same try/catch pattern as `connection.js`. If storage
  is unavailable, the choice lasts only for the page. The pure helpers (`loadProjection`,
  `saveProjection`, `nextProjection`) live in a new module, `view.js`.
- **Why `localStorage`, not `sessionStorage`**: the projection is a viewer preference across
  sessions. The last-N selector stays in `sessionStorage`, as in feature 001.

## R5. No WebGL (FR-012)

- **Decision**: before creating the map, `view.js::hasWebGL(createCanvas)` tries `webgl2`, then
  `webgl`, on an injected canvas factory (testable in Node). When both fail, `app.js` shows the
  FR-012 message in place of the map container and stops. The WebSocket isn't opened, because
  nothing could be drawn. MapLibre's `error` event is also logged to the console.

- **Base map unreachable** (analyze E1): map `error` events whose source is not a QSO overlay
  (`view.js::isBasemapError`) show the status banner "Base map unavailable; QSOs are still shown."
  The GeoJSON overlays don't depend on the base map, so they keep rendering.

## R6. Protocol change

- **Decision**: `hello.tiles` (feature 001) is replaced by `hello.basemap`:
  - `{"kind":"style","url":"…","attribution":"…"}`, or
  - `{"kind":"raster","url":"…","attribution":"…"}`.

  The protocol version goes from **1 to 2**, because a feature-001 page can't use the new shape.
  The page and the service always ship together, so no compatibility path is needed.
- **New pure helper**: `view.js::buildStyle(basemap)` returns the style URL for `style`, or a
  version-8 raster style object for `raster`.

## R7. Unchanged

- The WebSocket session, snapshots, upserts, the last-N selector and reconnection.
- `map-state.js`, `geodesic.js` and `connection.js`, with their tests (FR-007).
- The band legend remains a DOM overlay in frequency order (HR-2).
- No new Rust crates.
