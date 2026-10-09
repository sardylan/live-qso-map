# Feature Specification: QTH Marker, Auto-Zoom and Map Panels

**Feature Branch**: `004-qth-autozoom`
**Created**: 2026-10-09
**Status**: Complete (2026-10-09). See `checklists/verification.md`.
**Input**: User description: "Create a new spec for the frontend: show the QTH location with a house as
icon (use common icons or emoji) with position of the local station; display a geodesic line that
connects the local station with each remote station, using black color; show a button to enable
auto-zoom in order to display all markers automatically. If there are no QSOs (only QTH), it must zoom
enough to display the entire ham country; preserve globe or flat mode, if possible. If it is not
possible to display all markers in globe mode, switch to flat mode, but only if auto-zoom is enabled;
display on the bottom the scale which shows a standard size in meters or kilometers according to the
zoom (rescaling automatically with the zoom), only in flat mode; add a box on the bottom left part of
the map with the list of the last 5 QSOs, which can be disabled with a button; when computing auto-zoom
consider all items in the page, like the QSO box (if enabled), legend, etc."

## Clarifications

### Session 2026-10-09

- Q: Where does the QTH position come from? → A: The configured station position; if none is
  configured, the station position of the most recent QSO. One house; every line starts there.
- Q: With no QSOs, how is the station's entity framed? → A: A bundled table of approximate extents
  for DXCC entities; a fixed radius around the QTH when the entity is missing from the table.
- Q: How does the auto-zoom button behave? → A: A toggle. While on, the map refits after every change;
  any manual pan or zoom switches it off.
- Q: Should the error responses that break the new interface rules (constitution 1.3.0) be fixed
  here? → A: Yes, they are folded into this feature (FR-027).
- Q: What are the defaults when a page opens? → A: Auto-zoom and the last-QSO box both start on, and
  each viewer's choices are remembered in their browser.
- Q: What does "most recent" mean in the last-QSO box? → A: By QSO time (UTC), the same order as the
  limit selector. A late-arriving old QSO does not enter the box.
- Q: What happens to auto-zoom while a popup is open? → A: Refits pause while a popup is open, and
  one catch-up refit runs when it closes. A popup with no viewer interaction closes after 30 s.
- Q: What happens when a row in the last-QSO box is clicked? → A: That QSO's marker popup opens. The
  map pans only if the marker is off-screen, and auto-zoom pauses as for any popup rather than
  switching off. Rows without a remote position are not clickable.
- Q: Where is the globe centred during auto-zoom? → A: On the point that best frames the house and
  all markers. The map switches to flat only when no centre can show them all.
