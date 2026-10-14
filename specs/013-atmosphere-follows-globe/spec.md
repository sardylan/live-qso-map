# Feature Specification: Atmosphere that moves with the globe, and the grayline

**Feature Branch**: `feature/013-atmosphere-follows-globe`
**Created**: 2026-10-14
**Status**: Draft
**Input**: User description: "Check the atmosphere, because it seems that is not moving with the globe, if I move the globe manually. I also noticed that when the auto-zoom move the globe, the atmosphere remain fixed. Atmosphere must move together with the globe, regardless of automatic or manual zoom." Extended the same day: "To the existing spec/feature, together with the fix for the globe view, add also the equivalent for the flat view (grayline)."

<!--
  Use the amateur radio terms defined in docs/ham-radio.md §2 (QSO, entity, band, locator,
  dupe…). Requirements MUST NOT contradict its rules in §1 (e.g. UTC times, bands listed in
  frequency order).
-->

## Context

Feature 011 added an atmosphere to the globe (FR-010): a glow along the Earth's limb, brighter
on one side, and a sky colour behind it. The owner reports that the glow doesn't follow the
globe:

| # | Bug | Seen |
|---|-----|------|
| A1 | When the viewer drags or rotates the globe by hand, the Earth turns underneath but the glow stays where it was on screen: the bright side is always in the same corner of the window, whatever part of the Earth faces the viewer. | Owner, manual pan |
| A2 | When auto-zoom moves the globe to frame the house and the QSOs, the same happens: the Earth turns, the glow doesn't. | Owner, auto-zoom on |

The glow looks painted on the window instead of belonging to the Earth. This breaks the
illusion that the atmosphere is meant to give (feature 011, User Story 3).

**The flat map today:** the flat map shows nothing about the Sun at all (feature 011, FR-011).
The owner asked for its equivalent there: the **grayline**. That is the day/night terminator
with its twilight band, shown on the map, and the night side shaded.

Operators use the grayline to plan contacts. Along the twilight band, low bands (160m, 80m,
40m) often open over long paths between stations that are both near sunrise or sunset. Seeing
it on the map tells the operator where to point and listen.

## Clarifications

### Session 2026-10-14

- Q: What decides which side of the Earth the glow is brightest on? → A: The real Sun: the bright
  side faces the point where the Sun is overhead now, consistent with the sun-based theme.
- Q: Which control turns the grayline on and off? → A: A separate "Grayline" choice in the
  settings panel's Map group, on by default and saved in the browser. The atmosphere choice keeps
  controlling only the glow.
- Q: Does the globe also shade night and draw the twilight band on the Earth's surface? → A: Yes.
  The grayline is drawn on both views, under the glow on the globe, so both show the same thing.

### Session 2026-10-14 (clarify)

- Q: How does the shading go from day to night? → A: A smooth gradient that follows the Sun's
  height: no shade with the Sun above the horizon, darkening continuously to full night shade,
  with no hard edges.
- Q: Does the night shading cover the base map's place names and borders, or sit under them? → A:
  Under them: it darkens land, sea and roads, while place names, borders and coastlines stay on
  top at full contrast.
- Q: Does the map also mark the point where the Sun is overhead? → A: Yes, as a choice in the
  settings panel, saved in the browser, off by default.

### Session 2026-10-14 (implementation)

- Q: With the light on the real Sun, MapLibre's atmosphere puts a pale haze over the whole day
  side whenever it faces the viewer, which dims the map, its markers and its lines. How should the
  globe look? → A: The data comes first: markers, great-circle lines and land must stay fully
  visible. A little effect is fine for a pretty picture, but it must not reduce the visibility of
  the displayed data. (So the glow stays a thin rim along the limb, brightest towards the Sun,
  and is never a haze over the map: FR-004, FR-019.)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The atmosphere turns with the Earth when moved by hand (Priority: P1)

A viewer has the globe on screen with the atmosphere on. They drag the globe to look at another
continent, rotate it, and zoom in and out. The glow stays attached to the Earth: its bright side
turns away with the part of the Earth it was lighting, and the glow keeps hugging the limb at
every zoom level.

