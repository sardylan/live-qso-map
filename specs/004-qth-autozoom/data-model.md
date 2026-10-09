# Data Model: QTH Marker, Auto-Zoom and Map Panels

**Feature**: `004-qth-autozoom` | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Server side (Rust)

### `StationView` (new, `src/web/protocol.rs`)

This is sent once in `hello.station`. It is built at startup from `Config.station`.

| Field | Type | Rule |
|-------|------|------|
| `callsign` | `Option<String>` | `station.callsign`, trimmed and upper-cased (HR-5) |
| `locator` | `Option<String>` | `station.locator` as configured |
| `position` | `Option<Position>` | the explicit coordinates, otherwise the locator centre (same rules as `config_station`); the source is `config` |

When all three fields are `None`, `hello.station` is `null`.

### `EntityView` (new, `src/web/protocol.rs`)

This is sent once in `hello.entity`. It is `null` when there is no configured callsign, no country
data, no matching entity, or no extent for that entity.

| Field | Type | Rule |
|-------|------|------|
| `prefix` | `String` | the cty.dat primary prefix, without `*` |
| `name` | `String` | the cty.dat entity name |
| `bounds` | `[f64; 4]` | `[west, south, east, north]`, in degrees; `west > east` means it crosses the antimeridian |

### `ExtentTable` (new, `src/entity_extents.rs`)

- **Source**: it is parsed once at startup from the embedded `src/web/entity_extents.json`.
- **Shape**: `HashMap<String, [f64; 4]>`, keyed by prefix.
- **Validation**:
  - every box has `-90 ≤ s < n ≤ 90` and `-180 ≤ w, e ≤ 180`;
  - a bad entry is a startup `Error::Config { setting: "entity_extents", .. }`, which cannot happen
    with the committed file, as a unit test checks.

### `CountryData` (changed, `src/enrich/cty.rs`)

- `Entity` gains `prefix: String`, the primary prefix column without `*`.
- A new method, `entity_of(call) -> Option<(&str prefix, &str name)>`, resolves an entity with the
  same longest-prefix and exact-call rules as `lookup`.

### `ServerMessage::Hello` (changed)

- `protocol` is 3.
- New fields: `station: Option<StationView>` and `entity: Option<EntityView>`.
- **Signature**: `ServerMessage::hello(map, filter, station: &HelloStation)`. `HelloStation` holds
  the two precomputed views and is shared through `AppState`.

## Frontend (pure ES modules)

### QTH (`qth.js`, new)

`currentQth(helloStation, state) -> Qth | null`

```text
Qth = { lat, lon, callsign|null, locator|null, source, precision, origin: 'config'|'qso' }
```

1. When `helloStation.position` exists, the QTH comes from it, with `origin: 'config'`.
2. Otherwise it comes from the newest QSO in `state` (by `sort_key`) that has a `station`, with
   `origin: 'qso'`. Its callsign is that QSO's `station_call`, and its locator is its
   `adif.MY_GRIDSQUARE`.
3. Otherwise the result is `null`.

`houseFeature(qth)` returns a GeoJSON FeatureCollection with zero features or one Point.

### Lines (`layers.js`, changed)

`lineFeatures(markers, qth)` returns one LineString per marker, a great circle from `qth`. It
returns an empty collection when `qth` is null. The `color` property is dropped, because the line
colour is a fixed paint value.

### Framing (`autozoom-geo.js`, new)

| Function | Result |
|----------|--------|
| `framedPoints(qth, markers)` | `[[lat, lon], …]` |
| `fallbackFrame(qth, entity, radiusKm = 500)` | the entity bounds, else the circle bounds, else `null` (whole world) |
| `flatBounds(points)` | `[[w, s], [e, n]]`, the minimal longitude span, with `e` possibly above 180 |
| `boundingCap(points)` | `{center: [lat, lon], radiusDeg}` |
| `fitsGlobe(cap, marginDeg)` | `radiusDeg + marginDeg ≤ 90` |
| `globeZoomForCap(cap, {width, height, padding})` | the globe zoom that shows the whole cap in the free area, or `null` when none can (R5, browser finding) |
| `capScreenRadius(cap, zoom, {height})` | px from the centre to the cap edge on the globe, or `null` behind the limb (used by the tests) |

