# Data Model: MapLibre Map Rendering

**Feature**: `002-maplibre` | **Date**: 2026-10-09

The entities from feature 001 apply unless listed here.

## BaseMap (server configuration → `hello.basemap`)

```text
BaseMap = Style  { url: String, attribution: String }   // vector style JSON
        | Raster { url: String, attribution: String }   // tile URL template {z}/{x}/{y}
```

| Rule | Source |
|------|--------|
| `style_url` and `tile_url` both set: startup error `Config { setting: "map.style_url", … }` | FR-008 |
| Neither set: `Style { OpenFreeMap Liberty, OpenFreeMap attribution }` | FR-008, research R2 |
| `tile_url` set: `Raster { tile_url, tile_attribution or "" }` | FR-008 |
| `style_url` set: `Style { style_url, style_attribution or "" }` | FR-008 |

## Projection (per page)

```text
Projection = "globe" | "mercator"      // default "globe"; localStorage key "lqm.projection"
```

## View helpers (`view.js`, pure)

| Function | Contract |
|----------|----------|
| `buildStyle(basemap)` | `style` → `basemap.url`; `raster` → `{version: 8, sources: {basemap: {type: "raster", tiles: [url], tileSize: 256}}, layers: [{id: "basemap", type: "raster", source: "basemap"}]}` |
| `loadProjection(storage)` | `"globe"` unless storage holds `"mercator"`; never throws |
| `saveProjection(storage, p)` | swallows storage errors |
| `nextProjection(p)` | `globe` ↔ `mercator` |
| `hasWebGL(createCanvas)` | `true` when `getContext("webgl2")` or `getContext("webgl")` returns a context; `false` on null or a throw |

## Layer data (`layers.js`, pure)

| Function | Output |
|----------|--------|
| `markerFeatures(markers)` | `FeatureCollection` of `Point`s: `properties = {key, color, zRank}`, coordinates `[lon, lat]` |
| `lineFeatures(markers)` | `FeatureCollection` of `LineString`s, one per marker with a station: coordinates from `greatCircle` as `[lon, lat]` (unwrapped), `properties = {key, color, zRank}` |

Feature order follows `zRank`, ascending.
