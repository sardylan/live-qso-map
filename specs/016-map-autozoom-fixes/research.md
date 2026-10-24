# Research: Map Auto-Zoom Fixes

All numbers below come from running the current `src/web/assets/autozoom-geo.js` (release 3.2.0)
on the operator's snapshot (spec Assumptions) with MapLibre 5.24.0's globe geometry, as that file
already models it.

## R1: Why the globe shrinks to a dot

- **Finding**: the snapshot's bounding cap has a radius of 79.6°, centred near 7.9 N, 74.3 E.
  `fitsGlobe(cap, 5)` passes (79.6 + 5 ≤ 90). `globeZoomForCap` then applies its limb rule: the
  cap edge must stay within `LIMB_SHARE` (0.9) of the horizon angle. For a 79.6° cap that means a
  horizon of at least 88.5°, which only a camera very far away gives. The result is zoom −1.39 at
  1024 × 768 (globe radius 31 px) and −1.07 at 1920 × 960 (39 px).
- **Decision**: delete the limb rule and the 5° `fitsGlobe` margin. The globe zoom is only the
  free-area fit (R2). A separate border check then decides globe or flat at that zoom, and never
  zooms out (R3).
- **Alternatives considered**: lowering `LIMB_SHARE` or raising the margin. That keeps the same
  shape of failure: any cap that passes the margin can still shrink the globe.

## R2: Globe zoom = free-area fit only

- **Decision**: keep the existing closed form for "cap edge at `half` px from the centre"
  (`radius = half·c / (c·sinθ − half·(1 − cosθ))`, or no limit when the denominator is ≤ 0), capped
  at `MAX_ZOOM`. That form describes the near side only, so the fit is valid only when the cap edge
  is in front of the horizon. R3 rejects every other case.
- **Snapshot**: the fit gives a globe radius of 286 px at 1024 × 768 and 415 px at 1920 × 960. The
  horizon there is at 78.5° and 77.1°, and the cap edge (79.6°) lies 1.1° and 2.6° behind it.

## R3: The border check, in degrees of arc

- **Decision**: at the fit zoom, the horizon angle seen from the camera is
  `H = acos(R / (c + R))`, with R the globe radius in px and c the camera distance
  (`0.5 / tan(fov/2) · height`), both as `autozoom-geo.js` already computes them. The globe is
  kept when `capRadius + border ≤ H`. Otherwise the result is `null`, and auto-zoom shows the flat
  view (feature 004 FR-016 path, unchanged).
- **Horizon outside the view**: H shrinks as the zoom grows. It is about 77° at zoom 2, 57° at
  zoom 4, 18° at zoom 8 and 4.5° at zoom 12, for a 960 px high map. With a literal reading, a
  tight cluster of QSOs framed at zoom 12 would fail a 10° border and go flat, although no horizon
  is anywhere on screen. So the check applies only when the horizon circle is in view. Its
  on-screen radius (the cap formula at θ = H) is compared with half the map's diagonal. When it is
  larger, the horizon is outside the map and every framed marker meets the border. The spec's
  FR-006 is updated to say so.
- **Continuity**: the horizon enters the corners near zoom 4 (918 px against a 1,100 px
  half-diagonal at 1920 × 960). Caps that fit at that zoom are about 12° in radius, so their margin
  (about 45°) is far above any allowed border. The switch to flat happens only for wide caps at low
  zoom, as intended.
- **Alternatives considered**: a pixel border (rejected by the operator, spec Clarifications); a
  border scaled by H (harder to explain in the config file, and no gain).

## R4: Flat view centred on the QTH

