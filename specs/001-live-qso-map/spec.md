# Feature Specification: Live QSO Map

**Feature Branch**: `001-live-qso-map`
**Created**: 2026-10-08
**Status**: Complete (2026-10-09). Deferred owner checks are listed in
`checklists/verification.md`.
**Input**: User description: "QSOs arrive over UDP from different sources (plain ADIF, QARTest XML), are
normalized to ADIF, enriched with station and remote-station geolocation (logger data, QRZ.com, QRZCQ,
HamQTH, local country data), and broadcast live to every open web map page, which shows the last N QSOs
as band-colored markers with geodesic lines from the station. Contest-agnostic; runs on a small local
server, a VM or a cloud service."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Logged QSO appears live on every open map (Priority: P1)

A contest operator logs a QSO in their logger. The logger sends it as ADIF over UDP to the service. The
QSO already contains the remote station's locator and the station's own locator. Within seconds, every
open copy of the map page shows a marker at the remote station's position. The marker is colored by band,
and a geodesic line connects it to the station position.

**Why this priority**: This is the core value: live visual tracking of contest activity. It works
without any external lookup service, so it is the smallest slice that can be used on its own.

**Independent Test**: Open the map in two browsers. Send one UDP datagram that holds a single ADIF
record with GRIDSQUARE and MY_GRIDSQUARE. Both pages show one marker at the centre of the locator square,
in the band's color, with a geodesic line from the station's square.

**Acceptance Scenarios**:

1. **Given** two open map pages, **When** a datagram with one valid ADIF QSO that includes the remote
   and station positions arrives, **Then** both pages show a new marker and a geodesic line within the
   target time (SC-001).
2. **Given** a datagram that holds three ADIF records (with or without an ADIF header), **When** it is
   received, **Then** it is treated as three separate QSOs and three entries appear on the map.
3. **Given** a QSO whose remote position resolves to the same coordinates as a QSO already shown,
   **When** it arrives, **Then** no new marker is added. The existing marker takes the color of the most
   recent QSO's band, and its popup lists every QSO at that point that is still within the displayed
   window.
4. **Given** a QSO with no station position in the record, **When** it is processed, **Then** the
   configured default station position is used for the geodesic line.

---

### User Story 2 - Remote position resolved from lookup services (Priority: P2)

The incoming QSO has no usable remote position (no LAT/LON or GRIDSQUARE). The service queries every
configured lookup source and keeps the most precise position it gets: exact coordinates first, then a
6-character locator, then a 4-character locator, then the ham-radio country centre. The result is added
to the QSO record, and the QSO is shown on the map.

**Why this priority**: Many contest exchanges carry no locator (serial numbers, zones). Without lookup,
most HF contest QSOs could not be placed on the map.

**Independent Test**: Configure at least the local country data and one online provider. Send a QSO
that has only a callsign. Verify that the marker is placed at the best available precision, and that the
record broadcast to pages contains the resolved position plus its source and precision.

**Acceptance Scenarios**:

1. **Given** providers that return exact coordinates, a 6-char locator and a country, **When** they are
   all queried, **Then** the exact coordinates are used.
2. **Given** that only a 4-char locator and a country are available, **When** the position is resolved,
   **Then** the centre of the 4-char square is used.
3. **Given** a callsign that resolves only to a ham-radio country (e.g. a Sardinia prefix), **When** the
   position is resolved, **Then** the centre of that ham-radio country is used (Sardinia, not Italy).
4. **Given** that every online provider fails or times out, **When** local country data matches the
   callsign, **Then** the country centre is used and the QSO is not delayed beyond the lookup timeout.
5. **Given** a QSO that already carries LAT/LON or GRIDSQUARE from the logger, **When** it is enriched,
   **Then** no online lookup is made for its position.

---

### User Story 3 - Viewer selects how many recent QSOs to show (Priority: P2)

A viewer opens the map partway through a contest and immediately sees the last 20 QSOs. They switch the
selector to 100. The page fetches the extra history from the service and shows the last 100 QSOs. Other
viewers' pages are not affected.

**Why this priority**: Without history, a newly opened page would stay empty until the next QSO, which
makes the map useless for spectators who join late.

**Independent Test**: Send 150 QSOs, then open a new page. It shows the most recent 20. Select 100 and
it shows the most recent 100. Select 10 and it shows the most recent 10. A second page left at 20 is
unchanged throughout.

**Acceptance Scenarios**:

1. **Given** 150 QSOs in the service history, **When** a new page opens, **Then** it shows the 20 most
   recent QSOs.