**Why this priority**: this is the bug the owner saw first, and it happens with every manual
move.

**Independent Test**: open the page with the globe and the atmosphere on, auto-zoom off. Note
which part of the Earth the bright side of the glow is next to. Drag the globe half a turn
east. Check that the bright side moved with that part of the Earth instead of staying in the
same corner of the window.

**Acceptance Scenarios**:

1. **Given** the globe with the atmosphere on, **When** the viewer drags the globe east or west,
   **Then** the bright side of the glow moves across the screen together with the part of the
   Earth it was next to.
2. **Given** the globe with the atmosphere on, **When** the viewer drags the globe north or
   south, **Then** the glow follows in the same way.
3. **Given** the globe with the atmosphere on, **When** the viewer rotates the map (changes its
   bearing) or tilts it, **Then** the glow turns with the Earth.
4. **Given** the globe with the atmosphere on, **When** the viewer zooms in or out, **Then** the
   glow stays along the limb of the Earth at every zoom level where the limb is visible, with
   no gap and no offset between the Earth and its glow.

---

### User Story 2 - The atmosphere turns with the Earth during auto-zoom (Priority: P1)

The operator leaves auto-zoom on during a contest. Each new QSO moves the globe to frame the
house and the QSOs. The glow moves with the Earth during each move and after it, as it does for
a manual move.

**Why this priority**: auto-zoom is how most operators watch the map during a contest, so this
is the view they see most. It is the same defect as User Story 1, seen through another kind of
move.

**Independent Test**: open the page with the globe, the atmosphere and auto-zoom on. Send a QSO
with a station on the other side of the Earth from the house. While the globe moves, check that
the glow moves with the Earth. When the move ends, check that it is in the same place relative
to the Earth as a manual move to the same view would give.

**Acceptance Scenarios**:

1. **Given** the globe with the atmosphere and auto-zoom on, **When** a new QSO makes auto-zoom
   move the globe, **Then** the glow moves with the Earth during the whole animated move.
2. **Given** the same view reached once by auto-zoom and once by hand, **When** both are
   compared, **Then** the glow is the same in both.
3. **Given** auto-zoom switching the globe to the flat map because the QSOs don't fit (feature
   004), **When** the flat map shows, **Then** no atmosphere is drawn (feature 011, FR-011), and
   when the globe comes back, the glow follows the Earth again.

---

### User Story 3 - The grayline on the flat map and on the globe (Priority: P2)

The operator watches the map during a contest, flat or globe. The part of the Earth in night is shaded.
A softer band marks the twilight zone between day and night: the grayline. Both follow the real
Sun: they slide west as the hours pass. They stay attached to the map when the viewer pans or
zooms and when auto-zoom moves the map. On the globe they lie on the Earth's surface, under
the atmosphere glow, and turn with the Earth like the glow does. QSO markers, paths and the house marker stay clearly
visible on top of the shading.

**Why this priority**: it is new information, not a fix. The globe fix (User Stories 1 and 2)
comes first. It is still worth having in the same feature: it uses the same Sun, and it gives
both views the same picture of day and night.

**Independent Test**: open the flat map, and compare the shaded area with a reference
day/night map for the same UTC time. The terminator lines up. Pan and zoom: the shading moves
with the map. Switch to the globe, and check that the same places are shaded. Leave the page
open for an hour: the shading has moved about 15° of longitude west.

**Acceptance Scenarios**:

1. **Given** the flat map, **When** it is shown, **Then** the night side is shaded, and the
   shading fades smoothly to nothing across the twilight zone towards day, matching the Sun's
   position at the current UTC time.
2. **Given** the flat map or the globe with the grayline, **When** the viewer pans, zooms, rotates
   or tilts, or auto-zoom moves the map, **Then** the shading moves with the map and stays over
   the same places, at every frame.
3. **Given** the page left open, **When** time passes, **Then** the shading moves west with the
   Sun, without a reload and with no visible jump.
4. **Given** QSOs on the night side, **When** the grayline is drawn, **Then** their markers,
   paths and popups are as easy to see and click as on the day side.
