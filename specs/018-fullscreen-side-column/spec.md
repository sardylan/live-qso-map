# Feature Specification: Full-Screen Side Column

**Feature Branch**: `feature/018-fullscreen-side-column`
**Created**: 2026-10-25
**Status**: Draft
**Input**: User description: "New features for the frontend: review font sizes for full screen view
on monitors or TV screens; move all elements on the bottom left in a permanent full-height column on
the left; move the actual first line of info with actual operator callsign and band on top of the
left column, and add current UTC time, everything with a bigger font size; move the actual row with
scores just below the first line in the left column, with a bigger font size, but smaller than the
first line; add the software version tag on the bottom of the column; add a feature for forcing
reloading the whole page if the version of the backend is different, on the websocket reloading, in
order to have all connected frontend refresh the page automatically if the backend was updated."

## Problem Summary

The map is often shown full screen on a monitor or a TV in the shack or at a contest site, read
from a few metres away. Today the station status line (feature 017), the score line (feature 007)
and the last QSOs box (features 005 and 006) float over the bottom left corner of the map in a
small desktop font: hard to read from afar, and they cover part of the map.

Unattended screens also keep running an old page after the service is updated: they never pick up
the new page until someone reloads them by hand.

After this feature, the page has a permanent full-height column on the left, beside the map. From
top to bottom it shows the station status (band, operator, UTC time) in the largest type, the score
below it, the last QSOs box, and the software version at the very bottom. Text sizes suit a large
screen read from a distance. A page that reconnects to an updated service reloads itself.

## Clarifications

### Session 2026-10-25

- Q: Should the left column sit beside the map or over its left edge? → A: Beside the map; the
  map takes the remaining width and nothing is hidden.
- Q: What does the column do on windows narrower than 600 px? → A: It becomes a strip above the
  map with status, score and version; the last QSOs box is not shown.
- Q: What does the page compare the service version against? → A: The version the page itself was
  built for; it reloads whenever the service reports a different one, on its first connection too.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the station status from across the room (Priority: P1)

A viewer looks at a TV showing the map full screen. At the top of a column on the left they read,
in large type, the band the station is on, the operator at the radio and the current UTC time.
Just below, in smaller but still large type, they read the score.

**Why this priority**: The status and score are what people look at most; making them readable
from a distance is the core of the request.

**Independent Test**: Open the page full screen on a 1920×1080 screen with QSOs and a score
received, and read the column from 3 metres.

**Acceptance Scenarios**:

1. **Given** the page is open and the latest QSO was on 40m by IK0ABC, **When** the page is shown,
   **Then** the top of the left column shows `40m`, `IK0ABC` and the current UTC time, with band
   and operator in the largest text on the page.
2. **Given** the page is open, **When** a minute passes, **Then** the UTC time shown follows the
   UTC clock of the viewer's device without a reload.
3. **Given** a score was received, **When** the page is shown, **Then** the score line sits just
   below the status block, in text smaller than the status block and larger than the last QSOs box.
4. **Given** the station status or the score is old (features 017 and 007), **When** the page is
   shown, **Then** it is greyed as today; the UTC time is never greyed.

---

### User Story 2 - A permanent column beside the map (Priority: P1)

Every element that sat over the bottom left corner of the map now lives in a full-height column on
the left. The map fills the rest of the window and nothing covers its left side.

**Why this priority**: The column is where every other change of this feature lives.

**Independent Test**: Open the page at several window sizes and check that the column runs from
the top to the bottom of the window and the map starts at its right edge.

**Acceptance Scenarios**:

1. **Given** the page is open, **When** it is shown, **Then** a column runs the full height of the
   window on the left, and the map fills the remaining width without being covered by it.
2. **Given** the last QSOs box is shown (settings), **When** the page is shown, **Then** it sits in
   the column below the score line.
3. **Given** the viewer hides the last QSOs box in the settings, **When** the page is shown,
   **Then** the column stays, with the status, the score and the version tag.
4. **Given** the window is resized, **When** the map redraws, **Then** auto-zoom (features 004
   and 016) frames the QSOs in the map area beside the column, not behind it.
5. **Given** a QSO row in the last QSOs box is clickable (feature 005), **When** it is clicked,
   **Then** it behaves as today.

