# Data Model: Atmosphere that moves with the globe, and the grayline

There is no server data and no protocol change. The page gets two new browser keys and four
derived values, recomputed from the clock and never stored.

## Stored viewer settings (browser `localStorage`, Principle IV)

| Key | Values | Default | Controls | Read rule |
|-----|--------|---------|----------|-----------|
| `lqm.atmosphere` | `on` / `off` | on | The globe's glow (feature 011, unchanged). | Anything but `off` is on. |
| `lqm.grayline` | `on` / `off` | on | The grayline on both views (FR-009). | Anything but `off` is on. |
| `lqm.sun` | `on` / `off` | off | The Sun symbol on both views (FR-018). | Only `on` is on. |

If the browser refuses storage, the page keeps the default and doesn't fail. Writing ignores
errors. This is the same pattern as `loadAtmosphere` and `saveAtmosphere`.

## Derived values

### Subsolar point

| Field | Type | Rule |
|-------|------|------|
| `lat` | number, degrees | The solar declination, between −23.45 and +23.45. |
| `lon` | number, degrees | From −180 (included) to 180 (excluded). Moves west 15° per hour. |

- **Source:** `subsolarPoint(date)` in `sun.js`. Same equations as `solarElevation` (FR-004,
  FR-011).
- **Input:** the date, in UTC (HR-1).

The antisolar point is `(−lat, lon + 180)`, wrapped to the same range.

### Map light (globe glow)

| Field | Value |
|-------|-------|
| `anchor` | `"viewport"` |
| `position` | `[1.15, azimuthal, polar]`: the Sun's on-screen direction, from behind the globe (research R2, revised) |

- **Source:** `lightFor(subsolar, camera)` in `view.js`.
- **Recomputed:** on every camera `move`, as well as on the refreshes below.

### Grayline tiles

Web Mercator raster tiles (256 px, zoom 0–6, stretched beyond), drawn by the page's `grayline://`
protocol.

| Part | Rule |
|------|------|
| URL | `grayline://<theme>/<minute since 1970>/{z}/{x}/{y}` (`graylineTiles`). A new theme or minute gives new tiles. |
| Pixel colour | The theme's `night` colour. |
| Pixel opacity | `shade(h) × nightOpacity`, where `h` is the Sun's height at the pixel centre (`elevationAt`). `shade` is 0 for `h ≥ 0`, 1 for `h ≤ −12`, and `−h / 12` between. |

- **Source:** `tilePixels(z, x, y, size, subsolar, rgb, nightOpacity)` in `grayline.js`, with the
  subsolar point of the URL's minute.
- **Drawing:** one `raster` layer, `raster-fade-duration: 0`.

### Sun symbol point

A GeoJSON `Point` at the subsolar point, in source `sun`.

## Theme palette additions (`themes.js`)

| Field | Type | Rule |
|-------|------|------|
| `night` | CSS hex colour | The grayline fill colour. |
| `nightOpacity` | number | The full night shade, between 0.2 and 0.7. |

## Refresh

The light, the grayline tiles' URL and the Sun point are recomputed and applied:
- after every style load (theme switch);
- on the existing 60 s theme tick.

The choices only toggle each layer's `visibility`. Moving the map changes none of these values:
the layers are attached to the map.