2. **Given** a page that shows 20, **When** the viewer selects 500 and the history holds 150, **Then**
   all 150 are shown.
3. **Given** a page that shows N QSOs, **When** a new QSO arrives, **Then** it is added and the oldest
   one is removed from that page. A marker is removed once none of its QSOs is among the last N.
4. **Given** a history limit of 1000, **When** QSO number 1001 arrives, **Then** the oldest QSO is
   removed from the history.

---

### User Story 4 - Corrections and resends are handled without duplicates (Priority: P3)

A logger resends a QSO, either a repeated packet or an edit after the operator corrects the exchange, the
locator or even the callsign. The service recognises it as the same QSO. An identical resend is ignored.
A resend with changed content replaces the earlier version on every open page, without adding a new
entry.

**Why this priority**: Loggers resend and edit QSOs regularly. Without this, maps fill up with duplicate
entries and keep showing stale QSOs.

**Independent Test**: Send the same QSO twice; the map shows one entry. Send a correction that changes
its callsign, using the same logger QSO id; the existing entry is updated and the QSO count stays the
same.

**Acceptance Scenarios**:

1. **Given** a QSO already received, **When** a QSO with the same identity and identical content
   arrives, **Then** it is ignored.
2. **Given** a QSO already received, **When** a QSO with the same identity but different content
   arrives, **Then** it replaces the stored QSO and every open page updates the existing entry in place,
   keeping its original place in time ordering.
3. **Given** a QARTest QSO with logger id `3809734F4`, **When** a `contactreplace` with the same id
   changes the callsign or time, **Then** the existing entry is updated rather than a new one added.
4. **Given** two QSOs from a source without logger ids, with the same remote callsign and band but
   different modes, **When** both arrive, **Then** both are kept as separate QSOs.
5. **Given** a contest dupe (same station worked again, but a different identity), **When** it arrives,
   **Then** it is shown like any other QSO and is not flagged.

---

### User Story 5 - QSOs from QARTest are shown on the map (Priority: P3)

A station that logs with QARTest sends its QSO messages to the same UDP port. The service recognises the
QARTest format from the packet content, converts each message to an ADIF QSO, and processes it exactly
like a plain ADIF QSO. QARTest's periodic score messages, sent to the same port, are ignored.

**Why this priority**: QARTest is widely used in contests. Supporting it widens the user base, but the
pipeline already works without it.

**Independent Test**: Replay the reference capture `reference/qartest-16.9.1-cqww-ssb.txt` to the
service port. The four new QSOs appear on the map. The edits update the existing entries, and the score
messages produce nothing.

**Acceptance Scenarios**:

1. **Given** a QARTest `contactinfo` message, **When** it arrives on the UDP port, **Then** it is
   identified as QARTest without any per-sender configuration and converted to an ADIF QSO following
   the QARTest mapping (FR-004a).
2. **Given** the `contactinfo` for DL5QFN (`<band>40</band>`, `<txfreq>700000</txfreq>`, `<mode>LSB</mode>`,
   `<timestamp>2026-10-08 15:04:04</timestamp>`), **When** it is converted, **Then** the ADIF QSO has
   BAND 40m, FREQ 7.000, MODE SSB, SUBMODE LSB, QSO_DATE 20261008 and TIME_ON 150404.
3. **Given** a QARTest `contactreplace` message, **When** it arrives, **Then** it is treated as a
   correction of the QSO with the same logger id.
4. **Given** a QARTest `dynamicresults` (score) message, **When** it arrives, **Then** it is ignored
   without a warning.
5. **Given** plain ADIF and QARTest messages arriving on the same port, **When** both are processed,
   **Then** each is recognised and parsed correctly.

---

### Edge Cases

- **Malformed or unrecognised datagram** (truncated ADIF, invalid field lengths, unknown format, ADX):
  it is logged and discarded. Other records in the same datagram that can be parsed are still
  processed, and the service keeps running.
- **No remote position at all** (unknown prefix and no provider result): the QSO is still stored and
  broadcast and counts toward the last N, but it gets no marker. A warning is logged.
- **No station position** in the QSO or the configuration: the remote marker is shown without a
  geodesic line.
- **Different station positions** at the same marker (multi-station setups): the line is drawn from the
  station position of the most recent QSO at that marker.
- **Invalid locator** (wrong characters or a length other than 4, 6 or 8): it is treated as absent and
  the next precision level applies.
