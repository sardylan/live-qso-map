# Feature Specification: Map Auto-Zoom Fixes

**Feature Branch**: `feature/016-map-autozoom-fixes`
**Created**: 2026-10-24
**Status**: Draft
**Input**: User description: "There are little bugs on the frontend. When displaying the flat view
with auto-zoom, always prefer to center the map on the local station position. For the globe view
add a frontend parameter with the minimum border for display markers, switch to flat view if this
minimum border cannot be guaranteed. There is also a little bug on the auto-zoom level. Fetch "now"
https://contest.eracagliari.eu/ to have an example, asap in order to obtain the current situation"

## Problem Summary

Auto-zoom (feature 004) frames the house and every displayed QSO. On a live contest page
(https://contest.eracagliari.eu/, observed 2026-10-24, release 3.2.0, 1024 × 768 window, QTH in
Sardinia, QSOs from Western Europe to Australia), it fails in both projections:

- **Globe, auto-zoom on**: the whole Earth is drawn as a dot about 80 px across in the middle of
  an otherwise empty, dark map. The QSOs fit on one side of the globe, so the page keeps the globe,
  but it zooms out so far that markers, lines and labels can't be read. The rule meant to keep
  markers away from the globe's edge only ever zooms further out; it never gives up and switches
  to flat.
- **Flat, auto-zoom on**: the map is centred over the mid-Atlantic, not on the station. It is also
  zoomed out to about level 0.85, so the world repeats three times across the window and empty
  background shows below the map's bottom edge. The same QSOs would fit, clear of the panels, at
  about level 2.

The same happens at 1920 × 1080 in Firefox: auto-zoom zooms out too far. The operator supplied the
page's snapshot of the 20 displayed QSOs (CQ WW SSB, station IQ0ID, 2026-10-24 07:52–08:06Z). Those
20 positions plus the QTH fit inside a circle about 79.6° in radius on the globe. That passes
today's 5° check against the globe's edge, so the page keeps the globe. But at the zoom where that
circle fills the free map area, the two Australian QSOs are already 1–3° behind the visible
horizon. The old rule then shrinks the globe to a radius of only 30–40 px to bring them back.

After this feature, the flat view centres on the station whenever its position is known. The
globe is kept only when every framed marker can be shown at least a minimum arc inside the
globe's visible horizon. The operator sets that arc in degrees, and when it can't be guaranteed the
page switches to flat. In both views, auto-zoom uses the highest zoom level that still frames
everything.

## Clarifications

### Session 2026-10-24

- Q: How is the minimum globe border measured? → A: In degrees of arc from the globe's visible
  horizon, as seen from the camera at the zoom auto-zoom would use.
- Q (operator input): which data reproduces the bug? → A: The page's snapshot of 20 QSOs from
  https://contest.eracagliari.eu/ (listed under Assumptions). Auto-zoom zooms out too far at
  1920 × 1080 in Firefox as well as at 1024 × 768.
- Q: Who sets the minimum globe border? → A: The operator only, in the `[map]` section of the
  configuration file. It is the same for every viewer and is not shown in the settings panel.
- Q: Where does the globe centre under auto-zoom? → A: On the point that best frames the framed
  items (feature 004 FR-016), as before. Only the flat view centres on the station.
- Q: What is the default minimum globe border? → A: 10° of arc (range 0° to 45°).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Readable globe or a switch to flat (Priority: P1)

A viewer watches the map on the globe with auto-zoom on during a contest. As QSOs spread across
the world, the globe either stays large enough that every marker sits comfortably inside its
visible horizon, or the page switches to the flat view. It never shrinks the Earth to a dot.

**Why this priority**: This is the bug a viewer sees today on the live page. The map is unusable
until the viewer switches to flat by hand, which also turns auto-zoom off.

**Independent Test**: With the globe preference and auto-zoom on, load a set of QSOs whose
framed items fit on one side of the globe only near its edge (for example, a European QTH with
QSOs in Europe and Australia). Check that the page shows the flat view. Then load QSOs that sit
well inside one side of the globe, and check that the globe is shown with every marker at least
the configured arc inside its horizon.

