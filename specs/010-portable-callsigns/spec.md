# Feature Specification: Positions for Prefixed and Portable Callsigns

**Feature Branch**: `feature/010-portable-callsigns`
**Created**: 2026-10-13
**Status**: Draft
**Input**: User description: "New feature: consider callsign with country prefix or suffixes. If a
callsign have a prefix or a suffix, must be treated properly. In case of prefix, the prefix must be
used as country: IS0/DL0AAA -> Search it into online services, if the locator is inside Sardinia,
use it. On the contrary, use center Sardinia as position as per offline data. IT9/K0AAA -> The
same, but with Sicily. I/HB9AAA -> The same, use Italy as country. ES0/xxxxxx -> if I remember
well, ES0 is a particular island o Estony, so place the marker in the proper island (don't remeber
if the island is ES9 or ES0, but you got the point). In case of suffix: IS0AAA/P -> Search it
online, and if it does not exist, search the callsign without the suffix, like IS0AAA. Then follow
the normal flow. K0AAA/6 -> Same as before. Bugfix: if a callsign is not antartica and resolve to
latitude 0 and longitude 0, consider it as an error, and use the country of the callsign instead."

## Problem Summary

The **entity** of a compound callsign is already right. The country data reads `IS0/DL0AAA` as
Sardinia, `DL0AAA/P` as Fed. Rep. of Germany and `K0AAA/6` as the United States (`docs/ham-radio.md`
§2.2).

The **position** isn't. Online lookups receive the callsign exactly as logged, and the most precise
answer wins. So a station operating from Sardinia as `IS0/DL0AAA` is drawn at DL0AAA's home
locator in Germany. That's more precise than Sardinia's reference point, but wrong.

Two related problems:
- **No online record:** a portable callsign such as `IS0AAA/P` often has no online record of its
  own, while `IS0AAA` does. Today it falls back to the entity's reference point even when the
  base callsign has a usable locator.
- **Placeholder positions:** a source that answers 0°/0° (an empty record, read as numbers)
  puts the QSO in the Gulf of Guinea, and the placeholder locator `AA00AA` puts it at the South
  Pole.
- **Positions outside the entity:** an online locator can point outside the callsign's entity
  altogether, for example an outdated or mistyped home locator, and the QSO is then drawn in the
  wrong country.

## Clarifications

### Session 2026-10-13

- Q: Where should a station in a sub-area of an entity (`ES0`/`ES9` in Estonia, US call area 6
  in California, …) be drawn when no usable online locator exists? Is there offline data more
  detailed than cty.dat? → A: cty.dat only, for the moment: the entity's reference point. No
  free downloadable dataset with per-prefix positions was confirmed. The cty.dat in use has no
  per-prefix position overrides, and Club Log's `cty.xml` needs an API key and may only repeat
  entity positions. Entities that cty.dat separates (Italy / Sardinia / Sicily, USA / Alaska /
  Hawaii) are placed correctly. Finer sub-area data is kept as a follow-up (see "Follow-up").
