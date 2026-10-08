# Feature Specification: Band Filter

**Feature Branch**: `003-band-filter`
**Created**: 2026-10-09
**Status**: Complete (2026-10-09). See `checklists/verification.md`.
**Input**: User description: "Add a configuration for specify which bands have to be considered.
Ignored bands must not appear in frontend and must be ignored in the whole backend pipeline."

Terms follow `docs/ham-radio.md` §2. Band lists follow HR-2: always in frequency order.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Only the considered bands reach the map (Priority: P1)

The operator configures the service to consider only the six HF contest bands. During the contest,
the logger also sends QSOs from a 2m liaison station and a 60m test. Those QSOs never appear on any
map page, are not stored, and cause no callbook lookups. The legend lists only the six considered
bands.

**Why this priority**: This is the feature. Without it, out-of-scope QSOs clutter the map and use up
the provider lookup quota.

**Independent Test**: Configure `hf-contest`. Send one QSO on 20m and one on 2m. Only the 20m QSO
reaches the history and the pages. A counting lookup source sees exactly one call. The legend in
`hello` lists exactly 160m, 80m, 40m, 20m, 15m and 10m, in that order.

**Acceptance Scenarios**:

1. **Given** the filter `hf-contest`, **When** a QSO on 20m arrives, **Then** it is processed and
   shown as today.
2. **Given** the filter `hf-contest`, **When** a QSO on 2m or 60m arrives, **Then** it is dropped
   before any lookup, is not stored, and reaches no page.
3. **Given** the filter `hf-contest`, **When** a page connects, **Then** its legend lists only the
   considered bands, in frequency order.
4. **Given** no filter configured, **When** QSOs on any band arrive, **Then** behaviour is unchanged
   from feature 001.

---

### User Story 2 - Combine groups, single bands and exclusions (Priority: P2)

The operator selects bands with any mix of named groups, single bands and exclusions. For example,
`hf,2m,-60m` means every HF band plus 2m, except 60m. The effective set is shown at startup so the
operator can check it.

**Why this priority**: Groups make common setups a single word. Exclusions avoid listing ten bands
to drop one.

**Independent Test**: Load configurations with several selections and compare the effective band
sets, plus the startup log line, against the expected lists.

**Acceptance Scenarios**:

1. **Given** `hf-contest`, **Then** the effective set is 160m 80m 40m 20m 15m 10m.
2. **Given** `hf,2m,-60m`, **Then** it is 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m.
3. **Given** `warc,vhf`, **Then** it is 30m 17m 12m 8m 6m 5m 4m 2m 1.25m.
4. **Given** `-60m` (exclusions only), **Then** it is every band except 60m.
5. **Given** `HF-Contest, 40M` (mixed case, spaces, overlapping), **Then** it is the same as
   `hf-contest`.
6. **Given** an unknown name such as `hf-dx` or `21m`, **Then** the service refuses to start and
   names the bad entry.
7. **Given** a selection whose result is empty (e.g. `warc,-warc`), **Then** the service refuses to
   start.

---

### User Story 3 - A correction onto an ignored band removes the QSO (Priority: P3)

A QSO was logged on 20m and is on the map. The operator notices it was really made on 60m and
corrects it in the logger. The logger resends the corrected QSO. Because 60m is ignored, the QSO
disappears from every open page and from the history.

**Why this priority**: Without this, the map would keep showing a QSO on a band the logger no longer
has. Corrections are less common than new QSOs.

**Independent Test**: With the filter `hf-contest`, send a QARTest `contactinfo` on 20m, then a
`contactreplace` with the same logger id on 60m (5 MHz). Every connected page receives a removal for
that QSO, and the history no longer holds it.

**Acceptance Scenarios**:

1. **Given** a shown QSO on a considered band, **When** a correction moves it to an ignored band,
   **Then** it is removed from the history and every open page removes it, including its marker if
   the marker has no other QSOs.
2. **Given** a QSO removed this way, **When** a later correction moves it back to a considered band,
   **Then** it appears again as a QSO.
3. **Given** a correction to an ignored band for a QSO that was never shown, **When** it arrives,
   **Then** it is dropped like any other ignored QSO and no removal is sent.

---

### Edge Cases

- **Unknown band while a filter is active** (no `BAND`, no usable frequency, or a frequency outside
  every band): the QSO is dropped. With no filter configured it passes, as in feature 001.
- **Band colours configured for ignored bands**: accepted, but not sent to pages and not shown in the
  legend.
- **Duplicate entries** (`hf,20m` or a band in two groups): counted once.
- **The same band added and excluded** (`20m,-20m`): exclusions win, so 20m is ignored.
- **Resend of a dropped QSO**: dropped again, at no cost.
- **Configuration changes**: the filter is read at startup. Changing it requires a restart, which
  also clears the history (feature 001: the history is in memory only).
