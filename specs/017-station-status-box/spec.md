# Feature Specification: Station Status Box

**Feature Branch**: `feature/017-station-status-box`
**Created**: 2026-10-24
**Status**: Draft
**Input**: User description: "Analyze the broadcasted messages (both QSO and score), and detect if
there is the information of the current operator callsign (in QARTest there is a special OPON
command to change the current operator). The idea is to add in the frontend a box with current
station status information (band and operator call, for the moment, taken from the last logged QSO
or any other way to retrieve the information)"

## Problem Summary

During a multi-operator contest, viewers of the map can see what the station worked and its score,
but not what the station is doing now: which band it is on and who is at the radio. In QARTest the
operator at the radio is set with the OPON command, and every QSO logged afterwards is stamped with
that operator.

### What the broadcast messages carry (analysis)

Checked against the reference capture of QARTest 16.9.1 (CQ WW SSB) used since feature 001:

| Message | Operator | Band | Notes |
|---------|----------|------|-------|
| New QSO (`contactinfo`) | `operator` element, empty when OPON was never used | `band` and transmit frequency | Already read; kept on the QSO as `OPERATOR` |
| QSO correction (`contactreplace`) | same as new QSO | same as new QSO | Can correct a QSO logged long ago |
| QSO delete (`contactdelete`) | — | — | Ignored today |
| Score (`dynamicresults`) | **none** | none for "now" (per-band totals only) | Carries the station callsign and the category (for example `MULTI-OP`) |
| ADIF datagram (other loggers) | optional `OPERATOR` field | `BAND` / `FREQ` | Same meaning as in QARTest QSOs |

Conclusion: the operator is not broadcast on its own when OPON is typed, and score messages never
carry it. The only source is the operator stamped on each logged QSO. The current operator and band
are therefore those of the most recent logged QSO; an operator change becomes visible with the
first QSO that operator logs.

After this feature, every page shows a small station status box with the station's current band and
current operator, taken from the most recent QSO, updated live as QSOs arrive.

## Clarifications

### Session 2026-10-24

- Q: What does the operator line show when the latest QSO has no operator? → A: The station
  callsign of that QSO (the same one the service already uses for the station, falling back to the
  configured default station); a dash only when neither is known.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the band and operator now on the air (Priority: P1)

A viewer opens the map during a multi-operator contest and sees, in a dedicated box, the band the
station is working and the callsign of the operator at the radio. When the station logs a QSO on
another band, or a new operator takes over and logs a QSO, the box follows within seconds.

**Why this priority**: This is the whole request.

**Independent Test**: Send QSOs with different bands and operators to a running service and watch
the box on an open page.

**Acceptance Scenarios**:

1. **Given** a page is open and the latest QSO was logged on 40m by operator IK0ABC, **When** the
   page is shown, **Then** the box reads band `40m` and operator `IK0ABC`.
2. **Given** a page is open, **When** a QSO on 20m by operator IS0XYZ arrives, **Then** the box
   shows `20m` and `IS0XYZ` within 2 seconds, without a reload.
3. **Given** a page is opened after QSOs were logged, **When** it loads, **Then** the box shows the
   band and operator of the most recent QSO at once.
4. **Given** the service restarts and loads its saved QSO history (feature 014), **When** a page
   opens before any new QSO, **Then** the box shows the band and operator of the most recent saved
   QSO.
5. **Given** a correction arrives for a QSO logged an hour ago, **When** the box is shown, **Then**
   it is unchanged: it still reflects the most recent QSO by QSO time.

---

### User Story 2 - Know when the status is old or unknown (Priority: P2)

A viewer can tell when the information may no longer reflect the station: nothing logged yet, no
operator recorded, or no QSO for a while.

**Why this priority**: A band or operator from an hour ago, shown as "now", misleads viewers.

**Independent Test**: Start with an empty history, then log one QSO with an empty operator, then
wait past the staleness threshold.

**Acceptance Scenarios**:

1. **Given** no QSO has been logged, **When** the page is shown, **Then** the box shows a dash for
   band and operator.