- Q: Should the "position must lie inside the entity" check apply to every callsign, or only to
  callsigns with a location prefix? → A: Every callsign. 0°/0° and the `AA00AA` locator are two
  special cases of it. `AA00AA` is a placeholder locator whose centre lies at the South Pole: it is
  wrong for every station except an Antarctic one (that's why the request exempted Antarctica).
- Q: Entity rectangles overlap (Italy's contains Sardinia's and most of Sicily's). Does a position
  in Cagliari count as "inside Italy"? → A: No. A position belongs to the smallest entity whose
  rectangle contains it. Only rectangles **nested entirely** inside the entity's own rectangle
  count, such as Sardinia's and Sicily's inside Italy's, or Corsica's inside France's. Neighbours
  whose rectangles merely overlap along a border don't count, so a locator in eastern Germany
  stays "inside Germany".
- Q: When an online position is rejected, what is kept from that result? → A: Its non-location
  fields (operator name, e-mail, QSL information and the like) are kept. Only the location fields
  are dropped.
- Q: How does the operator learn that a position was rejected? → A: The QSO record carries a
  note naming each rejected source and the reason. It is visible wherever the record's fields are
  shown, and kept in the ADIF. The page itself doesn't change.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A station operating from another entity is drawn in that entity (Priority: P1)

As an operator watching the map, a QSO with `IS0/DL0AAA` (a German station operating from
Sardinia) appears in Sardinia, not at the station's home in Germany. When the online services
know a locator for it in Sardinia, the marker uses that locator; otherwise it sits at Sardinia's
reference point, with precision "country".

**Why this priority**: it's the main error the user sees. A whole category of contest QSOs (DX
operations, holiday stations, IOTA activations) lands in the wrong country.

**Independent Test**: log `IS0/DL0AAA` with an online record whose locator is in Germany, and
check that the marker sits at Sardinia's reference point with precision "country". Then give the
record a locator inside Sardinia, and check that the marker uses that locator.

**Acceptance Scenarios**:

1. **Given** `IS0/DL0AAA` and an online locator inside Sardinia, **When** the QSO is enriched,
   **Then** the position is that locator, with its precision and its online source.
2. **Given** `IS0/DL0AAA` and an online locator outside Sardinia (DL0AAA's home in Germany),
   **When** the QSO is enriched, **Then** the position is Sardinia's reference point, precision
   "country", source "cty".
3. **Given** `IT9/K0AAA`, **When** it is enriched, **Then** the same rule applies with Sicily.
4. **Given** `I/HB9AAA`, **When** it is enriched, **Then** the same rule applies with Italy. A
   locator in Sardinia or Sicily counts as outside Italy (nested entities), so it is rejected.
5. **Given** a prefixed callsign that no online service knows, **When** it is enriched, **Then**
   the position is the prefix entity's reference point.
6. **Given** the prefix written after the callsign (`DL0AAA/IS0`), **When** it is enriched,
   **Then** it is handled exactly like `IS0/DL0AAA`.
7. **Given** a rejected online position, **When** the QSO is stored, **Then** none of that
   result's location fields (locator, coordinates, town) appear in the record, but its operator
   name and other non-location fields do. The entity shown is still the prefix entity.

---

### User Story 2 - A portable callsign with no online record uses its base callsign (Priority: P1)

As an operator, a QSO with `IS0AAA/P` or `K0AAA/6` gets the position the online services give
for the callsign as logged. If they don't know it, the lookup is repeated with the base callsign
(`IS0AAA`, `K0AAA`), and the normal rules apply to that answer.

**Why this priority**: portable and mobile operation is very common in contests. Today most of
these QSOs fall back to a country-level position, although a locator is available.

**Independent Test**: log `IS0AAA/P` where only `IS0AAA` has an online record, and check that the
marker uses `IS0AAA`'s locator. Then give `IS0AAA/P` its own record, and check that it wins.

**Acceptance Scenarios**:

1. **Given** `IS0AAA/P` with its own online record, **When** it is enriched, **Then** that
   record is used, and no second lookup is made.
2. **Given** `IS0AAA/P` with no online record, and `IS0AAA` with one, **When** it is enriched,
   **Then** the position and other fields come from `IS0AAA`'s record.
3. **Given** `K0AAA/6` with no online record, **When** it is enriched, **Then** `K0AAA` is looked
   up, and its position is used under the normal rules.
4. **Given** neither the portable nor the base callsign is known online, **When** it is enriched,
   **Then** the position is the entity's reference point, as today.
5. **Given** a maritime or aeronautical mobile callsign (`DL0AAA/MM`, `/AM`), **When** it is
   enriched, **Then** no base-callsign lookup is made: the station is not at home, and has no
   entity.
6. **Given** a callsign whose online result is used, **When** the record is written, **Then** the
   callsign itself stays exactly as logged (HR-5).

---

### User Story 3 - Positions outside the entity, and placeholder positions, are treated as errors (Priority: P2)

As an operator, a QSO is never drawn outside its callsign's entity because of a wrong online
locator. Placeholder positions (0°/0°, or the `AA00AA` locator) are never drawn either. The QSO
is drawn at the entity's reference point instead, as when no position is found.

**Why this priority**: it's a visible bug, but only for wrong or empty online records.

**Independent Test**:
1. Give `DL0AAA` an online locator in France, and check that the marker sits at Germany's
   reference point, precision "country".
2. Give an online record coordinates 0/0, then locator `AA00AA`, and check the same.
3. Give an Antarctic callsign locator `AA00AA`, and check that it is accepted.

**Acceptance Scenarios**:

1. **Given** a callsign without prefix or suffix whose online locator lies outside its entity,
   **When** the QSO is enriched, **Then** that position is discarded, and the entity's reference
   point is used (unless another source gives an acceptable position).
2. **Given** an online result at exactly 0°/0°, or with locator `AA00AA`, **When** the QSO is
   enriched, **Then** that result's position is discarded, and the other sources' positions or the
   entity's reference point are used.
3. **Given** the logger sends latitude 0 and longitude 0, or the locator `AA00AA`, **When** the
   QSO is enriched, **Then** the logger position is ignored, and the lookups run as if the logger
   had sent no position.
4. **Given** an Antarctic callsign (for example `KC4AAA` at the South Pole) with locator `AA00AA`,
   **When** it is enriched, **Then** the position is accepted: it lies inside Antarctica.
5. **Given** a locator near 0°/0° but not at it (for example `JJ00`, a 4-character square whose
   centre is not 0°/0°), **When** it lies inside the callsign's entity, **Then** it is accepted as
   usual.
6. **Given** any discarded position, **When** it happens, **Then** the service logs a warning
   naming the callsign, the source and the reason, without stopping. The QSO record carries the
   same information in its rejection note (FR-013), visible with the QSO's other fields.

---

### Edge Cases

- **Entity with no known area**: a few entities have no area in the bundled data (13 of 346
  today). An online position for them can't be checked, so it is accepted, except the placeholders
  (0°/0° always; `AA00AA` unless the entity is Antarctica).
- **No country data at all**: without cty.dat there's no entity and no reference point. Online
  positions are accepted as today, except 0°/0° and `AA00AA`. A rejected position leaves the QSO
  undrawn, and a warning is logged.
- **Position on the edge of an entity**: an entity's area is a rectangle that encloses its land
  (feature 004). A locator inside the rectangle but in the sea, or in a neighbouring country whose
  rectangle only overlaps it, is accepted. A precise border check is out of scope.
- **Nested entities**: `I/HB9AAA` with an online locator in Cagliari is rejected, because the
  position lies in Sardinia, nested inside Italy's rectangle. The QSO is drawn at Italy's
  reference point. `IS0/HB9AAA` with the same locator is accepted.
- **Exact-call entries**: callsigns the country data lists as exact matches keep their entity and
  reference point. The rules above use that entity.
- **Several slashes** (`IS0/DL0AAA/P`): the location prefix decides the entity (User Story 1), and
  the base callsign without both prefix and suffix (`DL0AAA`) is the one retried online (User
  Story 2).
- **Logger position outside the entity**: the logger's position (LAT/LON or GRIDSQUARE) is trusted
  as it is (FR-012 of feature 001). The operator entered or confirmed it. Only the placeholders are
  rejected.
- **Antarctica and `AA00AA`**: the South Pole lies inside Antarctica's area, so the area check
  itself accepts `AA00AA` for Antarctic callsigns and rejects it for everyone else. The explicit
  placeholder rule (FR-008) keeps the same exception.

## Requirements *(mandatory)*

### Functional Requirements

**Prefixed callsigns (User Story 1)**

- **FR-001**: A callsign with a location prefix (`PREFIX/CALL` or `CALL/PREFIX`, the prefix being
  the shorter, prefix-like part, `docs/ham-radio.md` §2.2) MUST take its entity from that prefix,
  as today.
- **FR-002**: Its online lookups MUST first use the callsign as logged. If no online service knows
  it, they MUST be repeated with the base callsign (`DL0AAA` for `IS0/DL0AAA`).
- **FR-003**: An online position MUST be used only if it lies inside the area of the callsign's
  entity: the prefix entity for a prefixed callsign, otherwise the callsign's own entity. A
  position that also lies inside the area of another entity nested entirely within that area
  (Sardinia or Sicily within Italy) belongs to the nested entity, so it is **not** inside the outer
  one. This
  applies to **every** callsign. A position outside MUST be discarded, and the next acceptable
  source or the entity's reference point used, with precision "country" and source "cty".
- **FR-004**: When an online position is discarded, the location fields of the same result
  (locator, latitude, longitude, town and other address fields) MUST NOT be written to the QSO.
  Its non-location fields (operator name, e-mail, QSL information and the like) MUST be kept, as
  for any other result, without overwriting values already present.

**Portable suffixes (User Story 2)**

- **FR-005**: A callsign with a portable suffix (`/P`, `/M`, `/A`, `/QRP` and the others that the
  entity rules ignore, or a call-area digit such as `/6`) MUST be looked up online as logged first.
  Only if no online service knows it, the lookup MUST be repeated once with the base callsign.
- **FR-006**: A position found through the base callsign MUST follow the normal rules: the most
  precise wins, and it MUST carry its real precision and source (HR-7).
- **FR-007**: Maritime and aeronautical mobile callsigns (`/MM`, `/AM`) MUST NOT be retried with
  the base callsign.

**Placeholder positions (User Story 3)**

- **FR-008**: Two positions are placeholders, from any source (logger, online service or country
  data), and MUST be treated as no position from that source:
  - exactly 0° latitude and 0° longitude, always;
  - any position inside locator square `AA00` (this includes `AA00AA`, and the coordinates
    −90°/−180° some sources send), except for a callsign whose entity is Antarctica.

  The next source, or the entity's reference point, MUST be used instead. This applies even when
  the entity's area is unknown, or when there is no country data.
- **FR-009**: Every discarded position (FR-003, FR-008) MUST be logged as a warning naming the
  callsign, the source and the reason, without stopping the QSO.
- **FR-013**: Every discarded position MUST also be noted in the QSO record, in an application
  field (`APP_LIVEQSOMAP_` prefix, HR-11). Each entry names the source (logger, or the online
  service) and the reason (outside the entity, or placeholder). With several rejections, all are
  listed, in source order. The field is absent when nothing was discarded.

**Unchanged**

- **FR-010**: The callsign MUST stay as logged in the record and on the map (HR-5).
- **FR-011**: The entity shown for a QSO MUST still come from the country data (feature 005).
- **FR-012**: A position from the logger MUST still prevent any lookup (feature 001), unless it is
  a placeholder (FR-008). Logger positions are not subject to the area check (FR-003).

### Key Entities

- **Compound callsign**: a logged callsign split into a base callsign, an optional location
  prefix and optional suffixes, by the rules in `docs/ham-radio.md` §2.2.
- **Entity area**: the rectangle enclosing an entity's land, already bundled for the map's
  auto-zoom (feature 004). It is used to accept or discard a position. An entity's area excludes
  the areas of entities nested entirely inside it (Sardinia and Sicily inside Italy).
- **Entity reference point**: the entity's position in the country data, used whenever no
  acceptable position is found (precision "country").
- **Rejection note**: the list of discarded positions for a QSO, each with its source and reason,
  stored with the QSO's other fields.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a replayed set of prefixed callsigns (`IS0/…`, `IT9/…`, `I/…` and others) whose
  online locators are at the operators' homes abroad, 100% are drawn inside the prefix entity.
- **SC-002**: For prefixed callsigns whose online locator lies inside the prefix entity, 100% keep
  that locator's precision.
- **SC-003**: For portable callsigns whose own callsign is unknown online but whose base callsign
  is known, 100% get the base callsign's position, instead of a country-level one.
- **SC-004**: No QSO is ever drawn at 0°/0°, and no non-Antarctic QSO is drawn at `AA00AA`.
- **SC-005**: Every QSO whose position changed because of this feature shows a precision and a
  source that tell the operator how it was found (HR-7). Every QSO with a rejected position
  carries a rejection note that names the source and the reason.
- **SC-006**: Callsigns without a prefix or suffix whose online position lies inside their entity
  get exactly the same position as before this feature (no regression).
- **SC-007**: For a replayed set of plain callsigns whose online locator lies outside their
  entity, 100% are drawn inside the entity.

## Assumptions

- **Entity areas are rectangles**: "inside an entity" means inside the rectangle bundled by
  feature 004, which is coarse along borders. That's accepted for this feature.
- **Which callsign is the base**: the part that is a complete callsign after removing the
  location prefix and any portable suffix, following the existing entity rules (`IS0/DL0AAA` →
  `DL0AAA`, `IS0AAA/P` → `IS0AAA`, `K0AAA/6` → `K0AAA`).
- **"Does not exist online"** means that no configured online service returned a record for the
  callsign. A record without a position is a known callsign: no base-callsign retry is made.
- **Antarctica exception**: `AA00AA` covers the South Pole, where an Antarctic station really can
  be. So it's valid only for Antarctic callsigns. 0°/0° is never in Antarctica (south of 60°S), so
  it is rejected for every callsign.
- **Logger positions are trusted**: the area check applies to positions found online, not to those
  sent by the logger, which the operator entered or confirmed (only placeholders are rejected).
- **One extra lookup at most**: the base-callsign retry is a single extra round of lookups,
  bounded by the existing lookup timeout. It only happens when the logged callsign is unknown.
- **No new online services or settings**: the existing providers and their order are used.

## Follow-up (out of scope)

- **Sub-area positions**: to be investigated later. Look for offline data that places a station
  inside its entity by prefix or call area (for example Estonian `ES0`/`ES9`, US call areas, where
  `6` is California). The first candidate is Club Log's `cty.xml`: check whether its prefix entries
  carry their own coordinates, what the API key and its terms require, and how often it changes.
  Until then, the entity's reference point from cty.dat is used.
