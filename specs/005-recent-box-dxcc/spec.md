# Feature Specification: DXCC Entity and Date in the Last-QSO Box

**Feature Branch**: `005-recent-box-dxcc`
**Created**: 2026-10-09
**Status**: Complete (2026-10-09). See `checklists/verification.md`.
**Input**: User description: "On frontend, in last QSO box, hide mode and add Date. Add also the
DXCC country prefix and name."

## Clarifications

### Session 2026-10-09

- Q: Where do each QSO's DXCC prefix and name come from? → A: Always from cty.dat, resolved from
  the remote callsign, whatever source gave the position.
- Q: How are the date and time shown? → A: ISO date and UTC time in two columns, for example
  `2026-10-09` and `15:04Z`.
- Q: How are long entity names shown? → A: Cut with "…" at about 18 characters; the full name
  appears on hover.
- Q: Do the marker popups show the entity too? → A: Yes, the box and the popups.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See where each recent QSO is (Priority: P1)

A spectator glances at the last-QSO box. Each row shows the callsign, the band, the UTC date and
time, and the station's DXCC entity as prefix and name, for example `IS · Sardinia`. The mode is
no longer shown: for a contest it rarely changes, and the space is better used for the country.

**Why this priority**: the entity is what spectators look for first ("we just worked Japan"), and
the date removes doubt during contests that cross midnight UTC.

**Independent Test**: send QSOs with K1ABC, JA1XYZ and IS0ABC. Each row shows its date, time and
entity (`K · United States`, `JA · Japan`, `IS · Sardinia`), and no row shows a mode.

**Acceptance Scenarios**:

1. **Given** the box is shown, **When** a QSO arrives, **Then** its row shows the callsign, band,
   UTC date (`YYYY-MM-DD`), UTC time (`HH:MMZ`), entity prefix and entity name, and no mode.
2. **Given** a QSO whose remote callsign cty.dat resolves, **When** it is listed, **Then** the
   prefix and name come from cty.dat, even when the position came from the logger or a callbook.
3. **Given** a QSO whose callsign matches no entity, or when no country data is loaded, **When**
   it is listed, **Then** the entity cell shows "—" and the rest of the row is unchanged.
4. **Given** an entity name longer than about 18 characters (for example "Amsterdam & St. Paul
   Is."), **When** it is listed, **Then** it is cut with "…", and hovering shows the full name.
5. **Given** a QSO that crosses midnight UTC compared with the previous one, **When** both are
   listed, **Then** their different dates are visible.

---

### User Story 2 - Entity in the marker popup (Priority: P2)

A viewer clicks a marker. Each QSO in the popup also shows its entity as prefix and name, so the
popup matches the box.

**Why this priority**: it keeps the two views consistent. The map already shows where the station
is, so this is a smaller gain than the box.

**Independent Test**: click the marker of a QSO with JA1XYZ. The popup row shows `JA · Japan`
next to the existing call, band, mode, time and position source.

**Acceptance Scenarios**:

1. **Given** a marker popup, **When** it opens, **Then** each QSO row also shows `prefix · name`,
   or "—" when unknown; the mode stays in the popup.
2. **Given** an open popup, **When** one of its QSOs is corrected to another callsign, **Then**
   the entity in the popup updates with the rest of the row.

---

### Edge Cases

- **Portable and prefixed calls** (for example `IS0MZK/P`, `EA8/DL1ABC`, `KH6/W1XX`): the entity
  follows the same rules cty.dat lookups already use for positions (feature 001).
- **Exact-call entries in cty.dat** (for example `=4U1UN`): the exact match wins.
- **WAE entities** (for example Sicily `IT9`): the prefix is shown without the `*` marker
  (HR-6).
- **Callsign changed by a correction**: the entity is resolved again for the corrected QSO.
- **Country data missing at startup or after a failed refresh**: every entity cell shows "—";
  nothing else changes.
- **QSO with no date or time** in the record: the date and time cells are empty, as the time
  cell already is in feature 004.
- **Text from the network** (callsign, entity name): rendered as plain text, never as markup
  (feature 004, FR-022).

## Requirements *(mandatory)*

### Functional Requirements

**Server**

- **FR-001**: The service MUST give every QSO it sends to pages its DXCC/WAE entity (HR-6): the
  cty.dat primary prefix, without the `*` that marks WAE entities, and the entity name, both
  resolved from the remote callsign with the cty.dat rules used elsewhere (feature 001).
- **FR-002**: The entity MUST come from cty.dat alone, whatever source gave the position
  (logger, callbook or cty.dat).
- **FR-003**: When the callsign matches no entity, or no country data is loaded, the entity MUST
  be absent. Its absence MUST NOT affect anything else about the QSO.
- **FR-004**: A corrected QSO MUST carry the entity of its corrected callsign.
- **FR-005**: The entity MUST be sent in a way that follows the interface rules
  (`docs/http-api.md`: JSON, `snake_case`, absent values as `null`).

**Last-QSO box**

- **FR-006**: Each row MUST show, in this order: callsign, band, UTC date as `YYYY-MM-DD`, UTC
  time as `HH:MMZ` (HR-1), and the entity as `prefix · name`. The mode MUST NOT be shown.
- **FR-007**: Entity names longer than 18 characters MUST be cut to 18 characters, the last of
  which is "…". The full `prefix · name` MUST be available on hover.
- **FR-008**: A row whose QSO has no entity MUST show "—" in the entity cell.
- **FR-009**: Every rule of feature 004 for the box is unchanged: the five newest by QSO time,
  clickable rows, the hide button and the plain-text rendering.

**Marker popup**

- **FR-010**: Each QSO row in a marker popup MUST also show `prefix · name`, or "—" when unknown.
  The popup keeps its existing content, mode included.

### Key Entities

- **QSO entity**: the DXCC/WAE entity of the remote station: primary prefix (for example `JA`,
  `IS`, `IT9`) and name (for example "Japan"), from cty.dat. It is optional and resolved for each
  QSO, including after a correction.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a test with 30 callsigns that cover ordinary, portable, prefixed, exact-call
  and WAE cases, 100 % of the box rows and popups show the entity that the cty.dat rules give
  for that callsign.
- **SC-002**: With no country data, every row shows "—" as its entity, and the box and popups
  otherwise work as before.
- **SC-003**: At a 1024 px wide window, the box with five rows, including the longest cty.dat
  entity names, stays narrower than 45 % of the window, and no row wraps.
- **SC-004**: A new or corrected QSO shows its entity in the box within the 2 s that feature 001
  allows for a QSO to reach the page.

## Assumptions

- **Order of the box columns**: callsign, band, date, time, entity.
- **Separator**: the entity is written as `prefix · name`.
- **Unknown values**: "—" marks an unknown entity.
- **Truncation**: the 18-character cut applies to the name; the prefix is never cut.
- **Protocol**: adding the entity to the QSO data is an additive change under the interface rules
  (API-4). The plan decides whether the protocol version is bumped.
- **No new data source**: cty.dat is already loaded at startup (feature 001). Nothing is
  fetched.
- **Out of scope**:
  - showing CQ or ITU zones and continents;
  - colour by entity;
  - entity counts or statistics;
  - changing the legend.
