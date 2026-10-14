# Contract: browser test steps `atmosphere` and `grayline`

This contract extends feature 012's
[browser-test.md](../../012-page-fixes-browser-tests/contracts/browser-test.md). Two steps are
added at the end of the run, so steps 01 to 09 keep their numbers and screenshots.

| # | Step | Phase | Checks | Covers |
|---|------|-------|--------|--------|
| 10 | `atmosphere` | B | On the globe, dark theme, atmosphere on: the glow is brighter on the limb towards the Sun after a hand drag, still is after a drag of about 180°, and still is on every frame of an auto-zoom flight and after it. Away from the limb the map looks the same with the atmosphere on and off. | US1 1, US2 1–2, FR-003, FR-008, FR-019, SC-001, SC-002, SC-003, SC-011 |
| 11 | `grayline` | B | Flat map and globe: with the grayline on, a point in deep night is at least 15 % darker than with it off, and a point in full day is within 2 %. Same after a drag. The Sun position choice draws the symbol at the subsolar point only when on. Frame rates are recorded. The Grayline off and Sun position on choices survive a reload. | US3 1–2, 6, 9, FR-007, FR-016, FR-017, SC-004, SC-006, SC-007 |

## Test clock (steps 10 and 11)

Before step 10, the runner installs a clock shift with `Page.addScriptToEvaluateOnNewDocument`
and reloads the page. `Date` and `Date.now()` are shifted by a constant offset, so time still
runs, but "now" starts at a chosen UTC instant `T`. The runner picks `T` on the June solstice
of the current year, at the moment the subsolar longitude is 90° west of the house (house
evening, 18:00 local solar time). The runner computes the same `subsolarPoint(T + elapsed)` in
Node, so page and runner agree on the Sun. Steps 01–09 run on the real clock, before the shift.

## How step 10 `atmosphere` works

1. **Set up:** Dark theme. Globe projection. Auto-zoom off. Zoom out until the whole limb fits
   the window.
2. **Expected side:** read `#map`'s `data-view` (centre and zoom). The runner imports
   `subsolarPoint` from `src/web/assets/sun.js` and `lightFor` from `src/web/assets/view.js`, and
   replays `getSunPos`. The sign of the result's `x` is the side where the glow should be
   brighter: negative means the west (left) limb.
3. **Drag:** move the mouse horizontally across the globe (`Input.dispatchMouseEvent`
   `mousePressed`, `mouseMoved` steps, `mouseReleased`). Repeat until `|x|` is at least 0.5, or
   give up after three drags. Then wait for a stable view.
4. **Measure:**
   - take a PNG screenshot, and hand it back to the page as a `data:` URL;
   - draw it on a 2D canvas and average the luminance of a band 2–6 % of the globe's radius
     outside the limb, on the left and on the right, at the globe centre's height;
   - the globe's visible radius is `f · tan(asin(R / (f + R)))`, with `R = 512 · 2^zoom /
     (2π cos lat)` and `f = (height / 2) / tan(18.435°)` (MapLibre's camera); the band is
     0.3–1.4 % of it outside the limb, where the shader's thin atmosphere is.
5. **Check:** the brighter side matches the sign of `x`, and the two averages differ by at
   least 10 % of the brighter one. Before the fix this fails for one of the two drags in item 6,
   because the bright side never moves.
6. **Turn the globe:** drag about 180° of longitude, and do items 2 to 5 again. The expected
   side flips, and so must the measured one.
6b. **No haze (SC-011, FR-019):** after item 6, average 20×20 px boxes at the globe centre and
   at 50 % and 85 % of the radius towards the Sun, with the atmosphere on and off. Each pair is
   within 2 % (5 of 255).
7. **Auto-zoom:** send a QSO from Brazil (`GG66`, which keeps the framing wide), drag the globe
   so its centre is about 40° east of the house, and switch auto-zoom on. It flies back towards
   the house. On every frame of the flight (until `data-view` changes, at `moveend`), read
   `#map`'s `data-camera` and `data-light`, which the page writes together on every `move`. The
   light's on-screen side must match the Sun's side for that camera, worked out independently in
   the runner, wherever the Sun's `|x| ≥ 0.3`. At least 3 frames are needed. One mid-flight frame is kept
   as a screenshot, because a capture takes longer than a frame. Screen pixels can't be checked mid-flight: the zoom changes faster than the glow
   band is wide. After the flight, do items 2 to 5 around the globe's on-screen centre
   (`data-centre`, which auto-zoom's padding moves). Then switch to Flat and back to Globe, and
   do items 2 to 5 again (US2 3).
8. **Screenshots:** `10-atmosphere-drag`, `10-atmosphere-turned`, `10-atmosphere-flight`,
   `10-atmosphere-autozoom`.

## Failure messages

Each failure names the expected side, both luminance values and the `data-view`. For example:
`glow brighter on the right (L 38.2 / R 61.0), Sun on the left, view 12.3,-45.6,1.20`.

## How step 11 `grayline` works (research R13)

1. **Set up:** Light theme. Flat map. Auto-zoom off. Zoom out so the whole world width fits the
   window.
2. **Points:** compute the subsolar point (from `sun.js`), and pick:
   - a **night point:** 30° of latitude from the antisolar point, towards the equator, where the
     Sun is below −12°;
   - a **day point:** the same distance from the subsolar point, where the Sun is above +10°.
   Project both to screen from `data-view` (Web Mercator). Keep only points inside the window,
   moving them by up to ±60° of longitude to fit if needed.
3. **Compare:** take a screenshot with `#grayline` checked and one with it unchecked. Average a
   9×9 px box at each point:
   - **night point:** with the grayline on, it is at least 15 % darker;
   - **day point:** it differs by at most 2 %.
4. **Pan:** drag the map 300 px. Recompute the points and do item 3 again.
5. **Globe:** switch to Globe. Drag until the view centre is in deep night (Sun below −12°), and
   check that the centre box gets darker with the grayline on. Drag about 180°, until the centre
   is in full day (Sun above +10°), and check that it doesn't.
6. **Sun symbol:** back on the flat map. Check `#sun-marker`. The box at the subsolar screen
   point changes by at least 10 % against the screenshot with it unchecked.
7. **Smoothness (recorded, not judged):** on the globe, count `requestAnimationFrame` callbacks
   during a 2 s scripted drag with nothing on and with Atmosphere, Grayline and Sun position all
   on (each part alone was measured once during implementation: the grayline is the cost). The rates go in the report (`framesPerSecond` on the step) and
   the log. The step doesn't fail on them: the test browser renders in software (SwiftShader).
   During implementation, the grayline cost about 25 % there, and nothing on a real GPU (60
   frames/s in every case).
8. **Reload:** leave the grayline unchecked and the Sun position checked, and reload. Both
   checkboxes keep their state, and `localStorage` holds `lqm.grayline=off` and `lqm.sun=on`.
9. **Screenshots:** `11-grayline-flat`, `11-grayline-panned`, `11-grayline-globe-night`,
   `11-grayline-sun`.

## Unit-tested runner helpers (`web/browser-lib.test.js`)

- **`limbSamples`, `sunSide`:** for step 10.
- **`testInstant(house, year)`:** for the test clock.
- **`toScreenMercator(view, size, point)`:** for step 11.
- **`toScreenGlobe(view, size, point)`:** for step 11; returns `null` on the hidden half.
- **`luminance(rgba)`:** for both steps.