---

### User Story 3 - Text sized for monitors and TVs (Priority: P2)

On a large screen, every text on the page (column, legend, popups, banners, settings panel) is big
enough to read at a glance, and grows with the screen rather than staying at a fixed desktop size.

**Why this priority**: Improves every element, but the column (P1) already covers the most-read
ones.

**Independent Test**: Open the page full screen on a 1920×1080 screen and on a 3840×2160 screen
and compare text sizes relative to the screen height.

**Acceptance Scenarios**:

1. **Given** the page is full screen at 1920×1080, **When** it is shown, **Then** the legend and
   the last QSOs box are readable from 3 metres.
2. **Given** the same page on a 3840×2160 screen at the same physical size, **When** it is shown,
   **Then** text takes the same share of the screen height as at 1920×1080.
3. **Given** a laptop window 1280×720, **When** the page is shown, **Then** nothing overflows or
   overlaps and the map keeps most of the window.

---

### User Story 4 - Know the software version (Priority: P3)

The bottom of the column shows the version of the running service, so whoever looks at the screen
can tell which version it runs.

**Why this priority**: Useful for support and to confirm an update, but not watched during a
contest.

**Independent Test**: Open the page and compare the tag with the version the service reports at
startup.

**Acceptance Scenarios**:

1. **Given** the service runs version 3.3.0, **When** the page is shown, **Then** the bottom of the
   column shows `v3.3.0`, in small muted text.

---

### User Story 5 - Screens update themselves after a service update (Priority: P2)

The operator updates and restarts the service. Every open page, including unattended screens,
reconnects, notices the new version and reloads itself, so it runs the new page without anyone
touching it.

**Why this priority**: Unattended screens otherwise run a stale page forever; it matters at every
update but not during normal operation.

**Independent Test**: Open two pages, restart the service with a different version, and watch both
reload on their own.

**Acceptance Scenarios**:

1. **Given** a page loaded from version 3.3.0, **When** the service restarts as version 3.4.0 and
   the page reconnects, **Then** the page reloads itself once and then shows `v3.4.0`.
2. **Given** a page loaded from version 3.3.0, **When** the service restarts as the same version
   3.3.0 and the page reconnects, **Then** the page does not reload; it carries on as today.
3. **Given** the connection drops and comes back without a service restart, **When** the page
   reconnects, **Then** it does not reload.
4. **Given** a page reloads after an update, **When** it is open again, **Then** the viewer's
   settings (limit, box, theme, legend) are as before the reload.
5. **Given** a browser shows a cached copy of the version 3.3.0 page, **When** it connects to a
   service already running 3.4.0, **Then** the page reloads itself once and then shows `v3.4.0`.

---

### Edge Cases

- **No QSO yet**: the status block shows dashes for band and operator as today (feature 017); the
  UTC time is shown regardless.
- **No score yet**: the score line shows its placeholder as today (feature 007).
- **Very long values**: a long portable operator callsign (for example `IS0/IK0ABC/P`) or a long
  score line wraps or shrinks inside the column; the column never grows over the map and text never
  spills outside it.
- **Last QSOs box taller than the space left**: the box shows as many rows as fit and the version
  tag stays visible at the bottom.
- **Narrow windows** (below 600 px wide, phones): the column becomes a strip above the map without
  the last QSOs box (FR-005); widening the window past 600 px brings the column and the box back.
- **Device clock wrong**: the UTC time follows the device clock; a page cannot do better without a
  time source, which is out of scope.
- **Older service without a version** (a page from this feature talking to an older service, or the
  other way round): no reload is triggered by a missing version; the tag shows nothing.
- **Repeated reloads**: a page that reloads and still sees a version different from its own (for
  example a cache serving an old page) must not reload in a loop; see FR-012.
- **Settings panel open**: the panel on the right (feature 011) is unaffected by the column on the
  left.

## Requirements *(mandatory)*

### Functional Requirements

**Left column**

- **FR-001**: The page MUST show a column on the left side running the full height of the window,
  always visible, with the map filling the remaining width beside it (not behind it).