**Acceptance Scenarios**:

1. **Given** the globe preference, auto-zoom on and the configured minimum border, **When** at the
   highest zoom that fits the framed items in the free map area every marker is at least that
   arc inside the visible horizon, **Then** the page shows the globe at that zoom.
2. **Given** the same settings, **When** at that zoom any framed marker is closer to the visible
   horizon than the border, or behind it, **Then** the page shows the flat view (feature 004
   FR-016 automatic switch), not a smaller globe.
3. **Given** an automatic flat view, **When** later changes let the framed items fit the globe
   with the border again, **Then** the page returns to the globe (feature 004 FR-016).
4. **Given** the operator configures no minimum border, **When** the page loads, **Then** the
   documented default border applies.

---

### User Story 2 - Flat view centred on the station (Priority: P1)

A viewer on the flat view with auto-zoom on sees the station's house in the middle of the map,
with every displayed QSO in view around it, the way an operator naturally reads "where did my
signal go".

**Why this priority**: The flat view is also where the globe now falls back to (Story 1), so a
badly centred flat view would replace one bug with another.

**Independent Test**: With a known QTH, the flat preference and auto-zoom on, load QSOs in
several directions. Check that the house is at the centre of the free map area and that every
marker is visible and clear of the panels.

**Acceptance Scenarios**:

1. **Given** a known QTH, the flat view and auto-zoom on, **When** auto-zoom refits, **Then** the
   house is at the centre of the free map area (the map area left clear of the page panels,
   feature 004 FR-009) and every displayed marker is inside that area.
2. **Given** QSOs on both sides of the station that are far apart, **When** auto-zoom refits,
   **Then** the map stays centred on the house and zooms out only as far as it needs to show them.
3. **Given** the QTH is unknown, **When** auto-zoom refits the flat view, **Then** it frames the
   markers as before (feature 004), since there is no station to centre on.
4. **Given** the flat view after an automatic switch from the globe, **When** auto-zoom refits,
   **Then** the same centring on the house applies.

---

### User Story 3 - Correct auto-zoom level (Priority: P2)

Whatever the projection, auto-zoom picks the closest zoom that still shows everything it frames,
so the viewer sees as much detail as possible.

**Why this priority**: Most of the wasted space is gone once Stories 1 and 2 are fixed. This story
pins the zoom level so it doesn't regress to a far-out view.

**Independent Test**: For a fixed set of QSOs and window size, compare the zoom auto-zoom chose
with the highest zoom at which every framed item still fits in the free map area (under the
centring rule of the active view).

**Acceptance Scenarios**:

1. **Given** auto-zoom on and the frame from the live example (QTH in Sardinia, QSOs in Western
   Europe and Australia, 1024 × 768 window), **When** the flat view is shown, **Then** the zoom is
   the highest one that keeps every marker and the house in the free map area, centred on the
   house, and the map is not repeated three times across the window.
2. **Given** auto-zoom on, **When** a refit runs right after an automatic switch between globe and
   flat, **Then** the zoom it uses is computed for the projection now on screen, not the previous
   one.
3. **Given** auto-zoom on, **When** the framed items would allow a zoom beyond the maximum
   (feature 004 FR-013), **Then** the maximum level is used.

### Edge Cases

- **All QSOs at or next to the QTH**: the flat view centres on the house at the maximum zoom
  level (feature 004 FR-013). The globe shows them centred, since the border is easy to keep.
- **No QSOs displayed**: the frame is the station's entity extent, or the fixed radius around the
  QTH (feature 004 FR-010), centred on the house on the flat view. The minimum border applies to
  the globe as for markers.
- **QSOs spread right round the world from the station** (e.g. an antipodal QSO): on the flat view,
  centred on the station, a marker 180° of longitude away lies on the left and right edges of the
  world. The flat view zooms out to show the full width of the world centred on the station.
- **QTH near a pole**: centring on the house would need latitudes beyond what the flat map can
  draw. The view centres as close to the house as the map allows, and every marker stays visible.
