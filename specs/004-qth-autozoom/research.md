# Research: QTH Marker, Auto-Zoom and Map Panels

**Feature**: `004-qth-autozoom` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items. The spec was clarified on 2026-10-09 (nine questions in
total). The owner took two more decisions during planning: R2 (where the entity extents come from)
and R3 (how the house icon is drawn).

## R1. Where the QTH comes from (FR-001–FR-003, FR-025)

- **Decision**: the server sends `hello.station`, built once at startup from the configuration:
  - the callsign;
  - the locator;
  - the position, resolved by the same rules as `enrich::station::config_station`: explicit
    coordinates win, otherwise the locator.

  The page then picks the QTH with a pure function, `currentQth(helloStation, state)`:
  1. the configured position, when present;
  2. otherwise the `station` position of the newest QSO **in the page's state** that has one;
  3. otherwise none.
- **Rationale**: only the page knows which QSOs it holds, after its limit and the band filter.
  Taking the QTH from those QSOs keeps the house consistent with the lines that start there.
- **Alternatives**:
  - the server tracks the "latest station position" and pushes it: this adds a message, and the
    result can disagree with the page's window;
  - each line keeps its own QSO's station position, as in feature 001: rejected, because the owner
    chose one house, and every line starts there.

## R2. Entity extents (FR-010, SC-003)

- **Decision** (owner, 2026-10-09): **generate the extents from Natural Earth**:
  - **Source data**: Natural Earth **v5.1.2** GeoJSON at 1:10m, which is public domain:
    - `ne_10m_admin_0_map_subunits.geojson` (13.7 MB);
    - `ne_10m_admin_1_states_provinces.geojson` (40.7 MB).

    Both were fetched from `raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/`
    on 2026-10-09.
  - **Generator**: `tools/entity_extents.py`, a dev-only script that uses only the Python
    standard library. It downloads the two files into a cache directory and reads a hand-written
    mapping, `tools/entity_extents.toml`. It then writes `src/web/entity_extents.json`, which is
    committed.
  - **Mapping**: each cty.dat primary prefix (without the `*` that marks WAE entities) maps to one
    or more selectors:
    - an admin-0 subunit (`SU_A3`) or admin-1 unit (`iso_3166_2`);
    - whole countries with exclusions, for example `K` = the USA without `US-AK` and `US-HI`;
    - an optional clip box, for example European Russia.

    Examples: `IS` → `IT-88` (Sardegna); `IT9` → `IT-82` (Sicily); `I` → Italy without `IT-88`
    and `IT-82`; `VK` → Australia without its external territories; `K` → the lower 48 states.
  - **Output**: `{"source": "Natural Earth v5.1.2 (public domain)", "extents": {"IS": [w, s, e, n], ...}}`, in degrees, with
    longitudes in [-180, 180]. When an entity crosses the antimeridian, `w > e`, for example
    `3D2` for Fiji, or `UA0` for Asiatic Russia. The box is the minimal
    longitude span over all of the entity's vertices (the largest-gap method).
  - **Runtime**: the service embeds the JSON with `include_str!` and parses it once at startup with
    `serde_json`, an existing dependency. A malformed table is a startup error. A unit test parses
    the embedded file and checks that every box has `s < n`.
  - **Coverage**: the script lists every cty.dat entity that has no mapping. The target is every
    entity that Natural Earth represents. Tiny islands and rocks that Natural Earth lacks fall back
    to the 500 km radius (FR-010).
- **Rationale**: the data is reproducible, reviewable and licence-free, it has no runtime
  download, and adding an entity means adding one mapping line.
- **Cost**: writing a mapping of roughly 340 entries is a one-off effort. The script is not part of
  the build; it is rerun only when the mapping changes. See TR-005.
- **Alternatives**:
  - a hand-curated table with no generator: its accuracy can't be reviewed;
  - a subset of about 60 entities: too many entities would get only the radius;
  - operator configuration: rejected in clarification.

## R3. House icon (FR-001, FR-005)

- **Decision** (owner, 2026-10-09): an inline SVG house of about 28 px, with a dark outline and a
  light fill so that it reads on light and dark base maps:
  1. it is drawn once into a canvas;
  2. it is registered with `map.addImage('qth-house', …)`;
  3. it is rendered by a `symbol` layer, `qth`, with `icon-allow-overlap` and
     `icon-ignore-placement`.

  Layer order, bottom to top: `qso-lines` → `qth` → `qso-markers`. Remote markers therefore stay
  above the house and stay clickable (FR-005).