- **FR-002**: From top to bottom the column MUST hold: the station status block (band, operator,
  current UTC time), the score line, the last QSOs box (when shown), and the software version tag at
  the bottom edge.
- **FR-003**: The elements that overlaid the bottom left corner of the map (station status line,
  score line, last QSOs box) MUST no longer overlay the map.
- **FR-004**: Auto-zoom (features 004 and 016) MUST frame the QSOs and the QTH in the map area
  beside the column.
- **FR-005**: On windows narrower than 600 px the column MUST become a strip above the map holding
  the status block, the score line and the version tag; the last QSOs box MUST NOT be shown there,
  and the map fills the window below the strip.
- **FR-006**: Everything the elements did before (live updates, staleness greying, clickable QSO
  rows, settings for the box size, order and visibility) MUST keep working in the column.

**Status block, score and clock**

- **FR-007**: The status block MUST show the station's current band and operator (feature 017)
  and the current UTC time in `HH:MM:SS` followed by `UTC` (HR-1), updated every second from the
  viewer's device clock.
- **FR-008**: The status block MUST use the largest text on the page; the score line MUST use text
  smaller than the status block and larger than the last QSOs box.
- **FR-009**: Staleness greying of the status (feature 017, 15 minutes) and of the score (feature
  007) MUST stay as today; the clock is never greyed.

**Font sizes**

- **FR-010**: Text sizes across the page (column, legend, popups, banners, settings panel) MUST be
  reviewed for full-screen viewing on monitors and TVs: they MUST scale with the window size, so
  that text takes about the same share of the screen at 1920×1080 and at 3840×2160, with a lower
  bound that keeps a 1280×720 window usable.

**Version tag and automatic reload**

- **FR-011**: The service MUST tell each page its software version when the page connects, and the
  column MUST show it at the bottom as `v` followed by the version (for example `v3.3.0`).
- **FR-012**: Each page MUST carry the software version it was built for. On every connection,
  first one included, when the version the service reports differs from the page's own, the page
  MUST reload itself once. A page MUST NOT reload when the versions match, when either version is
  missing, or more than once for the same service version (no reload loop).
- **FR-013**: Adding the version to what the service sends MUST NOT break pages or services of
  older versions (API-2, API-4): the field is additive, so the protocol version does not change.
- **FR-014**: Viewer settings stored by the page (limit, box, theme, legend) MUST survive the
  automatic reload.

### Key Entities

- **Software version**: the version of the running service (for example `3.3.0`), the same as it
  reports at startup and in its release; sent to each page on connection.
- **Page version**: the software version the page itself was built for, the same for every copy
  of the page served by that release; compared with the service version at every connection.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a 1920×1080 screen viewed full screen from 3 metres, a viewer reads the band,
  operator, UTC time and score correctly in at least 9 out of 10 attempts.
- **SC-002**: Nothing in the column overlaps the map at any window size from 1280×720 to
  3840×2160.
- **SC-003**: Text in the column takes the same share of the screen height (within 10%) at
  1920×1080 and at 3840×2160.
- **SC-004**: After a service update, every open page shows the new version within 60 seconds of
  the service being back, with no human action.
- **SC-005**: A page that reconnects to the same service version never reloads (0 reloads across
  10 forced disconnections).
- **SC-006**: The UTC time shown never differs from the device's UTC clock by more than 1 second.

## Assumptions

- **Column beside the map, not over it** (clarified): the map shrinks to make room, so the column
  never hides QSOs. Its width is a plan decision, sized for the last QSOs box columns at the larger
  text.
- **Clock source**: the viewer's device clock, shown in UTC; contest stations and TVs normally run
  time-synced. Seconds are shown because the screen is a live wall display.
- **Version string**: the service's release version (as in `config.example.toml` releases, feature
  015), shown with a `v` prefix; no build hash or date.
- **Reload trigger**: only a version mismatch between the page and the service, seen on any
  connection; there is no manual "update
  available" prompt and no reload for configuration changes without a version change.
- **Legend, settings gear, scale bar, map credits**: stay where they are (right side and bottom of
  the map); only the bottom-left elements move.
- **Out of scope**: local time or other time zones, a time source other than the device clock,
  hiding the column, a setting for text size, moving the legend into the column.
