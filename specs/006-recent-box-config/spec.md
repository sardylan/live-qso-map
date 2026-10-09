# Feature Specification: Configurable Last-QSO Box

**Feature Branch**: `006-recent-box-config`
**Created**: 2026-10-09
**Status**: Draft
**Input**: User description: "Let the 'Last QSO' box be configurable. From the backend it must be
possible to choose which items to display, in the order given in the config file, from the
keywords call, band, mode, date, time, country. Keep call in bold and country in grey. The default
in config must be: call, band, time, country. All fields must be aligned as in a table, without
lines (keep more or less the current aspect). In the top right corner of the page, add a setting
for the number of last QSOs to display in the box, and to choose whether new QSOs go at the top or
the bottom of the list, separate from the current last-QSO number of the map. Defaults: map last
20, box last 5, newest at the bottom." Follow-up: "Add also the 'CQ Zone' to the list of
parameters, and put it in the default values between time and country."

## Clarifications

### Session 2026-10-09

- Q: The box size is separate from the map's "Last N". If the map shows 10 and the box 20, what
  does the box show? → A: The box is independent of the map. It always lists its own last N, even
  QSOs the map does not show; those rows have no marker and are not clickable.
- Q: Which box sizes can a viewer pick? → A: 3, 5, 10, 15 or 20; the default is 5.
- Q: Are the viewer's box size and order remembered? → A: Yes, in the browser, like the
  auto-zoom, box visibility and projection choices.
- Q: What happens with an invalid column list in the configuration? → A: The service refuses to
  start with a clear message, as with the band filter. An empty list or a repeated keyword is
  invalid too.
- Owner addition: a CQ zone column (`cqzone`) joins the keywords, and the defaults become
  `call, band, time, cqzone, country`.
- Q: Should the box have a header row now that the columns vary? → A: Yes, a small grey header
  row with short labels (Call, Band, Mode, Date, UTC, CQ, Country). An operator setting shows or
  hides it; it is shown by default.
- Q: Do box rows for QSOs that are not on the map look different? → A: Yes. They are slightly
  faded and show the tooltip "Not on the map (outside its last N)"; rows on the map look as
  today.
- Q: Should the map's "Last N" also survive a browser restart, like the box settings? → A: Yes.
  Every viewer setting is kept in the browser's local storage. The owner made this a
  constitution principle (1.4.0, Principle IV).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The operator chooses the box columns (Priority: P1)

The operator decides what spectators see in each box row, and in which order, with one setting:
for example `call, band, time, cqzone, country` for a CQ WW display, or
`call, band, mode, date, time, country` for a mixed-mode event. Every page shows those columns
in that order, aligned like a table without lines, the callsign in bold and the country in grey.

**Why this priority**: different events need different details. Today the columns are fixed in
the code.

**Independent Test**: set the columns to `country, call, mode`, start the service and send QSOs.
Every row shows the entity, the callsign in bold, then the mode, aligned in three columns. With
the setting removed, rows show call, band, time, CQ zone and country.

**Acceptance Scenarios**:

1. **Given** no column setting, **When** a page shows the box, **Then** each row shows call,
   band, time, CQ zone and country, in that order.
2. **Given** the setting `call, band, mode, date, time, cqzone, country`, **When** a page shows
   the box, **Then** each row shows those seven values in that order.
3. **Given** any setting, **When** rows are shown, **Then** each column starts at the same
   horizontal position on every row, there are no lines or borders between cells, the callsign is
   bold and the country is grey.
4. **Given** the header setting is on (the default), **When** the box is shown, **Then** a small
   grey header row labels each configured column, in the same order, aligned with the values.
   With the setting off, there is no header row.
5. **Given** a setting with an unknown keyword (for example `freq`), an empty list, or a repeated
   keyword, **When** the service starts, **Then** it refuses to start, and the message names the
   setting and the problem.
6. **Given** the settings are changed and the service restarted, **When** a page reconnects,
   **Then** it shows the new columns and header choice.

---

### User Story 2 - The viewer chooses the box size and order (Priority: P1)

At the top right of the page, next to the map's "Last N" selector, each viewer finds two more
settings for the box: how many QSOs it lists (3, 5, 10, 15 or 20; default 5), and whether the
newest QSO is at the top or the bottom (default: bottom). These are separate from the map's own
"Last N" (default 20), and the browser remembers them.