- **Edit that changes an identity-key field, from a source without logger ids** (e.g. plain ADIF with
  the callsign corrected): it is treated as a new QSO and the original entry remains. This is a known
  limitation. QARTest is not affected, because it has logger ids.
- **Correction of a QSO that is not in the history** (e.g. it was logged before the service started,
  or has been evicted): it is treated as a new QSO.
- **QSO deleted in the logger** (`contactdelete`): it is ignored, and the QSO stays on the map until
  it falls out of the displayed window or the history. Deletion is out of scope for this feature.
- **QARTest message with an empty or zero frequency**: the band comes from the `<band>` field instead.
- **Empty QARTest fields** (e.g. `<operator></operator>`): they are treated as absent and are not
  written to the ADIF record.
- **Lookup provider slow or unavailable**: each provider is bounded by a timeout. Failures are logged,
  and the QSO continues with the best result obtained.
- **Page connection lost**: the page reconnects automatically and refills its last N from the service
  history.
- **Service restart**: the history is lost and open pages refill with whatever arrives after the
  restart. This is accepted.
- **Burst of QSOs** (e.g. a logger replaying several records at once): every QSO is processed, and none
  is dropped while within the capacity target (SC-004).

## Requirements *(mandatory)*

### Functional Requirements

**Ingestion**

- **FR-001**: The service MUST listen on one configurable UDP port for QSO messages from any sender.
- **FR-002**: The service MUST identify the format of each datagram from its content alone. No
  per-sender configuration is needed.
- **FR-003**: The service MUST accept plain ADIF datagrams with an optional header and one or more
  records, and MUST treat each record as a separate QSO.
- **FR-004**: The service MUST accept QARTest UDP QSO messages and convert each one to an ADIF QSO. A
  datagram is QARTest when it is an XML document whose root element is `contactinfo`,
  `contactreplace`, `contactdelete` or `dynamicresults`, and whose `<logger>` (or `<soft>`) value
  starts with `QARTest`. Each QARTest datagram carries exactly one message.
  - `contactinfo`: a new QSO.
  - `contactreplace`: a correction (FR-009).
  - `contactdelete`: a deletion, ignored in this feature (FR-005a).
  - `dynamicresults`: a score message, ignored in this feature (FR-005a).
- **FR-004a**: QARTest fields MUST be converted to ADIF as follows. Empty elements are treated as
  absent.

  | QARTest element | ADIF field(s) | Rule |
  |-----------------|---------------|------|
  | `call` | CALL | as is |
  | `mycall` | STATION_CALLSIGN | as is |
  | `operator` | OPERATOR | when not empty |
  | `timestamp` (`YYYY-MM-DD HH:MM:SS`, UTC) | QSO_DATE, TIME_ON | `YYYYMMDD`, `HHMMSS` |
  | `txfreq` (units of 10 Hz) | FREQ (MHz), BAND | BAND derived from the frequency using the ADIF band plan |
  | `band` (metres) | BAND | used only when `txfreq` is missing or zero |
  | `mode` | MODE, SUBMODE | LSB/USB → MODE SSB + SUBMODE LSB/USB; other values mapped to the ADIF mode list |
  | `snt`, `rcv` | RST_SENT, RST_RCVD | as is |
  | `exch1`, `exch2`, `exch3` | SRX_STRING | the non-empty values, joined by single spaces |
  | `wpxprefix` | PFX | as is |
  | `contestname` | CONTEST_ID | as is |
  | `logger` | PROGRAMID, PROGRAMVERSION | e.g. `QARTest 16.9.1` → `QARTest`, `16.9.1` |
  | `id` | logger QSO id (FR-007) | also stored as an application-specific field |
  | `countryprefix`, `nr`, `points`, `duplicate`, `xqso`, `stationname` | application-specific fields | stored for information only; not used for position, identity or display |
  | `oldcall`, `oldtimestamp` | none | used only to match a correction when no logger id is present |

- **FR-005**: The service MUST log and discard datagrams in unrecognised formats (including ADX). This
  feature handles only plain ADIF and QARTest.
- **FR-005a**: QARTest score messages (`dynamicresults`) and delete messages (`contactdelete`) MUST be
  ignored and logged only at debug level. They are recognised, not malformed. Deleting QSOs is out of
  scope for this feature.
  > **Changed by feature 007**: score messages are now read and shown as the contest score line;
  > delete messages are still ignored. See
  > [`../007-contest-score-layout/spec.md`](../007-contest-score-layout/spec.md).
- **FR-006**: Every QSO MUST leave the ingestion stage as a valid ADIF record, whatever its source.