5. **Given** each theme (light, dim, dark), **When** the grayline is drawn, **Then** night and
   twilight are told apart from day, and the theme's look is kept.
6. **Given** the grayline switched off in the settings panel, **When** the browser is reopened,
   **Then** it is still off, on both views.
7. **Given** the globe with the grayline and the atmosphere on, **When** it is shown, **Then** the
   night shading on the Earth and the glow's bright side agree: the glow is brightest over the
   unshaded (day) side.
8. **Given** the grayline on and the atmosphere off (or the reverse), **When** the globe is shown,
   **Then** only the chosen one is drawn.
9. **Given** a first visit, **When** the map is shown, **Then** no Sun symbol is drawn. **Given**
   the "Sun position" choice switched on, **When** either view is shown, **Then** a small Sun
   symbol marks the point where the Sun is overhead, it moves with the Sun as time passes, and
   the choice is still on after the browser is reopened.

---

### Edge Cases

- **Globe zoomed far in** (the limb is off screen): there is no glow on screen, as today. When
  the viewer zooms back out, the glow is attached to the Earth at once, with no jump.
- **Switch from flat to globe**: the glow appears already attached to the Earth, in the right
  place for the view on screen.
- **Theme change** (light, dim, dark, or the sun-based automatic switch): the glow keeps its
  place on the Earth and only its colours change.
- **Atmosphere switched off and on again**: the glow comes back in the right place for the view
  on screen, not in a stale position.
- **Window resized** or the page opened in a narrow window: the glow stays attached to the
  Earth.
- **Page left open for hours** (a whole contest): the bright side follows the Sun as it moves
  west, without a reload, and without visible jumps.
- **Polar day and polar night** (near the solstices): the shaded area reaches a pole, and the
  terminator curves past the pole instead of crossing it. The shading stays correct, without
  gaps or stray stripes, near the top and bottom of the map.
- **Equinox**: the terminator runs almost north–south. It is drawn without breaks.
- **Date line and repeated worlds**: when the flat map is zoomed out far enough to show more
  than one copy of the world, or panned across the date line, every copy shows the same
  shading, with no seam at ±180°.
- **Zoomed in on one region**: the shading still matches there. A region fully in night is
  shaded evenly, and one fully in day isn't shaded at all.
- **Switch between globe and flat**: the grayline covers the same places in both projections
  at once. On the globe the glow is drawn too, for the same Sun.
- **Globe, night side facing the viewer**: the visible half of the Earth may be almost all
  shaded. Markers, paths and the house marker stay readable on it.
- **Sun symbol on the far side of the globe**: when the overhead point is on the hidden half of
  the Earth, no Sun symbol is drawn. It shows again when that half turns into view.
- **Sun symbol under a QSO marker**: the QSO marker stays on top and clickable. The Sun symbol
  is never clickable and never hides a QSO.
- **Fast, repeated moves** (a pile-up of QSOs with auto-zoom on, or a quick flick by hand): the
  glow never lags visibly behind the Earth.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On the globe with the atmosphere on, the glow MUST follow the Earth. When the
  Earth turns on screen, its bright side MUST move with it, staying on the edge nearest the Sun.
  It MUST NOT stay at a fixed place in the window.
- **FR-002**: FR-001 MUST hold for every way the view changes: dragging, rotating, tilting,
  zooming (mouse wheel, pinch, buttons, double click), auto-zoom moves, and the page's own
  moves (for example, panning to show a clicked QSO).
- **FR-003**: FR-001 MUST hold during animated moves, at every frame, not only when the move
  ends.
- **FR-004**: The bright side of the glow MUST face the real Sun: it is brightest on the edge of
  the globe towards the point where the Sun is overhead at the current time (UTC), as that point
  lies on screen, and it MUST follow that point as it moves through the day, while the page
  stays open. When the Sun is straight in front of or behind the globe, the glow is even all
  round. The Sun position MUST be the
  same one the sun-based automatic theme uses (feature 011), so the two never disagree.
- **FR-005**: At every zoom level where the limb is visible, the glow MUST line up with the limb
  of the Earth, with no gap or overlap a viewer can see.