- **Score or delete messages**: unaffected; still ignored as in feature 001.

## Requirements *(mandatory)*

### Functional Requirements

**Configuration**

- **FR-001**: The service MUST accept a band selection setting through the command line, the
  environment and the configuration file, with the usual precedence (feature 001, FR-032).
- **FR-002**: A selection MUST be a list of entries. Each entry is an ADIF band name (e.g. `20m`), a
  group name (FR-003), or either of those prefixed with `-` to exclude it. Names MUST be
  case-insensitive and surrounding whitespace ignored.
- **FR-003**: The service MUST provide these groups. Each lists its bands in frequency order:

  | Group | Bands |
  |-------|-------|
  | `lf-mf` | 2190m, 630m, 560m |
  | `hf` | 160m, 80m, 60m, 40m, 30m, 20m, 17m, 15m, 12m, 10m |
  | `hf-contest` | 160m, 80m, 40m, 20m, 15m, 10m |
  | `warc` | 30m, 17m, 12m |
  | `vhf` | 8m, 6m, 5m, 4m, 2m, 1.25m |
  | `uhf` | 70cm, 33cm, 23cm |
  | `shf` | 13cm, 9cm, 6cm, 3cm, 1.25cm |
  | `ehf` | 6mm, 4mm, 2.5mm, 2mm, 1mm, submm |

- **FR-004**: The effective band set MUST be the union of the positive entries, minus the union of
  the excluded entries. A selection with no positive entries starts from every band. An absent or
  empty setting means every band is considered and no filter is active.
- **FR-005**: The service MUST refuse to start, with a message naming the entry, when an entry is
  neither a band nor a group, or when the effective set is empty.
- **FR-006**: At startup the service MUST log the effective band set in frequency order, or that
  every band is considered.

**Pipeline**

- **FR-007**: When a filter is active, a QSO MUST be dropped as soon as its band is known after
  ingestion and normalisation, when that band is not in the effective set or is unknown. A dropped
  QSO MUST NOT reach duplicate handling, lookups, the history or any page, and MUST be logged at
  debug level only.
- **FR-008**: When a dropped QSO is a correction of a QSO currently in the history (same identity,
  feature 001 FR-007), that QSO MUST be removed from the history. A removal MUST be pushed to every
  open page, and the service MUST forget the QSO, so that a later correction back to a considered
  band is treated as new.
- **FR-009**: With no filter active, the pipeline MUST behave exactly as in feature 001.

**Map pages**

- **FR-010**: The band legend and colour list sent to pages MUST contain only considered bands, in
  frequency order (HR-2).
- **FR-011**: On receiving a removal, a page MUST drop that QSO and recompute its window and
  markers. A marker with no remaining QSOs disappears.
- **FR-012**: A page MUST never display a QSO on an ignored band. FR-007 and FR-008 guarantee it
  never receives one.

### Key Entities

- **Band selection**: the configured list of entries (bands, groups, exclusions).
- **Band group**: a named, fixed list of bands in frequency order.
- **Effective band set**: the bands that result from the selection, in frequency order; "all" when
  no filter is active.
- **Removal**: a notification to pages that a QSO no longer exists, identified by the QSO's id.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a filter active, 0% of QSOs on ignored or unknown bands reach the history, a page
  or a lookup source, across a mixed test stream of at least 1,000 QSOs on all bands.
- **SC-002**: 100% of QSOs on considered bands are processed exactly as without the filter, in the
  same test stream.
- **SC-003**: Every documented selection example (User Story 2) yields exactly its expected band
  set, and every invalid selection stops the service with a message naming the bad entry.
- **SC-004**: A correction onto an ignored band removes the QSO from every open page within 2
  seconds of the datagram being sent.
- **SC-005**: Every band list shown or sent to pages is in frequency order. A test checks this for
  every group and for the full set.

## Assumptions

- The selection is static for the life of the process (FR-006 and the configuration-changes edge
  case). Live reconfiguration is out of scope.
- Removal is only introduced for corrections onto ignored bands. Logger delete messages
  (`contactdelete`) remain out of scope, although a future delete feature could reuse the same
  removal path.
- Group membership is fixed in the service; user-defined groups are out of scope.
- QSOs from QARTest and from ADIF are filtered alike, using the band each normalises to (feature
  001, and `docs/ham-radio.md` HR-3).
- Decisions confirmed with the project owner on 2026-10-09:
  - allow-list with groups, and exclusions with `-`;
  - the group definitions in FR-003;
  - unknown bands are dropped while a filter is active;
  - a correction onto an ignored band removes the QSO everywhere;
  - QSOs are dropped at ingest, before everything else.