- **Decision**: a new pure function in `autozoom-geo.js` takes the QTH, the framed points, the map
  size and the padding, and returns `{ center, zoom }`:
  - `dLon` = the largest |Δlongitude| from the QTH, wrapped to [−180°, 180°] (the shorter way,
    FR-002);
  - `up` and `down` = the largest Mercator-y distance above and below the QTH, with latitudes
    clamped to ±85.051129°;
  - zoom = min(log2(halfW · 360 / (2 · dLon · 512)), log2(halfH · 2π / (max(up, down) · 512)),
    `MAX_ZOOM`), where halfW and halfH are half the free area (map size minus padding);
  - `dLon = 0` and `up = down = 0` (everything at the QTH) give `MAX_ZOOM`.
- The page calls `map.easeTo({ center: qth, zoom, padding })`. MapLibre's padding puts the centre
  in the middle of the free area, as `fitBounds` already does today.
- **Poles (FR-003)**: MapLibre's Mercator clamps the camera so it never shows beyond ±85°. Near a
  pole it moves the centre towards the equator by itself, which is "as close to the house as the
  map allows". Markers on the far side could then fall outside, so in that case, and only then,
  the zoom is lowered until the clamped view holds them. A unit test covers a QTH at 80 N.
- **No QTH**: unchanged feature 004 `fitBounds(flatBounds(…))` (FR-004).
- **Snapshot estimate**: centred on 39.05 N, 8.38 E, dLon = 136° (VK3ATL) and the vertical span is
  set by VK3ATL below and SM5VTP above. That gives about zoom 1.5 at 1920 × 1080 with the default
  panels, against about 2 for an uncentred `fitBounds`. The spec accepts this cost (Clarifications,
  "always prefer").

## R5: The flat zoom seen at level 0.85

- **Finding**: on the live page at 1024 × 768, the flat view settled near zoom 0.85, centred
  mid-Atlantic. An uncentred `fitBounds` on the snapshot bounds with the measured padding should
  give about 1.57. The exact cause was not reproduced outside the page.
- **Decision**: the flat refit no longer uses `fitBounds` when the QTH is known (R4). The new path
  computes its own zoom, and the browser regression step (R7) asserts the camera from `data-view`
  for the snapshot at both window sizes. If the 0.85 zoom comes from elsewhere (padding,
  projection timing), that step fails first, as Principle I requires.
- **FR-009 (one refit, the projection on screen)**: `fitNow` already calls `showProjection` before
  choosing the camera. The plan keeps that order and adds a unit test on `autozoom.fitted` for the
  globe → flat → globe sequence within consecutive refits.

## R6: The setting `map.globe_border_deg`

- **Decision**: `map.globe_border_deg`, a number in degrees, default 10, valid range 0 to 45
  inclusive, finite. CLI `--globe-border-deg`, environment `LQM_GLOBE_BORDER_DEG`. It is layered
  like every map setting (CLI > environment > file > default), and validated with a config error
  naming `map.globe_border_deg`.
- It is sent as `hello.globe_border_deg` (a number). This is an added field, so the protocol stays
  at version 3 (`docs/http-api.md` API-4). If the field is missing or out of range, the page uses
  10.
- `--save-config` writes it, and `deploy/config.example.toml` lists it commented out (feature 015
  checks).
- **Name**: the unit suffix follows `max_age_days` and `timeout_secs`.

## R7: Tests

- **Unit (node --test web/)**: border check (snapshot → `null` at both sizes; a European-only cap
  → a zoom; a tight cap at zoom 12 → a zoom; border 0 at the horizon → kept); the flat centred
  frame (snapshot numbers, an antimeridian QTH, everything at the QTH, a pole); the old limb/margin
  tests are rewritten to the new rule.
- **Rust**: config layering, range and parse errors, the default, `hello` serialisation, and the
  example-config and save-config checks for the new key.
- **Browser (tests/browser)**: a new step loads the snapshot positions (callsigns and coordinates
  only), with the globe preference and auto-zoom on. At 1024 × 768 and 1920 × 1080 it asserts the
  flat view, the house at the free-area centre (SC-001) and the zoom (SC-004). Per LL-007, the
  step is rehearsed in the CI rehearsal pipeline before the branch merges.