- **FR-006**: The feature 011 rules for the atmosphere glow stay as they are: on by default, the
  viewer's on/off choice is saved in the browser, the glow isn't drawn on the flat map, and each
  theme has its own colours (feature 011, FR-010 and FR-011). The grayline (FR-009 to FR-018) is a
  separate layer with its own choice, not part of the glow.
- **FR-007**: Following the Earth MUST NOT make the map slower to move. Panning, zooming and
  auto-zoom moves MUST stay as smooth as with the atmosphere off.
- **FR-008**: The release-only real-browser tests (feature 012) MUST check that the glow moves
  with the Earth, after a manual move and after an auto-zoom move.

**Grayline (User Story 3)**

- **FR-009**: The page MUST draw the grayline on both the flat map and the globe, unless the
  viewer switched it off. A "Grayline" choice in the settings panel's Map group turns it on and
  off for both views. It is on by default, and the choice is saved in the browser and survives a
  restart. It is separate from the atmosphere choice: each can be on or off whatever the other
  is.
- **FR-010**: The shading at each place MUST follow the Sun's height there, as a smooth gradient
  with no hard edges:
  - **Day:** no shade where the Sun is at or above the horizon (0°).
  - **Twilight:** the shade grows continuously as the Sun goes down, so the grayline (civil
    twilight, the Sun between 0° and 6° below the horizon, the thresholds of feature 011's
    sun-based theme) is the lighter part of the fade.
  - **Night:** full night shade from 12° below the horizon (the end of nautical twilight) and
    lower, so the fade is wide enough to see at a world view.
- **FR-011**: The grayline MUST use the same Sun position as the globe's glow and the sun-based
  theme (FR-004), at the current UTC time. It MUST follow the Sun while the page stays open,
  with no visible jump.
- **FR-012**: The grayline MUST be attached to the map. It MUST stay over the same places during
  and after pans, zooms, rotations, tilts, auto-zoom moves and the page's own moves. On the flat
  map this holds on every copy of the world it shows, with no seam at the date line.
- **FR-013**: QSO markers, QSO paths, the house marker, their popups and the map's controls MUST
  stay above the grayline, with the same contrast and clickability as in day. The base map's
  borders and its names of places (cities, towns, villages, states, countries, continents) MUST
  also stay above it, at their day contrast. The shading darkens the base map's land, sea,
  roads and buildings. In some themes, names of seas, rivers and roads are drawn under the
  shading, slightly darkened, and stay readable.
- **FR-014**: Each theme MUST have its own grayline shading, readable in that theme: night and
  twilight visible against day in light, dim and dark.
- **FR-015**: On the globe, the grayline MUST lie on the Earth's surface, under the atmosphere
  glow. It MUST turn with the Earth under every move, at every frame, as the glow does (FR-001
  to FR-003). The night shading and the glow's bright side MUST agree: the glow is brightest
  over the day side.
- **FR-016**: Drawing the grayline MUST NOT make either view slower to move (as FR-007 for the
  glow).
- **FR-017**: The release-only real-browser tests (feature 012) MUST check, on the flat map and on
  the globe, that night is shaded on the side away from the Sun and day isn't shaded on the side
  facing it, and that the shading stays over the same places after a move. They MUST also check
  that the Grayline and Sun position choices survive a reload, and that the Sun symbol is drawn
  only when its choice is on.
- **FR-018**: A "Sun position" choice in the settings panel's Map group MUST show or hide a small
  Sun symbol at the point where the Sun is overhead, on both views. It is off by default, and the
  choice is saved in the browser and survives a restart. It is independent of the Grayline and
  atmosphere choices. The symbol MUST use the same Sun position (FR-004) and follow it as time
  passes. It is drawn below QSO markers, paths and the house marker, is not clickable, and is
  distinct from every QSO and house marker in every theme.
- **FR-019**: The atmosphere MUST NOT reduce the visibility of the map's data (clarification,
  implementation session). It is a thin glow along the limb only, never a haze over land, sea,
  QSO markers, QSO paths or the house marker.

### Key Entities