- Q: (Owner's browser test.) After an automatic switch to flat, the projection button still offered
  "Flat". What should the button show and do? → A: It follows the view on screen. Clicking "Globe"
  then shows the globe; if the QSOs cannot fit there, auto-zoom switches off so the globe stays.
- Q: (Owner, follow-up.) Should every Globe/Flat switch turn auto-zoom off? → A: Yes. Any switch
  with the projection button turns auto-zoom off.
- Q: The view was switched to flat automatically and auto-zoom turns off. What happens to the
  view? → A: It stays as it is. The automatic flat lasts until the viewer uses the projection
  button or turns auto-zoom back on, and a reload restores the saved preference.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the QTH and the paths from it (Priority: P1)

An operator or visitor opens the map during a contest. A house icon marks the station's own location
(the QTH). Every worked station is joined to the house by a black great-circle line, so the paths
stand out the same way on any base map, while the markers keep their band colours.

**Why this priority**: the QTH is the reference point of the whole map. The lines and auto-zoom
(US2) both depend on it.

**Independent Test**: configure a station position, send three QSOs on different bands, and open the
page. A house appears at the configured position, and three black lines run from it to three
band-coloured markers.

**Acceptance Scenarios**:

1. **Given** a configured station position and no QSOs, **When** a page opens, **Then** a house icon
   is shown at that position.
2. **Given** a configured station position, **When** QSOs arrive, **Then** each remote marker is
   joined to the house by a black great-circle line, and the markers keep their band colours.
3. **Given** no configured station position, **When** the first QSO that carries a station position
   arrives, **Then** the house appears at that position; when a later QSO carries a different
   station position, the house moves there.
4. **Given** no configured station position and no QSO with one, **When** a page is open, **Then**
   no house and no lines are shown, and remote markers are still shown.
5. **Given** a house on the map, **When** the viewer clicks it, **Then** a popup shows the station
   callsign (when known), its locator or coordinates, and where the position came from.

---

### User Story 2 - Auto-zoom keeps everything in view (Priority: P1)

A viewer leaves the map on a wall screen. With auto-zoom on, the map keeps re-framing itself so the
house and every displayed QSO stay visible and nothing hides behind the legend, the last-QSO box or
the controls. At the start of a contest, with no QSOs yet, the map frames the station's own entity.

**Why this priority**: an unattended display is the main use of the map during a contest, and today
it shows a fixed world view.

**Independent Test**: with auto-zoom on, send QSOs to stations on several continents. After each one
the map refits so that every marker and the house are visible and none is covered by a panel.

**Acceptance Scenarios**:

1. **Given** auto-zoom on and a house with no QSOs, **When** the page opens, **Then** the map frames
   the whole entity of the station (for example Sardinia for IS0MZK).
2. **Given** auto-zoom on, **When** a QSO is added, corrected or removed, the displayed limit
   changes, a panel is shown or hidden, or the window is resized, **Then** the map refits so the
   house and every displayed marker are visible.
3. **Given** auto-zoom on, **When** the map is fitted, **Then** no marker and no house lies under
   the legend, the last-QSO box (when shown), the controls or the attribution.
4. **Given** auto-zoom on, **When** the viewer pans, zooms or rotates the map by hand, **Then**
   auto-zoom switches off and the button shows it as off.
5. **Given** auto-zoom on and a marker popup open, **When** QSOs arrive, **Then** the map does not
   move while the popup is open. When it closes, by the viewer or after 30 s without interaction,
   the map refits once.
6. **Given** auto-zoom off, **When** the viewer clicks the button, **Then** auto-zoom switches on and
   the map refits at once.
7. **Given** a viewer who turned auto-zoom off, **When** the page is reloaded in the same browser,
   **Then** auto-zoom is still off.

---

### User Story 3 - Globe or flat, chosen sensibly (Priority: P2)

The viewer prefers the globe. Auto-zoom keeps the globe while every marker can be seen on it, and
moves to the flat map only when the QSOs span more of the Earth than one side of the globe can show.
When they fit again, the globe comes back.

**Why this priority**: it keeps the viewer's choice whenever possible and still lets auto-zoom show
everything during a DX-heavy contest.

**Independent Test**: choose the globe and turn auto-zoom on. Add a QSO in Japan and one in
California from a European QTH. The map switches to flat to show them all, then returns to the globe
when those QSOs are removed.

**Acceptance Scenarios**:

1. **Given** the globe preference and auto-zoom on, **When** the house and every displayed marker fit
   on the visible side of the globe, **Then** the map stays a globe.
2. **Given** the globe preference and auto-zoom on, **When** they no longer fit on the visible side,
   **Then** the map switches to flat and frames them all.
3. **Given** that automatic switch to flat, **When** they fit on the globe again, **Then** the map
   returns to the globe.
4. **Given** auto-zoom off, **When** markers lie outside the visible side of the globe, **Then** the
   projection never changes by itself.
5. **Given** an automatic switch, **When** the page is reloaded, **Then** the viewer's saved
   preference is still the one they chose by hand; automatic switches are never saved.
6. **Given** the flat preference, **When** auto-zoom fits the map, **Then** it never switches to the
   globe.
7. **Given** an automatic switch to flat, **When** the viewer drags the map, **Then** auto-zoom
   turns off and the map stays flat. It returns to the globe only through the projection button,
   by turning auto-zoom back on (when everything fits), or by a reload.

---

### User Story 4 - Last-QSO box (Priority: P2)

A box at the bottom left lists the five most recent QSOs: callsign, band, mode and UTC time, newest
first. Spectators see at a glance what has just been worked. A button hides or shows the box.

**Why this priority**: it adds context on a wall screen. The map is usable without it.

**Independent Test**: send seven QSOs. The box lists the last five, newest first. Hide it with the
button and reload: it stays hidden.

**Acceptance Scenarios**:

1. **Given** the box is shown, **When** QSOs arrive, **Then** it lists the five most recent displayed
   QSOs, newest first, each with callsign, band, mode (and submode when present) and UTC time.
2. **Given** a QSO in the box is corrected or removed, **When** the change arrives, **Then** the box
   shows the corrected QSO or drops the removed one and moves the next one up.
3. **Given** fewer than five QSOs, **When** the box is shown, **Then** it lists those it has; with
   none, it says that no QSOs have been received yet.
4. **Given** the box is shown, **When** the viewer clicks the hide button, **Then** the box
   disappears; clicking again shows it; the choice is remembered in that browser.
5. **Given** QSO text from the network, **When** the box shows it, **Then** it appears as plain text
   and never as markup.
6. **Given** a row of a QSO with a remote position, **When** the viewer clicks it, **Then** that
   marker's popup opens. The map pans only if the marker was hidden. Auto-zoom stays on, paused
   until the popup closes.

---

### User Story 5 - Distance scale in flat mode (Priority: P3)

In flat mode, a scale bar at the bottom shows a round distance in metres or kilometres and adjusts
as the viewer zooms. On the globe, where one scale cannot be correct across the view, it is hidden.

**Why this priority**: useful for judging distances, but not needed during normal viewing.

**Independent Test**: switch to flat and zoom from the whole world down to a city. The scale changes
its length and label (for example 2000 km, 500 km, 10 km, 500 m) and always stays readable. Switch to
globe: it disappears.

**Acceptance Scenarios**:

1. **Given** flat mode, **When** the page is shown, **Then** a scale bar is shown at the bottom with
   a round value in metric units (m below 1 km, km above).
2. **Given** flat mode, **When** the viewer zooms or pans, **Then** the scale updates to the new
   view.
3. **Given** the globe, whether by preference or after an automatic switch back, **When** the page is
   shown, **Then** the scale is hidden.

---

### Edge Cases

- **QSOs without a remote position**: they appear in the last-QSO box but not on the map, so
  auto-zoom ignores them.
- **QTH unknown**: auto-zoom fits only the remote markers. With no markers and no house, it shows the
  whole world.
- **Station entity unknown, or not in the bundled table**: with no QSOs, auto-zoom frames a fixed
  radius around the house.
- **Single marker plus the house very close together**: auto-zoom stops at a maximum zoom level, so
  the map is never zoomed into a single street.
- **Panels taking most of a small screen**: the free area left for fitting is never smaller than a
  minimum size. If panels leave less than that, auto-zoom fits into the whole map and some panels
  may cover markers.
- **Paths crossing the antimeridian** (for example Europe to New Zealand): the lines and the fit both
  follow the short great-circle path, not the long way around the flat map.
- **Popup open during changes**: refits pause, so the viewer can read it. When the popup's marker is
  removed, the popup closes (feature 003) and the catch-up refit runs.
- **A burst of QSOs**: refits are coalesced, so the map does not jump on every message.
- **Late QSO** (arrives after newer ones, with an older QSO time): it is shown on the map, but it
  enters the last-QSO box only if it is among the five newest by QSO time.
- **Reconnection**: auto-zoom, box visibility and the projection preference are kept, and the box
  refills from the snapshot.
- **The base map fails to load**: the house, lines, box, scale and auto-zoom still work.
- **QTH from a QSO under a band filter** (feature 003): only QSOs on considered bands reach the page,
  so only they can set the house when no station position is configured.

## Requirements *(mandatory)*

### Functional Requirements

**QTH and lines**

- **FR-001**: The page MUST show a house icon at the QTH whenever the QTH is known, including before
  any QSO arrives. The icon must be a widely recognised house symbol, readable on any base map.
- **FR-002**: The QTH MUST be the configured station position. When none is configured, it MUST be
  the station position of the most recent QSO on the page that carries one. It MUST update when that
  changes.
- **FR-003**: When the QTH is unknown, the page MUST show no house and no lines, and remote markers
  MUST still be shown.
- **FR-004**: Every remote marker MUST be joined to the QTH by a black great-circle line
  (`docs/ham-radio.md` §2.10). These lines replace the band-coloured lines of feature 001, and the
  markers keep their band colours.
- **FR-005**: The house and lines MUST be drawn so that remote markers stay clickable on top of them.
- **FR-006**: Clicking the house MUST show the station callsign (when known), the QTH as a locator
  or coordinates, and the source of the position (HR-7), all as plain text.

**Auto-zoom**

- **FR-007**: The page MUST offer an auto-zoom toggle button. It MUST start on for a viewer who has
  never used it, and each viewer's choice MUST be remembered in their browser.
- **FR-008**: While auto-zoom is on, the map MUST fit the house and every displayed remote marker.
  It MUST refit on every displayed-data change: QSO added, corrected or removed, limit changed,
  snapshot received, QTH changed. It MUST also refit when a panel is shown or hidden and when the
  window is resized.
- **FR-009**: The fit MUST keep a margin so that nothing it frames lies under any visible page
  element: legend, last-QSO box, limit selector, map controls, attribution and status banners.
  The only exception is the small-screen edge case: when panels leave a free area smaller than the
  minimum, the fit uses the whole map.
- **FR-010**: With a known QTH and no displayed remote markers, auto-zoom MUST frame the extent of
  the station's DXCC entity (HR-6). The entity comes from the station callsign and the bundled
  extent table. It MUST frame a fixed radius around the QTH when the entity is unknown or missing
  from the table.
- **FR-011**: Any manual pan, zoom, rotation or pitch by the viewer MUST switch auto-zoom off and
  show it as off. Changes the page makes itself MUST NOT switch it off.
- **FR-012**: Switching auto-zoom on MUST refit the map immediately.
- **FR-012a**: Opening a popup (a marker's or the house's) MUST NOT switch auto-zoom off.
  - While a popup is open, refits MUST pause. Changes keep updating the markers, lines, box and the
    popup content.
  - When the popup closes, one catch-up refit MUST run if auto-zoom is still on.
  - While auto-zoom is on, a popup with no viewer interaction for 30 s MUST close by itself. With
    auto-zoom off, popups never close by themselves.
- **FR-013**: Auto-zoom MUST never zoom in beyond a maximum level (city scale), whatever it frames.
- **FR-014**: Refits MUST be coalesced so that a burst of changes causes at most one refit
  animation per second.

**Projection**

- **FR-015**: Auto-zoom MUST keep the viewer's projection preference whenever everything it frames
  fits on it.
- **FR-016**: With the globe preference and auto-zoom on, when the framed items cannot all be seen
  on the visible side of the globe at once, the page MUST switch to flat. It MUST return to the
  globe as soon as they fit again.
  - On the globe, auto-zoom MUST centre on the point that best frames the framed items, not
    necessarily the QTH.
  - "Cannot all be seen" means no centre exists from which they all lie on the visible side.
- **FR-017**: The page MUST NOT change the projection by itself when auto-zoom is off, and it MUST
  NOT switch a flat preference to the globe. Turning auto-zoom off, by the button or by a manual
  move, MUST NOT change the projection. An automatic flat view stays until the viewer uses the
  projection button or turns auto-zoom back on.
- **FR-018**: Automatic projection switches MUST NOT change the saved preference. The projection
  button MUST always offer the view that is not on screen, automatic switches included. Clicking it
  shows that view and saves it as the preference (feature 002). Any switch by the button MUST also
  turn auto-zoom off, as a manual move does.

**Last-QSO box**

- **FR-019**: When enabled, a box at the bottom left of the map MUST list the five most recent
  displayed QSOs, newest first. "Most recent" means by QSO time (UTC), the order the limit selector
  uses. A QSO that arrives late with an older time enters the box only if it is among the five
  newest; a correction that changes the time moves its row accordingly. Each row shows the callsign, band, mode (with submode when present)
  and UTC time (HR-1). The box MUST NOT cover the attribution.
- **FR-020**: The box MUST follow every change: new QSOs, corrections, removals, snapshots and limit
  changes. With no QSOs, it MUST say that none have been received yet.
- **FR-021**: A button MUST hide and show the box. The box MUST start shown for a viewer who has
  never used the button, and each viewer's choice MUST be remembered in their browser.
- **FR-022**: QSO data in the box MUST be rendered as text only, never as markup.
- **FR-022a**: Clicking a row of a QSO with a remote position MUST open the popup of that QSO's
  marker.
  - The map MUST pan only when the marker is outside the free map area (FR-009), and only enough
    to show it and its popup.
  - That pan is made by the page, so it MUST NOT switch auto-zoom off (FR-011). Auto-zoom pauses
    as for any open popup (FR-012a).
  - Rows of QSOs without a remote position MUST NOT look or act clickable.

**Scale**

- **FR-023**: In flat mode, the page MUST show a scale bar at the bottom of the map. Its length
  matches a round distance in metric units: metres below 1 km, kilometres from 1 km.
- **FR-024**: The scale MUST update as the view changes. It MUST be hidden whenever the map shows
  the globe, whether by preference or automatic switch.

**General**

- **FR-025**: The service MUST give each page the configured station position and callsign, when
  they exist, before any QSO arrives. It MUST also give the extent of the station's entity, when
  one is known.
- **FR-026**: Features 001–003 MUST keep working unchanged, apart from the line colour (FR-004):
  markers, popups, the limit selector, the band filter and legend, the projection button, the base
  map and the no-WebGL message.
- **FR-027**: Every error response of the service MUST have a JSON body
  `{"error": "<code>", "message": "<text>"}` (`docs/http-api.md`, API-3). This covers unknown
  paths, methods that are not allowed, and failed WebSocket upgrades on `/ws`; today these
  responses have an empty or plain-text body. New or changed messages (FR-025) MUST follow API-2
  and API-4.

### Key Entities

- **QTH**: the station's own location: a position with precision and source, plus the station
  callsign when known. It comes from the configuration, or from the most recent QSO that carries a
  station position.
- **Entity extent**: an approximate bounding area of a DXCC entity, keyed by the entity (for example
  its prefix in the country data). It is bundled with the service, not fetched.
- **Auto-zoom state**: per viewer: on or off (remembered), plus whether the page is currently
  showing flat because of an automatic switch (not remembered).
- **Last-QSO box state**: per viewer: shown or hidden (remembered). Its content is derived from the
  displayed QSOs.
- **Framed set**: the items auto-zoom must keep visible: the house and every displayed remote marker.
  When there are no markers, it is the entity extent or the fixed radius.
- **Obstructed area**: the parts of the map covered by visible page elements, which the fit keeps
  clear.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a configured station position, a page that opens with no QSOs shows the house
  within 2 s of loading.
- **SC-002**: With auto-zoom on, after any change the house and 100 % of the displayed markers are
  inside the visible map and outside every visible panel within 2 s. This holds in a test with
  QSOs on every continent, on window sizes from 1024×600 to 3840×2160.
- **SC-003**: With auto-zoom on and no QSOs, the station's whole entity is visible and fills at
  least half of the free map width or height. Checked for IS0 (Sardinia), I (Italy), W (USA) and
  VK (Australia).
- **SC-004**: A single manual pan or zoom always switches auto-zoom off: 0 refits follow it until
  the viewer switches auto-zoom back on. With auto-zoom on, the map moves 0 times while a popup is
  open, and refits exactly once after it closes.
- **SC-005**: With 2,000 QSOs displayed, a stream of 10 QSOs per second causes at most one refit
  per second, and the map stays interactive.
- **SC-006**: Under the globe preference, the projection changes only when auto-zoom is on and the
  framed items cannot fit on the globe. In a scripted run of 50 changes, every switch matches that
  rule, and a reload always restores the hand-chosen preference.
- **SC-007**: The last-QSO box matches the five most recent displayed QSOs after every change in a
  1,000-QSO test stream, including corrections and removals.
- **SC-008**: In flat mode, the scale's labelled distance is within 5 % of the true distance it
  spans at the map centre, at 10 zoom levels from world to city.

## Assumptions

- **Icon and line style**:
  - the house uses a common house emoji or symbol icon from the page itself, with no new external
    service;
  - "black" lines may use slight transparency, so that overlapping paths remain distinguishable.
- **Entity extents**:
  - a static, approximate table, one bounding box per DXCC entity, shipped with the service;
  - it is matched to the entity that the existing country data resolves for the station callsign;
  - entities that span the antimeridian are handled;
  - the default fixed radius when the entity is missing is 500 km around the QTH.
- **"Fits on the globe"**: some centre exists from which every framed item lies on the visible side
  of the globe, with the same margins as FR-009. Auto-zoom uses that centre (FR-016).
- **What gets framed**:
  - "displayed" means the QSOs shown under the viewer's limit selector and the band filter;
  - QSOs without a remote position are not framed.
- **Box contents**: the box shows the last five displayed QSOs, so the limit selector and band filter
  apply to it too. The count of five is fixed.
- **Units**: the scale uses metric units only; there are no imperial units.
- **Remembered choices**: like the feature-002 projection preference, choices are kept in the
  browser's local storage. If storage is unavailable, each page load uses the defaults.
- **Protocol**: the station position, callsign and entity extent reach the page in the opening
  handshake. This is a protocol change, versioned the same way as in feature 002.
- **Out of scope**:
  - server-side rendering;
  - per-viewer choice of line colour;
  - a configurable number of rows in the box;
  - range rings or beam headings.
