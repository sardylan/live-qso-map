# Feature Specification: Contest Score and Page Layout

**Feature Branch**: `007-contest-score-layout`
**Created**: 2026-10-09
**Status**: Draft
**Input**: User description: "Add a new spec for displaying contest scores, taken from the
'every-minute' message broadcast by QARTest. The current score must be a new simple one-line box
on the bottom left corner of the page, on top of the Last QSO box (if visible), always visible.
Content could be: 'x.xxx.xxx - Cyyy Zzzz', where x is the total points scored, yyy the sum of all
countries and zzz the sum of all zones. Suggest any possible variant that remains one line and keeps
it simple. Refactor the frontend with this: zoom feature, flat/globe toggle and auto-zoom toggle on
the top left corner; all user settings (last QSO and counters) on the top right corner; legend on
the bottom right; last QSO box on the bottom left (togglable) with Scores."

## Clarifications

### Session 2026-10-09

- Q: Which one-line format? → A: The score, the QSO count, and the sum of every multiplier type:
  `1.234.567 pts · 1.523 QSO · 183 mults`. This works for any contest (HR-10), whatever its
  multiplier types are called.
- Q: Which thousands separator? → A: A dot: `1.234.567`.
- Q: What does the box show before the first score, and when scores stop arriving? → A:
  `Score —` until the first score message arrives. If none arrives for 5 minutes (QARTest sends one
  a minute), the last score stays but is greyed out, with a tooltip saying when it was last updated.
  New pages get the last known score at once.
- Q: Which settings go in the top-right panel? → A: the map's "Last N", the box size, the box order,
  and the Last QSOs show/hide switch, which moves out of the top-left controls. The top left keeps
  only zoom, Globe/Flat and Auto-zoom.
- Q: Must the layout work on narrow screens? → A: No. The layout must work from 1024 px wide up;
  phones and smaller screens are out of scope.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the current contest score (Priority: P1)

During a contest, spectators and operators glance at the map and see the station's current score
in one line at the bottom left: `1.234.567 pts · 1.523 QSO · 183 mults`. The logger (QARTest)
broadcasts its score every minute, and the line updates within seconds of each broadcast, on every
open page.

**Why this priority**: the score is the one number everyone in a contest cares about. Today the
service receives it and throws it away (feature 001, FR-005a).

**Independent Test**: replay the two QARTest score messages from the feature-001 reference
capture. The box first shows `90 pts · 6 QSO · 10 mults`, then `132 pts · 7 QSO · 11 mults`.

**Acceptance Scenarios**:

1. **Given** a QARTest score message, **When** it arrives, **Then** within 2 s every open page
   shows `<score> pts · <QSO total> QSO · <sum of all multiplier totals> mults`, with dots as
   thousands separators, on one line.
2. **Given** a score message whose multipliers have other types (for example prefixes, or states),
   **When** it arrives, **Then** the multiplier count is still the sum of all types listed in its
   totals.
3. **Given** no score message yet since the service started, **When** a page is shown, **Then** the
   box reads `Score —`.
4. **Given** a score was received, **When** a new page opens or a page reconnects, **Then** it shows
   that last score at once, without waiting for the next broadcast.
5. **Given** the last score message is more than 5 minutes old, **When** the page is shown,
   **Then** the score stays visible but greyed out, and its tooltip says when it was last updated
   (UTC). The next score message restores the normal look.
6. **Given** a malformed score message, **When** it arrives, **Then** it is ignored and logged, and
   the last good score stays.

---

### User Story 2 - A tidy, predictable page layout (Priority: P2)

Each corner of the page has one job:

| Corner | Holds |
|--------|-------|
| top left | the map controls: zoom, Globe/Flat, Auto-zoom |
| top right | the viewer's settings: "Map: last N QSOs", "Box: last N", "Newest: top/bottom", and the Last QSOs show/hide switch |
| bottom right | the band legend |
| bottom left | the score line, with the Last QSOs box under it when shown |

**Why this priority**: the page has grown feature by feature, and controls ended up where there
was room. This puts each control where people expect it.

**Independent Test**: open the page at 1024 px and 1920 px wide. Each control is in its corner as
listed. Hide the Last QSOs box from the top-right panel: the score line moves down to take its
place.

**Acceptance Scenarios**:

1. **Given** the page, **When** it is shown, **Then** the top-left corner holds only the zoom
   controls, the Globe/Flat button and the Auto-zoom button.
2. **Given** the page, **When** it is shown, **Then** the top-right panel holds the map's "Last N",
   the box size, the box order and the Last QSOs show/hide switch, grouped as one panel.
3. **Given** the page, **When** it is shown, **Then** the band legend is at the bottom right.
4. **Given** the Last QSOs box is shown, **When** the page is shown, **Then** the score line sits
   directly above it at the bottom left. **Given** the box is hidden, **Then** the score line
   moves down to its place.
5. **Given** the flat map with its scale bar, **When** the page is shown, **Then** the scale bar,
   the box and the score line are stacked at the bottom left without overlapping, and none covers
   the attribution.
6. **Given** auto-zoom is on, **When** it frames the QSOs, **Then** it keeps clear of the score
   line too (feature 004, FR-009).
7. **Given** the Last QSOs show/hide switch in its new place, **When** it is used, **Then** it
   behaves as before (feature 004), and its state is still remembered across browser restarts
   (constitution Principle IV).