- **Rationale**: it looks the same on every operating system. That includes a Raspberry Pi kiosk
  without a colour emoji font, where 🏠 shows as an empty box. The SVG is a constant string, so it
  needs no network and no font.
- **Alternatives**:
  - a DOM `maplibregl.Marker`: it sits above the canvas and would block clicks on remote markers
    near the house;
  - an emoji glyph in a symbol layer: it needs a glyph server, and its look depends on the OS font.

## R4. Lines from the QTH, black (FR-004)

- **Decision**: `lineFeatures(markers, qth)` builds one great circle from the QTH to every marker.
  It returns no lines when `qth` is null.
- **Line style**: `line-color: #000`, `line-opacity: 0.6`, `line-width: 1.5`. The opacity is the
  slight transparency allowed by the spec's Assumptions.
- **Other changes**: the per-line `color` property is dropped. The feature-001 rule "line from the
  most recent QSO's station position" no longer applies, so the feature-002 tests on
  `lineFeatures` are updated.
- **Rationale**: one origin, one colour. The great-circle code (`geodesic.js`) is reused unchanged.

## R5. Auto-zoom geometry (FR-008–FR-010, FR-013, FR-016)

All of it lives in a new pure module, `autozoom-geo.js`:

- **Framed points**: the QTH and every marker. With no marker, the frame is:
  - the entity box from `hello.entity`;
  - otherwise a 500 km circle around the QTH, approximated by its bounding box;
  - otherwise the whole world.
- **Flat bounds**: `[west, south, east, north]` with the minimal longitude span. Each point's
  longitude is unwrapped next to the span's west edge, so a set that crosses the antimeridian gets
  `east > 180`, which MapLibre `fitBounds` accepts. Without this, a Europe-to-New Zealand set would
  be framed the long way round.
- **Globe fit**: the smallest spherical cap that contains every framed point. It is computed with
  an iterative algorithm over unit vectors (Welzl-style on the sphere for up to 2,000 points, or
  centroid refinement). The points fit when `capRadius + marginDeg ≤ 90°`. The margin turns the
  padding into degrees at the current globe size. The cap centre is the camera centre (FR-016,
  clarified "best centre"), and the zoom comes from `cameraForBounds` on the cap's flat bounds.
- **Zoom cap**: `maxZoom: 12` on every fit (city scale, FR-013).
- **Verification**: the pure functions are covered by `node --test`. The behaviour of
  `cameraForBounds` under globe projection in MapLibre 5.24.0 is checked in a spike task, which
  runs before the glue that depends on it. If it falls short, the zoom is computed from the cap
  radius and the viewport size instead (`zoomForCap`, also pure).
- **Spike result** (T029, 2026-10-09, from the MapLibre 5.24.0 source rather than a browser run):
  - `cameraForBounds` and `fitBounds` support the globe. `GlobeCameraHelper` delegates to the
    vertical-perspective helper. That helper takes the flat-map centre of the bounds and then
    solves the zoom so that the box's corners and edge midpoints fall inside the padded clip
    space.
  - When no zoom satisfies them, it returns `undefined`, with a console warning.
  - **Decision**: on the globe, the glue builds a box centred on the cap, takes its zoom from
    `cameraForBounds`, and eases to the cap centre with the same padding. `zoomForCap` covers the
    `undefined` case.
  - **Test-set correction**: IS + JA + W6 do fit on one side of the globe, seen from over the North
    Pole (cap radius about 56°). The "cannot fit" example is now IS + ZL + PY + JA, which needs more
    than 90°.

## R6. Obstructed area and padding (FR-009)

- **Decision**: `paddingFor(mapRect, rects, minFree)`, a pure function:
  1. each visible panel rectangle (legend, last-QSO box, limit selector, control groups,
     attribution, visible banners) is assigned to the map edge it is nearest to;
  2. the padding on each side is the largest depth that panels reach from that edge, plus 12 px;
  3. if the free area left is smaller than `minFree` (240×160 px), the function returns a uniform
     12 px padding instead (the edge case).

  The glue measures the panels with `getBoundingClientRect()` on every fit.
- **Rationale**: this is generic, so a future panel is covered by adding its selector to one list.
- **Alternatives**: fixed paddings per layout break when panels resize, for example a long legend
  or a narrow window.

## R7. Auto-zoom controller (FR-007, FR-008, FR-011, FR-012, FR-012a, FR-014, FR-016–FR-018)