**Why this priority**: a wall screen and a desktop browser need different box sizes, and
"newest at the bottom" reads like a log, which many operators prefer.

**Independent Test**: set the map to the last 10 and the box to 15 with newest at the top, then
send 20 QSOs. The map shows 10 markers, and the box lists 15 rows, newest first. Reload: the box
is still 15 with newest at the top.

**Acceptance Scenarios**:

1. **Given** a first visit, **When** the page opens, **Then** the map shows the last 20, the box
   lists the last 5, and the newest box row is at the bottom.
2. **Given** the box size is changed, **When** the change is made, **Then** the box immediately
   lists that many of the most recent QSOs (or all of them, when there are fewer).
3. **Given** the order is switched to "newest at the top", **When** a new QSO arrives, **Then** it
   appears as the first row; with "newest at the bottom" it appears as the last row.
4. **Given** the map shows the last 10 and the box is set to 20, **When** 20 QSOs exist, **Then**
   the box lists all 20. The 10 that are not on the map are slightly faded, are not clickable, and
   show the tooltip "Not on the map (outside its last N)" on hover; the other 10 look as before
   and open their marker's popup.
5. **Given** a viewer changed the map's "Last N", the box size or the order, **When** the browser
   is closed and the page opened again, **Then** all three choices are kept.
6. **Given** the box is hidden with its button (feature 004), **When** the box settings are
   changed, **Then** they apply when the box is shown again.

---

### Edge Cases

- **Fewer QSOs than the box size**: the box lists those that exist.
- **Box rows beyond the map window**: shown faded, without a marker, not clickable, and with the
  "Not on the map" tooltip. Auto-zoom frames
  only what is on the map (feature 004).
- **Map set to "All"**: the box still lists only its own last N.
- **A QSO missing a value for a column** (no mode, no date, unknown country or CQ zone): the cell
  shows a placeholder ("—" for country and CQ zone, as in feature 005; empty otherwise), and the
  columns stay aligned.
- **CQ zone overrides**: cty.dat gives some callsign prefixes their own zone inside an entity
  (for example VY1 is zone 1, while Canada's default is 5; R8H is zone 18, while Asiatic Russia's
  default is 17). The CQ zone follows those overrides, not only the
  entity's default zone.
- **Long country names**: cut as in feature 005, with the full name on hover.
- **Keywords in the setting**: they ignore case and surrounding spaces (`" Call "` is `call`).
- **Corrections and removals**: the box follows them as before, for every chosen column.
- **Reconnection**: the box refills with its own last N from the snapshot.
- **Storage unavailable**: the defaults apply on each page load.
- **Text from the network**: shown as plain text, never as markup (feature 004, FR-022).

## Requirements *(mandatory)*

### Functional Requirements

**Operator: box columns**

- **FR-001**: The configuration MUST offer a box-columns setting: an ordered list of one or more
  keywords from `call`, `band`, `mode`, `date`, `time`, `cqzone` and `country`. It MUST be
  settable like the other settings (configuration file, command line, environment), with the same
  precedence.
- **FR-002**: When the setting is absent, the columns MUST be `call, band, time, cqzone,
  country`.
- **FR-003**: An unknown keyword, an empty list or a repeated keyword MUST stop the service at
  startup, with a message that names the setting and the problem. Keywords MUST be compared
  ignoring case and surrounding spaces.
- **FR-004**: Every page MUST show the configured columns, in the configured order, on every box
  row.
- **FR-005**: The columns MUST show:
  - `call`: the remote callsign, in bold;
  - `band`: the band;
  - `mode`: the mode, with the submode when present (`SSB/USB`);
  - `date`: the UTC date as `YYYY-MM-DD`;
  - `time`: the UTC time as `HH:MMZ` (HR-1);
  - `cqzone`: the remote station's CQ zone as a number (for example `15`), from cty.dat with its
    per-callsign overrides, the same way as the country (feature 005), or "—" when unknown;
  - `country`: the DXCC/WAE entity as `prefix · name`, in grey, with the feature-005 rules (name
    cut at 18 characters, full label on hover, "—" when unknown).
- **FR-006**: The rows MUST look like a table without lines: each column starts at the same
  position on every row, with no borders or separators, and the box keeps its current look.
- **FR-006a**: The configuration MUST offer a box-header setting (on or off, default on), settable
  like the other settings. When it is on, the box MUST show one header row in a smaller grey
  style, labelling each configured column in order: `call` Call, `band` Band, `mode` Mode,
  `date` Date, `time` UTC, `cqzone` CQ, `country` Country. The header MUST line up with the value
  columns. When there are no QSOs, the empty-state message replaces both the header and the
  rows.