---

### Edge Cases

- **Several loggers sending scores** (for example two QARTest stations in a multi-op network): the
  latest message received is shown.
- **A score message for another station callsign** than the configured one: it is shown anyway,
  because the score belongs to whoever runs the logger on that network.
- **A score of 0, or no multipliers yet**: shown as `0 pts · 0 QSO · 0 mults`.
- **Very large scores** (tens of millions): the line still fits on one line at 1024 px, for example
  `12.345.678 pts · 9.876 QSO · 1.234 mults`.
- **Service restart**: the score is held in memory, so the box shows `Score —` until QARTest's next
  broadcast, which comes within a minute.
- **Stale score at page load**: a page that opens when the last score is already more than 5
  minutes old shows it greyed out at once.
- **Clock differences**: staleness is measured from when the service received the message, not
  from the timestamp in the message.
- **Text from the network**: the score values are shown as plain text, never as markup.

## Requirements *(mandatory)*

### Functional Requirements

**Score**

- **FR-001**: The service MUST read QARTest score messages (`dynamicresults`), which feature 001
  ignored, and take from their totals:
  - the total score;
  - the total number of QSOs;
  - the sum of the totals of every multiplier type listed.
- **FR-002**: The service MUST keep the latest score in memory, with the time it was received. It
  MUST send it to every open page within 2 s of receiving it, and to every page that opens or
  reconnects afterwards.
- **FR-003**: A malformed score message MUST be ignored and logged, without changing the last score
  or affecting QSO handling.
- **FR-004**: Score messages MUST NOT change the map, the QSO history or the last-QSO box: they are
  not QSOs.

**Score line**

- **FR-005**: The page MUST show the score as one line: `<score> pts · <QSOs> QSO · <mults> mults`,
  with a dot as the thousands separator in every number, for example
  `1.234.567 pts · 1.523 QSO · 183 mults`. Before the first score, it MUST show `Score —`.
- **FR-006**: The score line MUST always be visible: it has no hide switch.
- **FR-007**: When the latest score is more than 5 minutes old, by the time the service received it,
  the line MUST be greyed out. Once a score is shown, the line always has the tooltip
  `Last updated HH:MMZ` in UTC (HR-1). A new score restores the normal look.
- **FR-008**: The score values MUST be rendered as plain text.

**Layout**

- **FR-009**: The top-left corner MUST hold only the zoom controls, the Globe/Flat button and the
  Auto-zoom button, in that order.
- **FR-010**: The top-right corner MUST hold one settings panel with "Map: last N QSOs", "Box: last
  N", "Newest: top/bottom", and the Last QSOs show/hide switch. The switch MUST keep its feature-004
  behaviour and its remembered state.
- **FR-011**: The band legend MUST be at the bottom right, above the attribution, as today.
- **FR-012**: The bottom-left corner MUST stack, from bottom to top: the scale bar (flat map only),
  the Last QSOs box (when shown), and the score line. None of them may overlap another or cover the
  attribution.
- **FR-013**: Auto-zoom MUST treat the score line like the other panels and keep the QSOs clear of
  it (feature 004, FR-009).
- **FR-014**: The layout rules FR-009 to FR-013 apply to windows 1024 px wide or wider. Narrower
  windows are out of scope.
- **FR-015**: Features 001–006 MUST keep working unchanged, apart from the positions of the controls
  that this feature moves.

### Key Entities

- **Contest score**: the latest score reported by the logger: total score, total QSOs, sum of all
  multiplier totals, and the time the service received it. It is held in memory only.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Replaying the two score messages from the feature-001 reference capture shows exactly
  `90 pts · 6 QSO · 10 mults`, then `132 pts · 7 QSO · 11 mults`, on every open page, each within
  2 s.
- **SC-002**: A page that opens after a score was received shows it within 1 s of opening.
- **SC-003**: With no score message for 5 minutes, every page greys the line out within 10 s of the
  5-minute mark, and restores it within 2 s of the next message.
- **SC-004**: At 1024 px and 1920 px wide, each control is in the corner listed in User Story 2, and
  no two panels overlap, with the box shown or hidden, on the globe or on the flat map.
- **SC-005**: A score line of `12.345.678 pts · 9.876 QSO · 1.234 mults` fits on one line at 1024 px
  wide.
- **SC-006**: 100 malformed score messages in a row change nothing on the page, and QSOs keep being
  processed.

## Assumptions

- **Source**: only QARTest score messages are read. Other loggers' score formats are out of scope.
- **Units**: "pts" is the contest score, the number QARTest reports as `<score>`. The points total
  of the breakdown is not shown.
- **Mults**: the sum of all `band='total' mode='ALL'` multiplier entries, whatever their type.
  Per-mode totals are not added, so mixed-mode contests are not double-counted. For CQ WW that is
  countries plus zones.
- **Staleness threshold**: fixed at 5 minutes, five missed one-minute broadcasts.
- **Protocol**: the score reaches pages as an additive message, under the interface rules
  (`docs/http-api.md`). The plan decides whether the protocol version changes.
- **Out of scope**:
  - per-band breakdowns;
  - score history or graphs;
  - a rate (QSOs per hour);
  - claimed versus verified scores;
  - layouts for windows narrower than 1024 px, such as phones;
  - keeping the score across restarts (QSO persistence is not part of the project).