- **Decision**: `autozoom.js`, a pure state machine with an injected clock:
  - **State**: `{ enabled, popupOpen, pending, lastFitAt, autoFlat }`.
  - **Events**:
    - `change()`: data, limit, snapshot, QTH, panel or resize;
    - `userMove()`: a MapLibre `movestart` carrying an `originalEvent`, which covers drag, wheel,
      keyboard and the navigation buttons;
    - `toggle()`;
    - `popupOpened()` / `popupClosed()`;
    - `tick(now)`.
  - **Output**: actions such as `{fit: true}` or `{projection: 'mercator' | 'globe'}`.
  - **Coalescing**: changes set `pending`. A fit runs when `now - lastFitAt ≥ 1000 ms` and no popup
    is open, so there is at most one refit per second (FR-014, SC-005).
  - **Page-made moves**: they have no `originalEvent`, so they never count as user moves (FR-011).
  - **Projection**: `effectiveProjection(preference, fitsGlobe, enabled)`.
    - With a globe preference and auto-zoom on, the result is `mercator` when the points don't fit
      and `globe` when they do.
    - Otherwise the result is the preference.

    The saved preference (`lqm.projection`) is written only by the projection button (FR-018).
- **Popup timer**: while auto-zoom is on, an open popup closes after 30 s with no `pointerdown`,
  `pointermove`, `wheel` or `keydown` inside it. The timer restarts on each such event (FR-012a).
- **Rationale**: every rule in the spec becomes a testable transition, and `app.js` only wires
  MapLibre events and timers to it.

## R8. Last-QSO box (FR-019–FR-022a)

- **Decision**: `recent.js`, pure:
  - `recentQsos(state, 5)` returns the last five of `visible(state)`, newest first. `visible` is the
    limit window sorted by `sort_key` (QSO time), so the box follows the limit selector and the band
    filter, and a late QSO enters it only when it is among the five newest.
  - `rowView(qso)` returns `{call, band, mode, time, markerKey}`. `markerKey` is null when the QSO
    has no remote position.
- **Glue**: rows are built with `textContent` (FR-022). A row with a `markerKey` gets a button role
  and a pointer cursor. A click opens that marker's popup, and the page pans with `panBy` only when
  the marker's screen point is outside the free area (R6).
- **Placement**: bottom-left. The MapLibre scale control sits in the bottom-left corner, so the box
  is stacked above it.

## R9. Scale bar (FR-023, FR-024)

- **Decision**: MapLibre's `ScaleControl({unit: 'metric', maxWidth: 120})` at bottom-left. It shows
  round metric values and updates on every move.
- **Visibility**: the glue adds the control when the effective projection is `mercator` and removes
  it on `globe`. A pure `scaleVisible(projection)` makes that rule testable.
- **SC-008**: MapLibre measures the scale at the map centre. It is checked manually against the
  great-circle distance between two points on screen (quickstart §6).

## R10. Buttons and stored preferences (FR-007, FR-021)

- **Buttons**: two buttons in the top-left control group, next to Navigation and Projection:
  - **Auto-zoom** has `aria-pressed` set to on or off;
  - **Last QSOs** shows and hides the box.

  Both have a `title` and an `aria-label`, so they work from the keyboard and have accessible
  names.
- **Stored preferences**: in `localStorage`, under `lqm.autozoom` (`on`/`off`) and `lqm.recent`
  (`shown`/`hidden`). Helpers in `view.js` follow the projection preference pattern. Missing or
  unknown values and a storage that throws all fall back to the defaults (on, shown).

## R11. JSON error responses (FR-027, API-3)

- **Decision**: one helper, `web::json_error(status, code, message) -> Response`, which returns
  `{"error": code, "message": message}` with `Content-Type: application/json`. It is used by:
  - **unknown paths**: `Router::fallback` returns `404 not_found`;
  - **wrong methods**: `Router::method_not_allowed_fallback` (axum 0.8.9) returns
    `405 method_not_allowed`;
  - **failed upgrades on `/ws`**: the handler takes `Result<WebSocketUpgrade,
    WebSocketUpgradeRejection>` and maps a rejection to its own status (`400`, `405` or `426`)
    with `websocket_upgrade_required`.
- **Rationale**: no new crate, and every error path goes through one function that has its own
  tests.

## R12. Protocol version (FR-025, API-4)

- **Decision**: the protocol version becomes **3**.
  - `hello` gains `station` and `entity`. Both can be null.
  - Every other message is unchanged.
- **Rationale**: adding fields is not a breaking change under API-4. The bump is still made, as the
  spec's Assumptions ask, so that a page can tell whether QTH data is guaranteed to be present.