**Duplicate and correction handling**

- **FR-007**: Each QSO MUST have an identity.
  - When the source message carries a logger QSO id (e.g. QARTest `<id>`), the identity is the station
    callsign plus the logger id.
  - Otherwise the identity is the composite key: station callsign, remote callsign, band, mode, QSO
    date, and QSO start time truncated to the minute.
  - Comparison MUST ignore letter case and surrounding whitespace.
- **FR-008**: A QSO whose identity and content match a QSO in the history MUST be ignored.
- **FR-009**: A QSO whose identity matches a QSO in the history but whose content differs MUST replace
  that QSO. It MUST be broadcast as an update to the existing entry, not as a new QSO. This applies to
  QARTest `contactreplace` messages even when the callsign or time has changed. Without a logger id, a
  correction is matched using `oldcall` and `oldtimestamp`.
- **FR-010**: Contest dupes (QSOs with a different identity) MUST pass through unchanged and MUST NOT
  be flagged. QARTest's own `<duplicate>` flag is stored but not acted on.

**Enrichment: station**

- **FR-011**: Station information (station callsign and station position) MUST come from the QSO
  record when present. The configured defaults MUST be used only to fill fields the record lacks.

**Enrichment: remote position**

- **FR-012**: If the QSO already has the remote station's coordinates or locator, that position MUST be
  used and no online lookup made for it.
- **FR-013**: Otherwise the service MUST query every configured lookup source: QRZ.com, QRZCQ and
  HamQTH (each only when its credentials are configured) and the local ham-radio country data. It MUST
  keep the most precise position, in this order: exact coordinates, 6-character locator, 4-character
  locator, ham-radio country.
- **FR-014**: A locator MUST be converted to the coordinates of the centre of its square. A ham-radio
  country MUST be converted to the reference centre point of that country, with countries following
  the amateur radio entity list rather than political borders (e.g. Italy, Sardinia and Sicily are
  separate countries).
- **FR-015**: Each online lookup MUST be bounded by a configurable timeout. If a provider fails, the
  QSO MUST continue with the remaining sources.
- **FR-016**: Lookup results MUST NOT be cached. Each QSO that needs a lookup queries the sources again.
- **FR-017**: The resolved information (position, locator, country and any other details providers
  return) MUST be added to the QSO record using standard ADIF fields where they exist. Values supplied
  by the logger MUST NOT be overwritten.
- **FR-018**: The record MUST state where the remote position came from (logger, provider name or
  country data) and its precision (coordinates, 6-char, 4-char or country).
- **FR-019**: A QSO whose position cannot be resolved MUST still be stored and broadcast, without
  coordinates.

**Distribution and history**

- **FR-020**: Every change to the history MUST be pushed live to every open map page: a new QSO or an
  updated QSO.
- **FR-021**: The service MUST keep an in-memory history of enriched QSOs, with a configurable maximum
  size (0 = unbounded). It evicts the oldest QSOs when full. Losing it on restart is acceptable.
- **FR-022**: When a page opens or asks for more QSOs, the service MUST send the most recent QSOs from
  its history, as many as the page requests.

**Map page**

- **FR-023**: The service MUST serve the map page on one configurable TCP port.
- **FR-024**: The map MUST show a marker for each distinct remote position among the displayed QSOs.
  QSOs whose positions are exactly equal MUST share a single marker.
- **FR-025**: A marker's color MUST be the configured color of the band of the most recent QSO it
  holds. Unknown bands MUST use a configured fallback color. The map MUST show a band-color legend.
- **FR-026**: A marker's popup MUST list each displayed QSO at that position: remote callsign, band,
  mode, date/time, and position source and precision.
- **FR-027**: For each marker, the map MUST draw a geodesic (great-circle) line from the station
  position to the marker.
- **FR-028**: Markers MUST be stacked by QSO date/time, with the most recent on top. A corrected QSO
  keeps its original place in time ordering.
- **FR-029**: Each page MUST have its own selector for how many recent QSOs to show: 10, 20, 50, 100,
  200, 500 or All (everything in the service history). The default is 20.
- **FR-030**: "Most recent" MUST be ordered by QSO date and start time. A marker MUST be removed when
  none of its QSOs is among the displayed ones.
- **FR-031**: A page MUST reconnect automatically after losing its connection and then refill its
  displayed QSOs.

**Configuration and deployment**

- **FR-032**: All settings MUST be configurable through command-line options, environment variables and
  a configuration file. Command line overrides environment, which overrides the file.