- **A very small window, or panels leaving almost no free area** (feature 004 FR-009 small-screen
  case): if the border can't be kept on the globe, the page shows the flat view.
- **Operator sets a border too large for any globe at the current window size**: the page always
  shows the flat view while auto-zoom is on under the globe preference. This is not an error.
- **Operator sets an invalid border** (negative, not a number, or beyond the documented range): the
  service rejects the configuration at startup, naming the setting (as for other `[map]` settings).
- **Auto-zoom off**: none of this changes the view. The projection and camera stay as the viewer
  left them (feature 004 FR-011, FR-017).

## Requirements *(mandatory)*

### Functional Requirements

**Flat view centring**

- **FR-001**: With a known QTH and auto-zoom on, every flat-view refit MUST centre the map on the
  house, measured in the free map area left by the page panels (feature 004 FR-009).
- **FR-002**: The flat-view zoom MUST be the highest level, up to the maximum (feature 004 FR-013),
  at which the house and every displayed marker lie inside the free map area with the house at its
  centre. Markers are measured along the shorter way round the world from the house.
- **FR-003**: When the house can't be exactly centred, the flat view MUST centre as close to the
  house as the map allows, and every displayed marker MUST stay inside the free map area. This
  happens whenever centring would show beyond the map's top or bottom edge (±85°): near a pole, or
  when the zoom needed for the markers leaves a map shorter than the window can centre on the house
  (seen at 1024 × 768 with the snapshot).
- **FR-004**: With an unknown QTH, the flat-view refit MUST keep the feature 004 behaviour (frame
  every displayed marker).

**Globe minimum border**

- **FR-005**: The operator MUST be able to set, in the map section of the configuration file, the
  minimum border for markers on the globe, in degrees of arc: the angle on the Earth's surface
  between a marker and the globe's visible horizon, as seen from the camera at the zoom in use.
  The setting MUST have a documented default (10°) and range (0° to 45°). It MUST be listed
  (commented out) in the example configuration (feature 015), and MUST reach the page with the
  other map settings.
- **FR-006**: With the globe preference and auto-zoom on, the page MUST keep the globe only when,
  at the highest zoom whose framed items fit the free map area (FR-008), the house and every
  displayed marker are each at least the minimum border inside the visible horizon. When the
  horizon lies entirely outside the map view at that zoom (zoomed in close), no marker can be
  near it and the border is met (research R3).
- **FR-007**: When FR-006 can't be satisfied, the page MUST switch to the flat view. It MUST NOT
  zoom the globe further out to bring markers away from the horizon. All other feature 004 rules for
  automatic switches keep applying (return to the globe as soon as it fits again, FR-016; the
  saved preference never changes, FR-018; no switch while auto-zoom is off, FR-017).
- **FR-008**: The globe's zoom MUST be the highest level, up to the maximum, at which the framed
  items fit the free map area, centred on the point that best frames them (feature 004 FR-016).

**Auto-zoom level**

- **FR-009**: Every refit MUST compute its zoom for the projection actually on screen after any
  automatic switch made by that same refit.
- **FR-010**: The flat view under auto-zoom MUST NOT be zoomed out beyond the level FR-002 and
  FR-003 require. In particular, it MUST NOT show the world repeated, or empty space beyond the
  map's top or bottom edge, when the framed items fit at a higher zoom.
- **FR-011**: The live-example case (Problem Summary, positions under Assumptions) MUST be covered
  by an automated check for both projections, at 1024 × 768 and at 1920 × 1080.

**Unchanged**

- **FR-012**: Everything else in feature 004 MUST keep working unchanged: refit triggers and
  coalescing, panel padding, popups pausing refits, manual moves switching auto-zoom off, the
  projection button, and the no-QSO frame. The same applies to features 002 and 013 (projection
  preference, atmosphere, grayline) and to features 011 and 012 (settings panel, browser tests).

### Key Entities *(include if feature involves data)*

- **Minimum globe border**: an operator setting in the map section of the configuration file. It
  is an angle in degrees of arc (default 10°, range 0° to 45°), sent to the page with the other map
  settings, and it decides when the globe is legible enough to keep under auto-zoom.
