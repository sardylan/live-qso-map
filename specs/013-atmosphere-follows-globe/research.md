# Research: Atmosphere that moves with the globe, and the grayline

All findings are against MapLibre GL JS 5.24.0, the version the page loads (feature 002). The
source was read from `maplibre-gl@5.24.0/dist/maplibre-gl-dev.js`.

## R1: Why the glow stays fixed on screen (bugs A1, A2)

**Finding**: MapLibre draws the atmosphere in `drawAtmosphere()`. The bright side comes from
`u_sun_pos`, which `getSunPos(light, transform)` builds from the style's **light**:

```js
const lightPos = [-_lp.x, -_lp.y, -_lp.z];          // _lp = light.position, cartesian
if (light.properties.get('anchor') === 'map') {
  rotateZ(roll); rotateX(-pitch); rotateZ(bearing);
  rotateX(center.lat); rotateY(-center.lng);
}
return lightMat * lightPos;                          // direction to the sun, camera space
```

The page never sets a light, so the style's default applies: `anchor: "viewport"`,
`position: [1.15, 210, 30]`. With `viewport`, the matrix is the identity: the sun direction is
fixed in camera space, so the glow is always brightest towards the same corner of the window,
whatever the globe shows. That is A1 (hand moves) and A2 (auto-zoom moves): both change only
the transform, never the light.

With `anchor: "map"`, the light vector goes through the **same rotations as the globe**
(`VerticalPerspectiveTransform._calcMatrices`: roll, −pitch, bearing, lat, −lng). The sun then
is a fixed direction on the Earth. `getSunPos` runs on every frame, so the glow follows each
animation frame too (FR-003).

**First decision** (planning): set the style light to `anchor: "map"`.