**Viewer: box size and order**

- **FR-007**: The top right of the page MUST offer, next to the map's "Last N" selector and
  visibly separate from it, a box-size selector (3, 5, 10, 15, 20) and an order choice ("newest at
  the top" or "newest at the bottom").
- **FR-008**: The defaults MUST be: map last 20 (unchanged), box last 5, and newest at the bottom.
- **FR-009**: The box MUST list the most recent QSOs by QSO time (feature 004), up to its own size,
  independently of the map's "Last N". The page MUST receive enough QSOs for both.
- **FR-010**: Box rows of QSOs that are not on the map MUST NOT look or act clickable. They MUST
  be shown slightly faded, with every value still readable, and MUST show the tooltip "Not on the
  map (outside its last N)" on hover. Rows of QSOs on the map keep the feature-004 look and click
  behaviour.
- **FR-011**: The map's "Last N", the box size and the order MUST be remembered for each viewer
  in their browser's local storage, so that they survive a browser restart (constitution
  Principle IV), and MUST be restored on the next visit. Missing, invalid or unavailable storage
  MUST fall back to the defaults. A "Last N" saved earlier for the tab only is not carried over.
- **FR-012**: Changing the box size or order MUST update the box immediately, without a reload.

**General**

- **FR-013**: The service MUST give every QSO sent to pages its CQ zone, resolved from the remote
  callsign with cty.dat, whatever source gave the position, and re-resolved after a correction,
  like the entity in feature 005.
- **FR-014**: Features 001–005 MUST keep working unchanged, apart from the box columns, size and
  order defined here and the map's "Last N" now surviving a browser restart. That covers the
  markers, the popups, auto-zoom, the box toggle, the band filter and the entity in the popups.

### Key Entities

- **Box header** (operator): on or off (default on); a header row with one short label per
  column.
- **Box columns** (operator): an ordered, non-empty list of distinct keywords from `call`,
  `band`, `mode`, `date`, `time`, `cqzone` and `country`. It is set at startup and sent to every
  page.
- **CQ zone**: the remote station's CQ zone (1–40) from cty.dat, with its per-callsign
  overrides. It is optional and resolved for each QSO.
- **Viewer preferences**: the map's "Last N", the box size (3, 5, 10, 15 or 20) and the order
  (newest at the top or the bottom), all remembered in the viewer's browser across restarts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each of 10 column settings, including all seven keywords, a single one, and
  orders different from the default, 100 % of the rows show exactly those columns in that order.
- **SC-007**: For 20 callsigns, including prefixes with zone overrides (VY1, R8H, R8S, R0T), the CQ
  zone shown matches cty.dat's zone for that callsign in 100 % of cases.
- **SC-002**: Each of 6 invalid settings (unknown keyword, empty list, repeated keyword, and
  combinations) stops startup with a message that names the setting.
- **SC-003**: For every combination of map size (10, 20, 50, All) and box size (3, 5, 10, 15,
  20), the box lists exactly min(box size, available QSOs) rows. In a 25-QSO test, the map shows
  exactly its own window.
- **SC-004**: With "newest at the bottom", a new QSO appears as the last row within 2 s; with
  "newest at the top", as the first row.
- **SC-005**: In every row, each column starts within 1 px of the same position as in the other
  rows, at window widths of 1024 px and 1920 px.
- **SC-006**: After the browser is closed and reopened, 100 % of the viewer's choices (map "Last
  N", box size, order) are restored.

## Assumptions

- **Wording on the page**: the map selector reads "Map: last N QSOs". The box settings read
  "Box: last N" and "Newest: top / bottom".
- **Names of the keywords**: `country` stands for the DXCC/WAE entity of feature 005 (HR-6), and
  `cqzone` for the CQ zone.
- **CQ zone format**: the zone number alone (for example `15`), with no "CQ" prefix.
- **How the page receives enough QSOs**: this is left to the plan; a per-page request is one
  option.
- **Feature 005 change**: newest at the bottom becomes the default, where feature 005 listed the
  newest first. "Newest at the top" restores the old order.
- **Out of scope**:
  - per-viewer choice of columns;
  - column widths or custom styles;
  - a box larger than 20 rows;
  - sorting by anything other than QSO time.