- **Free map area**: the part of the map left clear of the page panels (feature 004 FR-009). The
  centre of the flat view and the limits of every fit are measured in it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a known QTH and the flat view under auto-zoom, after every refit the house is
  within 2 % of the free map area's width from its centre, and within 2 % of its height unless the
  view is held at the map's top or bottom edge (FR-003).
- **SC-002**: Under auto-zoom, 100 % of the displayed markers and the house are inside the free map
  area after every refit, in both projections.
- **SC-003**: On the globe under auto-zoom, every displayed marker is at least the configured
  border, in degrees of arc, inside the globe's visible horizon. When that can't be met, the flat
  view is on screen within one refit (at most 1 s, feature 004 FR-014).
- **SC-004**: For the live-example frame at 1024 × 768 and at 1920 × 1080, with the globe
  preference and the default border, the page shows the flat view, centred on the house, at a zoom
  within 0.25 levels of the highest one that keeps everything in the free map area. Today it
  shows the globe 30–40 px in radius, or the flat view at about level 0.85. *(Feature 018 moved
  the small size to 1280 × 800: beside its left column, a 1024 px window leaves the map narrower
  than the flat world at the zoom that fills the map's height.)*
- **SC-005**: For any fixed set of positions and window size, the zoom auto-zoom chooses is within
  0.25 levels of the highest zoom that satisfies the active view's rules (FR-002 or FR-008).
- **SC-006**: All existing automated checks for features 002, 004, 011, 012 and 013 pass unchanged,
  except those that asserted the old flat centring or the old globe-shrinking behaviour, which
  are updated to the new rules.

## Assumptions

- "Frontend parameter" means an operator setting in the `[map]` section of the configuration file
  (confirmed, see Clarifications), sent to the page like the last-QSO box settings (feature 006).
  It is not a per-viewer setting in the settings panel, so every viewer of the same site sees the
  same behaviour.
- "Centre on the local station" means the house (the QTH, feature 004 FR-002) is at the centre of
  the free map area, not of the whole window. Panels are already kept clear by FR-009.
- "Prefer" means centring wins whenever the QTH is known. Without a QTH, the old fit is kept.
- The globe keeps centring on the point that best frames the items (feature 004 FR-016), not on
  the house (confirmed, see Clarifications). Centring on the house is only for the flat view.
- The flat-view zoom bug and the tiny globe are the "little bug on the auto-zoom level". Both were
  seen on the live page with the same QSO set, and this spec covers both.
- The existing 5° keep-away margin and the rule that shrinks the globe to move markers away from
  its horizon are both replaced by the new minimum border.
- The default of 10° (confirmed, see Clarifications) keeps markers clearly off the squashed
  band next to the horizon, and still lets a European station's typical European and
  Mediterranean QSOs stay on the globe.
- The automated check uses the positions from the operator's snapshot (station IQ0ID, QTH
  39.051738 N, 8.37711 E), as `lat, lon`:
  - VK3ATL −38.155567, 144.367475 · VK6LG −34.938333, 117.873333
  - SM5VTP 58.571685, 16.553067 · SK6D 57.955013, 12.455583 · DL9ZP 53.600088, 11.428925
  - M5S 52.77, −1.47 · G0MBA 51.803614, 1.140091 · DF6QV 51.675729, 9.003269
  - ON6IO 51.02, 4.623333 · M1STI 50.978099, −0.748722 · DM1SV 49.147049, 8.688986
  - OK2TS 49.104532, 17.166542 · TM1FC 48.854167, 2.375 · F4ICZ 48.069238, 1.334538
  - YT4M 44.895833, 20.375 · YO8SBQ 44.561667, 25.956667 · EA1FUM 42.346452, −3.721318
  - EE7I 40.395833, −3.708333 · CT2FUN 40.229167, −8.458333 · CT2IOV 37.1578, −8.073143
- Only positions are used from the snapshot. Names, addresses and other personal details in it are
  not copied into the project.