2. **Given** the latest QSO has no operator (OPON never used), **When** the page is shown, **Then**
   the operator line shows the station callsign (for example `IS0MZK`), or a dash when no station
   callsign is known.
3. **Given** the most recent QSO is older than 15 minutes, **When** the page is shown, **Then** the
   box is greyed out and its tooltip says when the last QSO was logged.

### Edge Cases

- **Operator change without a QSO**: OPON typed, no QSO logged yet. The box keeps the previous
  operator until the new operator logs a QSO; no message tells the service earlier.
- **Correction of the latest QSO**: if a correction changes the band or operator of the most recent
  QSO, the box follows the corrected values.
- **Deleted QSO**: deletes are ignored (feature 001), so a deleted latest QSO still drives the box.
- **QSO with a band the service does not recognise, or no band**: the band line shows a dash; the
  operator still shows.
- **Band filter**: the page's band filter (feature 003) does not affect the box; it always reflects
  the station.
- **Several stations sending to one service**: the box reflects the most recent QSO received,
  whatever station logged it.
- **Clock differences**: staleness compares the QSO's UTC time with the service's clock, never the
  browser's, so it also works after a restart. A logger clock that is wrong by minutes shifts the
  greying by the same amount.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every page MUST show a station status box with two values: current band and current
  operator.
- **FR-002**: The current band and operator MUST be those of the most recent QSO in the service's
  history, ordered by QSO date and time, from any supported source (QARTest or ADIF).
- **FR-003**: The box MUST update on every open page within 2 seconds of a QSO that changes the most
  recent QSO, without a reload.
- **FR-004**: A page that opens, or reconnects, MUST show the current status at once.
- **FR-005**: After a restart, the status MUST come from the most recent QSO of the loaded history.
- **FR-006**: With no QSO in the history, the box MUST show a dash for both lines.
- **FR-007**: When the most recent QSO has no operator, the operator line MUST show that QSO's
  station callsign, falling back to the configured default station, and a dash when neither is
  known.
- **FR-008**: When the most recent QSO is more than 15 minutes old, the box MUST be greyed out, with
  a tooltip giving the QSO's time in UTC.
- **FR-009**: The band filter MUST NOT change what the box shows.
- **FR-010**: Score messages MUST keep being handled as today; this feature reads nothing new from
  them.
- **FR-011**: The box MUST follow the page's theme (feature 011) and MUST NOT cover the map's
  controls, the score line or the last-QSO box at 1024 px wide and above.

### Key Entities

- **Station status**: the band and operator of the most recent QSO, with that QSO's date and time;
  empty when the history is empty.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a test with 10 QSOs alternating bands and operators, every open page shows the
  band and operator of each new QSO within 2 seconds, 10 times out of 10.
- **SC-002**: A page opened at any moment shows the same status as pages that were already open.
- **SC-003**: After a restart with a saved history, the status shown equals that of the latest saved
  QSO before any new QSO arrives.
- **SC-004**: With no QSO for 15 minutes, every page greys the box out within 10 seconds of the
  threshold.
- **SC-005**: Viewers can name the station's current band and operator from the page alone, without
  opening the QSO list.

## Assumptions

- **Source of the operator**: the QSO's `OPERATOR`; QARTest broadcasts no separate operator-change
  message and its score message carries none (analysis above). If a future QARTest version sends
  one, it is a later feature.
- **"Most recent"**: by QSO date and time, not arrival order, so corrections and late QSOs of the
  past do not move the box backwards.
- **Band format**: as elsewhere on the page (for example `40m`), taken from the QSO's band.
- **Staleness threshold**: 15 minutes, fixed. A contest station can go several minutes between QSOs
  without leaving the band; the score's 5 minutes would grey the box too often.
- **Out of scope**: mode, frequency, rig, the station callsign as a third line, a history of
  operator changes, per-operator statistics, any setting to hide or move the box.
- **Layout**: from 1024 px wide up, as in feature 007; the exact place is a plan decision.