### Padding (`autozoom-geo.js`)

`paddingFor(mapRect, panelRects, {gap: 12, minFree: [240, 160]})` returns
`{top, right, bottom, left}` in px.

### Auto-zoom controller (`autozoom.js`, new)

The state:

```text
{ enabled: bool, popupOpen: bool, pending: bool, lastFitAt: ms|-Infinity, autoFlat: bool }
```

The transitions. Each returns a list of actions.

| Event | Effect |
|-------|--------|
| `toggle()` | `enabled = !enabled`. When it becomes on, `pending = true` and the action is `fitNow`. When it becomes off, there is no projection action, and `autoFlat` is kept, so turning auto-zoom back on can return to the globe when everything fits (FR-017). |
| `change()` | when enabled, `pending = true` |
| `userMove()` | when enabled, `enabled = false`, `pending = false`, and the action is `saveOff` only; the projection is unchanged (FR-017) |
| `popupOpened()` | `popupOpen = true` |
| `popupClosed()` | `popupOpen = false`; when enabled, `pending = true` (the catch-up refit) |
| `tick(now)` | when `enabled && pending && !popupOpen && now - lastFitAt ≥ 1000`, the action is `fit`; then `pending = false` and `lastFitAt = now` |
| `fitted(fitsGlobe, preference)` | sets `autoFlat = preference === 'globe' && !fitsGlobe`, and returns the projection to apply |

**Invariant**: `autoFlat` is true only while the preference is the globe and auto-zoom switched the
view to flat. It is cleared by the projection button, or by a fit that fits the globe.
Nothing here writes `lqm.projection` (FR-018).

### Last-QSO box (`recent.js`, new)

- `recentQsos(state, n = 5)` returns the last `n` of `visible(state)`, reversed (newest first).
- `rowView(qso)` returns `{call, band, mode, time, markerKey|null}`:
  - `mode` is `MODE/SUBMODE` when a submode is present;
  - `time` is UTC (HR-1);
  - `markerKey` is `"lat,lon"`, the same key `markers()` uses.

### Preferences (`view.js`, extended)

| Function | Stored key | Values | Default |
|----------|------------|--------|---------|
| `loadAutoZoom` / `saveAutoZoom` | `lqm.autozoom` | `on` / `off` | on |
| `loadRecent` / `saveRecent` | `lqm.recent` | `shown` / `hidden` | shown |
| `scaleVisible(projection)` | — | — | `projection === 'mercator'` |

## Generator (dev-only, `tools/`)

### `tools/entity_extents.toml`

```toml
[IS]          # Sardinia
admin0 = ["ITD"]

[I]           # Italy: Natural Earth splits Sardinia and Sicily into their own subunits
admin0 = ["ITX"]

[TA]          # Asiatic Turkey
admin1_of = ["TUR"]
exclude_admin1 = ["TR-22", "TR-39", "TR-59"]

[UA]          # European Russia (Natural Earth's western "Russia" subunit)
admin0 = ["RUE"]
clip = [19.0, 40.0, 70.0, 79.5]
```

Selector keys:

- `admin0`: a list of `SU_A3` codes;
- `admin1`: a list of `iso_3166_2` codes;
- `admin1_of`: every admin-1 unit of these `adm0_a3` countries;
- `exclude_admin1`: admin-1 codes to remove (with `admin1_of`);
- `clip`: `[w, s, e, n]`;
- `note`: free text.

Entities that Natural Earth cannot represent go in an `[unmapped]` table, mapping each prefix to
a reason. The generator fails when an entity is neither mapped nor listed.

### `src/web/entity_extents.json` (generated, committed)

```json
{ "source": "Natural Earth v5.1.2 (public domain)", "extents": { "IS": [8.13, 38.86, 9.83, 41.31] } }
```

The file carries no timestamp, so regenerating it from the same mapping gives the same bytes. Keys
are sorted and values are rounded to 2 decimals.
