# Contract: page module additions

All new functions are pure, and are unit-tested in `web/*.test.js` (`node --test`).

## `sun.js`

```js
/** The point where the Sun is overhead at `date` (UTC): { lat, lon } in degrees. */
export function subsolarPoint(date)
```

- `lat` is the declination that `solarElevation` uses, and `lon` is in the range −180 (included)
  to 180 (excluded).
- `solarElevation(date, p.lat, p.lon) ≥ 89.5` for `p = subsolarPoint(date)`.
- `solarElevation` is unchanged (the feature 011 tests stay green).

## `geodesic.js`

```js
/** The point `distanceDeg` along the great circle from `[lat, lon]` at `bearingDeg`: [lat, lon]. */
export function destination(point, bearingDeg, distanceDeg)
```

- A distance of 0 gives the start point.
- 90° due north from the equator gives the North Pole.
- 180° from any point gives its antipode.

## `grayline.js` (new)

```js
/** The night shade (0–1) at a Sun height: 0 at and above 0°, 1 from −12°, linear between. */
export function shade(elevation)
/** The Sun's height (degrees) at lat/lon with the Sun overhead at `subsolar`. */
export function elevationAt(subsolar, lat, lon)
/** [lat, lon] of pixel (px, py) of Web Mercator tile z/x/y, `size` px wide. */
export function tileLatLon(z, x, y, size, px, py)
/** RGBA pixels of grayline tile z/x/y: colour `rgb`, opacity shade × nightOpacity. */
export function tilePixels(z, x, y, size, subsolar, rgb, nightOpacity)  // → Uint8ClampedArray
/** The raster source's tiles for a theme and the minute of `date`. */
export function graylineTiles(theme, date)                            // → ['grayline://…/{z}/{x}/{y}']
export function parseGraylineUrl(url)                                  // → { theme, minute, z, x, y } | null
export function rgbOf(hex)                                             // → [r, g, b]
/** The style layer id to insert the grayline before: first boundary, else first symbol. */
export function graylineBeforeId(styleLayers)                          // → string | undefined
```

- **`shade`:** it rises from 0 to 1 over 0° to −12°, by less than 0.05 per 0.25°. It is 0.5 at
  −6°.
- **`elevationAt`:** within 0.1° of `solarElevation` for 12 date-times × a 20°/30° grid.
- **`tilePixels`:** every pixel has the colour `rgb`. Its alpha is within 1 of
  `round(shade(elevationAt(pixel centre)) × nightOpacity × 255)`, sampled on tiles 0/0/0, 2/1/2
  and 3/7/3. Deep night is at full opacity, and full day is clear.
- **`graylineBeforeId`:** with the `liberty`, `fiord` and `dark` fixtures it gives
  `boundary_3`, `boundary_state` and `boundary_state`. With no boundary layer it gives the first
  symbol layer; with neither, `undefined`.

## `view.js`

```js
export function lightFor(subsolar, camera)   // camera: { lat, lng, bearing, pitch }
                                             // → { anchor: 'viewport', position: [1.15, az, polar] }
export function loadGrayline(storage)        // → boolean, default true
export function saveGrayline(storage, on)
export function loadSunMarker(storage)       // → boolean, default false
export function saveSunMarker(storage, on)
```

- **`lightFor`:** replay MapLibre 5.24.0's `getSunPos` over a grid of cameras and five subsolar
  points. The light always comes from behind the globe (`z ≤ −0.9`: no haze, FR-019). Its
  on-screen direction is the Sun's within 1° whenever the Sun's on-screen part is at least 0.3.
  With the Sun straight in front or behind it is `(0, 0, −1)`, an even rim.
- **Load and save:** they follow the `loadAtmosphere` and `saveAtmosphere` rules: unknown values
  and storage that throws fall back to the default.

## `themes.js`

Every palette has `night` (hex) and `nightOpacity` (between 0.2 and 0.7).

## `app.js` and `index.html` (covered by the browser test)

- **`index.html`:** the Map group gets two checkboxes after `#atmosphere`: `#grayline`
  "Grayline (day and night)" and `#sun-marker` "Sun position".
- **`addOverlays()`:**
  - adds the `raster` source and layer `grayline` (tiles `graylineTiles(theme, now)`, max zoom
    6) before `graylineBeforeId(...)` of the base map's layers, or before `qso-lines`;
  - adds source and layer `sun` (symbol, image `sun-icon`) just below `qso-lines`.
- **`grayline://` protocol:** registered once with `maplibregl.addProtocol`. It returns
  `{ data: ImageBitmap }` of `tilePixels` for the URL's theme and minute.
- **`applySun()`:** caches the Sun position and sets the light. It calls `setTiles` on
  `grayline` only when the theme or minute changed, sets the `sun` point, and sets both layers'
  `visibility`. It is called from `onStyleLoad()`, the
  60 s theme tick and both checkboxes.
- **Handlers:** one `move` handler, `applyLight`, which only computes and sets the light. No
  `zoom` or `render` handler is added, and no click or hover handler is bound to `sun` or
  `grayline`.