- **Sun position**: the point on the Earth where the Sun is overhead at the current UTC time.
  The glow, the grayline, the Sun symbol and the sun-based theme all use it.
- **Grayline choice**: on (default) or off, saved in the browser. It controls the grayline on
  both views.
- **Sun position choice**: off (default) or on, saved in the browser. It shows the Sun symbol on
  both views.
- **Atmosphere choice** (feature 011, unchanged): on (default) or off. It controls only the glow
  on the globe.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After the globe is turned 180° by hand, the bright side of the glow is next to the
  same part of the Earth as before the move, in 100% of trials. Today it stays in the same
  corner of the window.
- **SC-002**: For the same final view, the glow after an auto-zoom move and after a manual move
  is the same, in 100% of trials.
- **SC-003**: During an animated auto-zoom move, the glow is next to the same part of the Earth
  in every captured frame. There is no frame where the glow and the Earth are out of step.
- **SC-004**: On real display hardware, the map moves as smoothly with the atmosphere, the
  grayline and the Sun symbol all on as with all three off: a viewer comparing the two can't
  tell them apart (measured during implementation: 60 frames/s in every case). The release test
  records the frame rates of a scripted 2-second drag in its report, but doesn't judge them: its
  browser renders in software, where every full-screen layer costs CPU (owner's decision,
  implementation session).
- **SC-005**: At any time, the brightest part of the glow is on the edge of the globe towards the
  point where the Sun is overhead, within 1° of that point's direction on screen, whenever the
  point is more than 20° from the view centre and from its antipode. The side of the Earth in
  daylight is on the bright side.
- **SC-011**: Away from the limb (more than 3 % of the globe's radius inside it), the map's
  colours differ by at most 2 % with the atmosphere on and off.
- **SC-006**: On both views, the place where the shading starts (the Sun at 0°) and the place
  where it reaches full night (the Sun at −12°) are each within 1° (about 110 km) of a reference
  Sun-height calculation for the same UTC time, everywhere outside the polar caps. Between
  them, the shade only grows from day towards night, with no visible step.
- **SC-007**: After a pan or zoom of either view, the shading covers the same places as before,
  to within one screen pixel at the end of the move.
- **SC-008**: Every QSO marker on the night side can be seen and clicked as easily as one on the
  day side, in all three themes. Every city, state or country name readable on the day side is as
  readable on the night side.
- **SC-009**: An operator shown either view can tell, without explanation, which part of the
  world is in day, in twilight and in night.
- **SC-010**: A viewer who sees the globe for the first time, shown it before and after the
  fix, says the fixed atmosphere looks like it belongs to the Earth.

## Assumptions

- "Moves with the globe" covers every change of view on the globe (pan, rotation, tilt, zoom),
  not only zoom, as the owner's examples are a manual drag and an auto-zoom move.
- The fix and the grayline are made in the page. The protocol and the service's data don't
  change; the service only serves one more page file.
  Two viewer settings are added, the Grayline and Sun position choices, saved in the browser
  (constitution, Principle IV).
- The atmosphere's colours, its thickness and the sky behind the globe stay as feature 011 set
  them. Only where the glow sits, and which side is brighter, changes.
- The Sun position updates often enough that its drift (about 15° of longitude per hour) never
  shows as a jump; a refresh about once a minute is enough.
- The flat map keeps having no atmosphere glow (feature 011, FR-011). The grayline is shown on both
  views.
- The grayline is the civil twilight band, the Sun between 0° and 6° below the horizon, as in the
  sun-based theme. The shading fades on to full night at −12°, so nautical twilight is the darker
  part of the fade. Astronomical twilight (to −18°) is shaded as night.
- Auto-zoom frames the house and the QSOs only, as before. The grayline and the Sun symbol don't
  change what it frames.
- No grayline-based propagation advice is given: the map shows where the grayline is, and the
  operator draws the conclusions.
- "Grayline" is added to the glossary (`docs/ham-radio.md` §2) while this feature is planned.
- A check by eye in a real browser (Chrome, the browser of feature 012) is the reference. The
  automated check compares screenshots of the glow before and after a move.