- **FR-033**: Settings MUST include at least: UDP port (default 12060), HTTP port (default 8080), bind
  address (default all interfaces), history size (default 10000, 0 = unbounded), default station
  callsign and position, lookup provider credentials, provider order, lookup timeout, location of the
  local country data, and the band color palette with a fallback color.
- **FR-034**: The service MUST be able to run on a small single-board computer or mini PC, on a virtual
  machine and on a cloud host. It MUST need only the UDP and TCP ports to be reachable.
- **FR-035**: The map page and the UDP port MUST be openly accessible, with no authentication or
  source filtering, as the project owner decided.

### Key Entities

- **QSO record**: one contact as an ADIF record: remote callsign, band, mode, date/time, station
  information and enrichment results. Identified by its identity (FR-007).
- **Identity**: station callsign + logger QSO id when the source provides one. Otherwise station
  callsign + remote callsign + band + mode + QSO date + start time to the minute.
- **QSO event**: what a source message means: new QSO or correction. Score and delete messages carry
  no event in this feature.
- **Source format**: the recognised datagram format (plain ADIF, QARTest) and how it maps to ADIF.
- **Position**: latitude/longitude plus its precision (coordinates, 6-char, 4-char, country) and its
  source (logger, provider, country data).
- **Lookup source**: an online provider (QRZ.com, QRZCQ, HamQTH) or the local country data, each with
  its credentials, order and timeout.
- **Ham-radio country**: an amateur radio entity with its callsign prefixes and reference centre point.
- **History**: an ordered, bounded, in-memory set of enriched QSOs that is shared by all pages.
- **Map page session**: one open page with its own selected N and its own live connection.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a QSO that already contains its remote position, the marker appears on every open
  page within 2 seconds of the datagram being sent, in 95% of cases.
- **SC-002**: For a QSO that needs online lookup, the marker appears within the lookup timeout plus 2
  seconds, even when a provider is unreachable.
- **SC-003**: A newly opened page shows the most recent 20 QSOs within 3 seconds.
- **SC-004**: On a Raspberry Pi 4-class host, the service handles a burst of 10 QSOs per second for one
  minute, with 50 open pages, and loses no QSO.
- **SC-005**: 100% of identical resends are ignored, and 100% of corrections update the existing entry
  without changing the QSO count.
- **SC-006**: Of malformed datagrams in a fuzzed test set, 100% are discarded without stopping the
  service, and valid records in the same datagrams are still delivered.
- **SC-007**: QSOs that resolve only to a country appear at the correct ham-radio country centre (e.g.
  IS0 prefixes in Sardinia, not mainland Italy) in 100% of a reference test set.
- **SC-008**: An operator can deploy and start the service with only a configuration of station
  defaults and ports, in under 10 minutes, on each of the three target environments.
- **SC-009**: Replaying the reference QARTest capture (10 datagrams) gives exactly 5 entries and no
  warnings. DL5QFN, B5Q, K6QA and K2WQX are new. K6QA is updated in place twice. HB9FQZ appears once:
  its QSO is not in the history, so the first correction creates it and the second updates it. The 2
  score messages produce nothing.

## Assumptions

- Viewers' browsers have Internet access, because map tiles and online lookups require it. The service
  host needs Internet access only for online lookups.
- Contest-specific logic (scoring, multipliers, exchange validation, dupe flagging) is out of scope.
  The feature is generic for any contest.
- An 8-character locator counts as "6-character or better" precision and is placed at the centre of
  its 8-character square.
- When several providers return the same precision level, the first one in the configured order wins.
  For non-position fields, the first provider in that order that has a value wins.
- QSOs without a position count toward a page's last N even though they draw nothing. No QSO list
  panel is part of this feature.
- Provider usage limits (e.g. daily lookup quotas) are the operator's responsibility. Since lookups are
  not cached (FR-016), every QSO needing a lookup counts against those limits.
- The default lookup timeout is 5 seconds per provider, configurable.
- QARTest timestamps are UTC. The reference capture confirms this: packet received 15:04:09Z,
  timestamp 15:04:04.
- The reference capture is QARTest 16.9.1. Other versions are assumed compatible until shown
  otherwise.
- Accepting UDP input from anyone (FR-035) means anyone who can reach the port can inject QSOs into the
  public map. The project owner accepts this risk for this feature.
- The project owner's stated design direction, carried forward to planning: queues between the
  ingestion, enrichment and distribution stages that support several producers and consumers, one live
  connection per open page with a broadcast to all of them, and a web map library for rendering.