**Revised during implementation (owner's rule, spec FR-019)**: an Earth-fixed light is
physically right, but when the day side faces the viewer the scattering shader puts a pale haze
over the whole day half of the disc. In the first browser run it dimmed land, labels and even
QSO markers (a red 40m marker turned pink). The owner's rule is that the data stays fully
visible. So the light stays a `viewport` light, recomputed from the camera on every move: it
points towards the Sun's on-screen direction, but **from behind the globe** (R2). That gives
feature 011's thin rim, brightest towards the Sun, and no haze over the map.

**Alternatives considered**:
- Draw our own glow (a custom WebGL layer or a CSS radial gradient behind the canvas). Rejected:
  it duplicates MapLibre's scattering shader, has to track the globe radius and centre offset on
  every frame, and is far more code (Principle III).
- An even glow with no bright side. Rejected by the owner (clarification, option C).

## R2: From the subsolar point to `light.position`

**Finding**: MapLibre's globe puts the point (φ, λ) on the unit sphere at
`(cos φ sin λ, sin φ, cos φ cos λ)` (`angularCoordinatesRadiansToVector`). Camera space is
standard WebGL: x right, y up, the camera looking down −z (the globe projection is a plain
`perspective` with no y flip). `light.position` is `[r, azimuthal, polar]`, converted by
`sphericalToCartesian` as `x = −r sin a sin p`, `y = r cos a sin p`, `z = r cos p`. `getSunPos`
negates the result.

To make the sun direction point at the subsolar point (φs, λs):

```
polar     = acos(−cos φs · cos λs)            (0°–180°)
azimuthal = atan2(cos φs · sin λs, −sin φs)   (−180° to 180°)
radial    = 1.15 (MapLibre's default; only the direction counts, the shader normalises it)
```

**Verification**: a script replays `getSunPos` with these values. With the view centred on the
subsolar point, the sun direction is `(0, 0, 1)`, straight at the viewer, for every point
tried: (0, 0), (−8.5, −30), (23, 120), (−23, −170) and (10, 179). With the view centred 90° east
of it, the sun lies at `x < 0`, the west limb, with `y` of the same sign as φs. The unit tests
keep this replay (R5).

**Revised (implementation)**: the page uses a `viewport` light, recomputed on every `move`:

1. Turn the Sun's Earth vector into camera space with the globe's own rotations,
   `Rx(−pitch)·Rz(bearing)·Rx(lat)·Ry(−lng)`. That gives `s`.
2. Keep its on-screen direction, and put the light behind the globe:
   `u = normalise(sx, sy, −2.5)`, so `z ≤ −0.93`. At `−0.7`, the first try, a crescent of haze
   still covered about 15 % of the disc near the lit limb.
3. Convert to the light position: `polar = acos(−uz)`, `azimuthal = atan2(ux, −uy)`.

**Lag**: the `move` event fires inside the animation frame, before the style update and the
drawing, so the new light is used in that same frame. There is no one-frame lag. That was the
reason this option was first rejected, and it was wrong. The mid-flight frames of browser
step 10 check it (FR-003).

**Tests** (`web/view.test.js`), on a replay of MapLibre's `getSunPos`:
- for every view in a grid of centres, bearings and pitches, the light comes from behind, with
  `z ≤ −0.9`;
- the rim faces the Sun's on-screen direction, within 1°;
- with the Sun straight in front of or behind the globe, the rim is even.

## R3: Where the subsolar point comes from

**Finding**: `sun.js` (feature 011) already computes the solar declination and the equation
of time for `solarElevation`. The subsolar point is:

- latitude = declination;
- longitude = −(UTC minutes + equation of time − 720) / 4, wrapped to −180°…180°.

**Decision**: split the shared part of `solarElevation` into an internal helper, and export
`subsolarPoint(date)` next to it. Both then use the same Sun position (FR-004). A test checks
that `solarElevation` at the subsolar point is 90° within 0.5°. UTC throughout (HR-1).

## R4: When the light is (re)applied

**Finding**:
- `map.setStyle()` (theme switch, feature 011) replaces the style, light included, so the light
  must be applied again in `onStyleLoad()`, as `applySky()` is.
- The Sun moves 0.25° per minute. The page already wakes every 60 s for the automatic theme
  (`THEME_TICK_MS`).
- The light depends on the camera (R2, revised), so it is recomputed on every `move` event. That
  covers pans, zooms, rotations, tilts, auto-zoom flights and the page's own moves. The cost is
  one `setLight` per frame while the map moves: a few multiplications and a style property
  update, measured by step 11's smoothness check (SC-004).

**Decision**: an `applyLight()` next to `applySky()`, called from `onStyleLoad()`, from every
`move`, and from the 60 s theme tick, which also refreshes the cached subsolar point. The grayline and the Sun symbol (R7 to R10) are refreshed by the same tick. A 0.25° step is not visible (the spec's no-jump edge case). The light is set on
both projections. On the flat map it changes only fill-extrusion shading (R6), and keeping it
set avoids a stale light when the globe comes back.

## R5: How to test it

**Unit tests (every change, `node --test 'web/*.test.js'`)**:
- `web/sun.test.js`: the subsolar point at the March equinox (latitude near 0°), the June
  solstice (near +23.44°) and the December solstice (near −23.44°). Its longitude moves 15° west
  per hour. `solarElevation` there is about 90°. Inputs at the date line and at midnight UTC
  wrap correctly.
- `web/view.test.js`: `lightFor(subsolar)` returns `anchor: "map"`. A copy of MapLibre 5.24.0's
  `getSunPos` and `sphericalToCartesian`, marked as copied, replays R2: centred on the Sun the
  direction is `(0,0,1)`, centred 90° east the Sun is on the west limb, and pole and date-line
  inputs work. These tests fail today, because there is no `lightFor`.

**Browser test (releases, feature 012)**: a new step `atmosphere` (contract
[contracts/browser-test.md](contracts/browser-test.md)). It measures the glow on both sides of
the globe from a screenshot. The PNG is decoded by Chrome itself: the runner hands the
screenshot back to the page as a `data:` URL and reads it with a 2D canvas. So there is no PNG
decoder in Node and no npm package. The page's WebGL canvas can't be read back, because
`preserveDrawingBuffer` is off. The step drags the globe with `Input.dispatchMouseEvent` (a new
`drag()` helper in `cdp.mjs`), then lets auto-zoom move it.

**Test clock**: the expected glow side depends on where the Sun is, so a real clock would make the
result depend on the hour of the release. Steps 10 and 11 run on a shifted page clock instead.
It is set with `Page.addScriptToEvaluateOnNewDocument`, starts at the June solstice moment when
the Sun is 90° west of the house, and keeps running. Equinox and pole geometry stay covered by
the unit tests (R5, R8).

## R6: Side effects of a map-anchored light

- The light also shades `fill-extrusion` layers (3D buildings in OpenFreeMap's `liberty`
  style). They show only at street-level zoom on the flat map, which a contest map never uses.
  With a map-anchored light the shading may come from another direction. Accepted, and noted in
  the quickstart.
- Hillshade layers use their own `hillshade-illumination-*` properties, not the style light.
  No change.
- Cost: one 4×4 matrix per frame, which MapLibre already computes. Nothing is added per frame
  (FR-007).

## R7: How to draw the grayline (FR-009 to FR-016)

> **Revised during implementation (2026-10-14).** The 25 stacked GeoJSON caps below were built
> and passed their unit tests, but on the globe they made the map 3× slower to drag. In the test
> browser's software renderer, a drag ran at 8.3 frames/s against 24.8 with none; with only 5
> caps it was still 14. The cost is the huge polygons themselves (the globe subdivides each
> one), not the number of passes. The page now draws the grayline as **raster tiles**, through
> its own `grayline://` protocol:
> - **The pixels:** each tile pixel gets the theme's night colour, at an opacity of
>   `shade(Sun height) × nightOpacity`, where `shade` is 0 at and above 0°, 1 from −12°, and
>   linear in between. So the fade is pixel-exact and smooth, with no 0.5° steps.
> - **The URL:** it carries the theme and the minute (`grayline://light/<minute>/{z}/{x}/{y}`).
>   `setTiles` with a new URL reloads the tiles in place: MapLibre keeps the old ones on screen
>   until the new ones arrive, so the refresh doesn't flash. That flash was R7's original reason
>   to reject raster tiles; it doesn't happen. The protocol returns an `ImageBitmap`, which
>   MapLibre 5 accepts directly.
> - **World copies, globe and poles:** raster tiles repeat on every world copy and drape over the
>   globe, so R8's pole closing and date-line unwrapping are no longer needed. Shading still stops
>   at about 85°, where Web Mercator ends.
> - **Speed:** one raster layer, one pass: 19.7 frames/s against 24.7 in software rendering, and
>   60 against 60 on a real GPU.
>
> The text below records the first design.

**Finding**: the places where the Sun is at height `h` or lower form a **spherical cap**. The cap
is centred on the antisolar point (the point opposite the subsolar point). Its angular radius is
`90° + h`, so `h = 0°` gives 90°, a hemisphere, and `h = −12°` gives 78°. A night gradient is
therefore a stack of nested circles on the Earth. Each one is a plain GeoJSON polygon.

**Decision**:
- **The polygons:** 25 nested caps, at `h = 0°, −0.5°, … −12°`, as one GeoJSON source,
  `grayline`. They are drawn by one `fill` layer, `grayline`, where every polygon has the same
  opacity `a`.
- **How the shade builds up:**
  - Overlapping fills in a layer blend, so a place inside `k` caps gets the shade `1 − (1 − a)^k`.
  - It grows from 0 at the horizon to the theme's full night shade at −12° (FR-010).
  - Each 0.5° step changes the shade by about 2 % of the total. Even at zoom 5 that is a step of
    11 px and 1 % opacity, which a viewer can't see.
  - `a` is chosen per theme so that `1 − (1 − a)^25` is the theme's night opacity.
- **The ring:** each cap is built with 361 points around the antisolar point, using the
  spherical destination formula. It is a new pure `destination(point, bearing, distanceDeg)` in
  `geodesic.js`, next to `greatCircle` (feature 001), with its own unit tests.

**Alternatives considered**:
- **Raster tiles drawn on the fly** (a custom protocol painting each tile pixel by pixel). Truly
  smooth, but the per-minute refresh clears the tile cache, so the whole night side flashes as
  tiles reload. It is also more code.
- **A custom WebGL layer.** It would need its own globe and Mercator shaders: MapLibre 5 custom
  layers must handle both projections themselves. Far too much code for Principle III.
- **One night polygon with a blurred edge.** MapLibre fills have no blur.

## R8: Caps over a pole, and the date line

> Superseded by the raster tiles (R7, revised): kept for the record.

**Finding**: a cap contains a pole when `|antisolar lat| + radius > 90°`. That happens for most
caps for weeks around the solstices. A ring around a pole doesn't close in longitude on the flat
map: going round it, the longitude grows by 360°.

**Decision**:
- **Longitudes:** unwrap them along the ring, so they are continuous.
- **A ring that spans 360°:** it goes round a pole. Close it through that pole: add
  `(last lon, ±90)` and `(first lon, ±90)`, with the pole on the antisolar latitude's side.
- **Other rings:** may run past ±180°. geojson-vt (inside MapLibre) wraps coordinates outside
  ±180° onto the neighbouring world copies, so every copy is shaded and there is no seam
  (FR-012).

**Unit tests** (`web/grayline.test.js`), on pure functions:
- every ring is closed;
- a solstice cap includes the right pole;
- a point inside the cap has `solarElevation ≤ h` (checked on a grid, against `sun.js`);
- a point outside has `solarElevation > h`;
- an equinox ring has no jump in longitude above 180°.

**Known limit**: MapLibre draws Mercator only up to ±85.05°, and on the globe it paints the
polar caps with a plain colour. Shading stops at about 85°, which SC-006 already excludes
("outside the polar caps").

## R9: Layer order, under labels and borders (clarification 2, FR-013)

**Finding** (OpenFreeMap styles, read 2026-10-14). In every style, all boundary layers
(`source-layer: "boundary"`) come after the land, water, road and building layers, and before
the place labels:

| Style | Theme | First boundary layer | Symbol layers before it |
|-------|-------|----------------------|-------------------------|
| `liberty` | light | 85 `boundary_3` | one-way arrows only |
| `fiord` | dim | 34 `boundary_state` | water and road names |
| `dark` | dark | 34 `boundary_state` | water name, one-way arrows, road names |

**Decision**:
- **Where the layer goes:** insert `grayline` just before the first boundary layer, or before
  the first symbol layer if a style has no boundary layer.
- **What that gives:** land, sea, roads and buildings are under the shading. Borders and place
  names (cities, states, countries, continents) are above it, at day contrast.
- **What gets shaded:** in `fiord` and `dark`, water and road names are shaded too. They show only
  when zoomed in, and they stay readable.
- **When:** the insertion point is found again after every style load, in `addOverlays()`.

**Unaffected**: the page's own layers (`theme-dim`, QSO paths, the house, QSO markers) are added
after the base map, so they stay above the shading.

## R10: The Sun symbol (clarification 3, FR-018)

**Decision**:
- **Drawing:** a GeoJSON point source `sun` and a `symbol` layer `sun`, inserted just below
  `qso-lines`, so it sits under every QSO overlay. Its icon is drawn once on a canvas: a
  sun-yellow disc with 8 rays and a dark outline. It is added with `map.addImage` after each
  style load, because a style switch removes images.
- **Always shown:** `icon-allow-overlap` and `icon-ignore-placement` are on, so labels never hide
  it.
- **Not clickable:** no click or hover handler is bound to it. The page binds handlers to named
  layers only (`qso-markers`, `qth`).
- **The far side of the globe:** MapLibre doesn't draw symbols the globe hides, so the symbol
  vanishes there by itself (spec edge case).

**Alternatives considered**:
- **A `circle` layer.** It is too close to the QSO markers, which are circles too.
- **A text glyph `☀`.** It isn't in OpenFreeMap's fonts.

## R11: Settings (FR-009, FR-018, Principle IV)

**Decision**: two new keys, next to `lqm.atmosphere` and following the same pattern. Any
missing, unknown or unreadable value falls back to the default, and a storage failure is
ignored.

| Key | Values | Default |
|-----|--------|---------|
| `lqm.grayline` | `on`, `off` | on |
| `lqm.sun` | `on`, `off` | off |

They get two checkboxes in the panel's Map group, after "Atmosphere on the globe": "Grayline
(day and night)" and "Sun position". Toggling one sets the layer's `visibility`; nothing is
rebuilt.

## R12: Shading colours per theme (FR-014)

**Decision**: each palette in `themes.js` gains `night` (a colour) and `nightOpacity` (the full
night shade). The values are a starting point, to be tuned by eye in the quickstart:

| Theme | `night` | `nightOpacity` |
|-------|---------|----------------|
| light | `#0b1a3c` | 0.40 |
| dim | `#000814` | 0.45 |
| dark | `#000000` | 0.55 |

`themes.test.js` checks that every palette has both fields, and that `nightOpacity` is between
0.2 and 0.7. Marker readability on the night side (SC-008) is checked by eye in the quickstart
and by the browser test's screenshots. A unit test can't know the base map's colours.

## R13: Browser test of the grayline (FR-017)

**Method**: compare screenshots with the grayline on and off. The difference is the shading
alone, whatever the base map draws underneath. Pixels are read as in R5: the PNG is handed back
to the page and read on a 2D canvas.

1. **Points:** the runner projects a point near the antisolar point (Sun below −12°) and one near
   the subsolar point (Sun above +10°) to screen. On the flat map this is Web Mercator from
   `data-view`. On the globe it is an orthographic projection about the view centre, at pitch 0
   and bearing 0.
2. **Check:** with the grayline on, the night point is at least 15 % darker than with it off,
   and the day point is within 2 %.
3. **Move:** after a drag, recompute the points and check again (FR-012).
4. **Sun symbol:** with "Sun position" on, the area around the subsolar screen point changes
   between on and off. With it off, it doesn't.
5. **Reload:** after a reload, `lqm.grayline = off` and `lqm.sun = on` are kept.
