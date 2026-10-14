# Feature Specification: QSOs Kept in a Local ADIF File

**Feature Branch**: `feature/014-adif-persistence`
**Created**: 2026-10-14
**Status**: Draft
**Input**: User description: "The feature must use a local database for storing data related to
QSOs, using a local file as DB, in order to keep the binary standalone. A simple delete of the
local file must be enough to reset the persistence (and of course a reboot of the service). Every
data should be kept in memory, and updated in real time. At start, the binary must pre-fill the
memory reading data from the DB, if any. As format, use a simple ADIF file as DB. When loading
data at the beginning, analyze every record as a source, as they are received in the UDP socket,
in order to perform the same checks in geolocalization (if position is already present, use it,
or enrich the data using online services. Since QSO are stored in memory, perform load ad start,
and perform a dump of the memorized data at the closing of the software (at graceful shutdown).
This means that if someone load an incomplete ADIF, at software closing the ADIF must be updated
with enriched data. Add also a flag to perform only the enrichment, without starting the UDP
listener."

## Problem Summary

Today the QSO history lives only in memory: a restart in the middle of a contest empties the map,
and every QSO already worked is gone until the logger sends it again. The operator wants the
history to survive a restart without adding anything to install: the service keeps working from
memory as it does now, reads its history from a single ADIF file at startup, and writes it back
when it stops. Deleting the file (and restarting) is the whole reset procedure.

Because each record read from the file goes through the same checks as a QSO received from the
logger, the file also works as an input: an operator can place an ADIF log with missing positions
there, and the service completes it. A separate mode does only that — load, complete, save, exit —
without listening for the logger.

## Clarifications

### Session 2026-10-14

- Q: The history keeps at most `history_size` QSOs and drops the oldest; what happens to QSOs
  beyond that limit when the file is saved? → A: The service no longer limits its history: it
  keeps every QSO, and every QSO is saved. The `history_size` setting is removed. The page shows
  at most the 1000 newest QSOs.
- Q: How does the 1000 cap appear in the page's count menu? → A: "All" is replaced by 1000: the
  menu offers 10/20/50/100/200/500/1000. A saved "all" choice falls back to 1000 and is saved
  again as 1000.
- Q: What happens to a file record on a band the service does not consider (`--bands`), or with no
  band? → A: It is always kept in the file and never shown on the map. In enrich-only mode it is
  completed like any other record (looked up, saved back with its derived fields); in a normal run
  it is left untouched: no lookup, saved back exactly as it was read.
- Q: Is a record restored with a rejected-position note looked up again? → A: Not in a normal run:
  it keeps its note and its fallback position, with no new lookup. Enrich-only mode looks it up
  again.
- Q: What does the service do when the history file exists but cannot be read at startup? → A: It
  moves the file aside (renamed with a UTC timestamp), logs it, and starts with an empty history;
  if the file cannot be moved either, it refuses to start.
- Q: Which file does enrich-only mode work on? → A: The option takes an optional path: with a path
  it completes that file in place; without one it uses the configured history file.
- Q: What happens when a lookup fails (network error, timeout, quota, credentials) instead of
  finding nothing? → A: The country fallback is shown and saved with a lookup-failed note; records
  with that note are looked up again at the next start, in both modes, and a successful lookup
  clears the note.
- Q: What happens if the file is deleted while the service is running? → A: The graceful stop
  always saves, so the file comes back; the documented reset is stop, delete, start.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The map survives a restart (Priority: P1)

An operator runs the service during a contest. The service is stopped (an update, a reboot, a
configuration change) and started again. The map shows the same QSOs as before, at the same
positions, without the logger resending anything and without new callbook lookups for QSOs that
were already placed.

**Why this priority**: losing the map on every restart is the main problem this feature solves.

**Independent Test**: send a few QSOs, stop the service gracefully, start it again, open the page:
the same QSOs appear at the same positions, and the log shows no callbook lookup for them.

**Acceptance Scenarios**:

1. **Given** a running service with 20 QSOs on the map, **When** it is stopped gracefully and
   started again, **Then** the page shows the same 20 QSOs, with the same positions, precisions
   and entities.
2. **Given** QSOs whose positions were found online in the previous run, **When** the service
   starts, **Then** no callbook lookup is made for them, and their positions keep the precision
   and source they had (they are not presented as sent by the logger).
3. **Given** a service started with no history file, **When** it starts, **Then** it starts with
   an empty map, and it creates the file at the next graceful stop.
4. **Given** a running service, **When** the operator stops it, deletes the history file and
   starts it again, **Then** the map is empty.
5. **Given** the service restored its history at startup, **When** the logger resends or corrects
   one of those QSOs, **Then** it is merged with the restored QSO as it would have been before the
   restart (a resend is ignored, a correction replaces it), and a contest dupe is still kept as a
   separate QSO (HR-9).

---

### User Story 2 - An incomplete log is completed at startup and saved back (Priority: P2)

An operator places an ADIF log in the history file's location — for example exported from a
logger, with locators for some QSOs and nothing for others. At startup every record is treated
like a QSO arriving from the logger: records with a usable position keep it, the others are placed
with the usual lookups (callbook, then country data). When the service stops gracefully, the file
is rewritten with the completed records.

**Why this priority**: it reuses the same file and the same checks as User Story 1; it adds value
for operators who start from an existing log.

**Independent Test**: start the service on a file with 3 records with `GRIDSQUARE` and 3 without
any position; check the map shows all 6, then stop the service and check the file now has a
position, its precision and its source for all 6, and the 3 original locators unchanged.

**Acceptance Scenarios**:

1. **Given** a file record with a locator or coordinates sent by a logger, **When** it is loaded,
   **Then** that position is used and no callbook lookup is made for it.
2. **Given** a file record with no position, **When** it is loaded, **Then** it is placed with the
   same lookups and rules as a QSO received from the logger (callbook, then country data,
   placeholder and outside-entity checks, HR-7).
3. **Given** a completed record, **When** the service stops gracefully, **Then** the saved record
   keeps every field it had in the file unchanged, and only adds the fields that were absent
   (HR-11), such as the derived position with its precision and source and the rejected-position
   note.
4. **Given** a file of several thousand records loaded at startup, **When** lookups are still
   running, **Then** the page and the logger listener are already available; restored QSOs appear
   on the map as soon as each one is placed.

---

### User Story 3 - Enrich a file without running the service (Priority: P3)

An operator wants only a completed ADIF file — for example before or after a contest — without
starting the map or listening for the logger. A command-line option starts the program in a mode
that loads the history file, places every record as in User Story 2, writes the file back and
exits.

**Why this priority**: a convenience on top of User Story 2; the service works without it.

**Independent Test**: run the program with the enrich-only option on a file with records missing
positions; check it exits by itself, opens no network port, and leaves a file whose records all
carry a position (or a rejected-position note when none could be found).

**Acceptance Scenarios**:

1. **Given** the enrich-only option, **When** the program runs, **Then** it does not listen for
   the logger and does not serve the page.
2. **Given** the enrich-only option, **When** every record has been processed, **Then** the file
   is written with the completed records and the program exits with a success status.
3. **Given** the enrich-only option and a missing or unreadable history file, **When** the program
   runs, **Then** it reports the problem and exits with a failure status without creating or
   changing any file.
4. **Given** the enrich-only option, **When** the operator interrupts it (Ctrl+C) before the end,
   **Then** the file is written with what was completed so far, like a graceful stop of the
   service.

---

### Edge Cases

- **Malformed records** in the file are logged and skipped, like a malformed QSO from the logger;
  the rest of the file is loaded. The log states how many records were loaded and how many were
  skipped.
- **The file cannot be read** (permissions, I/O error, not ADIF at all): the service moves it aside
  under a name carrying the UTC time, logs where it went, and starts with an empty history, so the
  map comes up and a later save never overwrites the unreadable file. If it cannot be moved
  either, the service refuses to start (FR-011).
- **The same QSO twice in the file** (same identity): merged as a resend or a correction, in file
  order — the later record wins. Contest dupes stay separate QSOs (HR-9).
- **A large file**: the service keeps every QSO it loads or receives; nothing is dropped for
  size, and every QSO is saved. The page still shows at most the 1000 newest (FR-023).
- **A page that saved the old "All" choice**, or an older page asking for every QSO: it gets the
  1000 newest, and the page saves its choice as 1000 (FR-025).
- **Records on bands the service does not consider** (`--bands`, default `hf-contest`), or with no
  band: always kept in the file and never sent to the page. A normal run leaves it untouched (no
  lookup, saved back as read); enrich-only mode completes it like any other record (FR-026). A QSO
  from the logger on such a band is still dropped on arrival, as today.
- **File deleted while the service runs**: the graceful stop writes it again from memory, so the
  history is not reset. The reset is stop, delete, start (FR-003).
- **Stop while lookups are still running**: records not yet placed are saved as they are, without
  a derived position; they are placed at the next start.
- **Stop is not graceful** (power loss, forced kill, crash): the file keeps what the last graceful
  stop saved; QSOs received since then are lost. The logger can resend them.
- **Saving fails** (disk full, read-only location): the error is logged and the previous file is
  left intact; the program still exits. A save interrupted half-way never leaves a truncated or
  mixed file.
- **Restored QSO removed later**: a restored QSO corrected onto a band the service does not
  consider is removed, as today (feature 003), and is not in the next save.
- **Rejected positions**: a record restored with a rejected-position note keeps the note and its
  fallback position in a normal run, with no new lookup, so restarts spend no callbook quota on
  it. Enrich-only mode looks it up again, since the online record may have been fixed (FR-028).
- **Failed lookups**: a lookup that fails (network error, timeout, used-up quota, refused
  credentials) is not the same as "not found". The QSO is shown at the country fallback, saved
  with a lookup-failed note, and looked up again at the next start; a success clears the note
  (FR-029).
- **Logger identities**: QSOs from loggers that send their own QSO identifier keep it in the saved
  record, so a correction sent after the restart still finds the restored QSO.
- **Contest score**: the latest score is not saved; it shows again when the logger next sends it.

## Requirements *(mandatory)*

### Functional Requirements

**The history file**

- **FR-001**: The service MUST keep its QSO history in a single local ADIF file (HR-11). It MUST
  NOT need any other program, server or installed component to do so.
- **FR-002**: The file location MUST be configurable through the configuration file, the
  environment and the command line, like the other settings. Its default MUST follow the same
  placement rules as the downloaded country data (the data folder on Debian and in the container,
  the program's folder on Windows and macOS).
- **FR-003**: Stopping the service, deleting the file and starting it again MUST be enough to start
  from an empty history. No other reset step MUST be needed. A graceful stop MUST always save, even
  if the file was deleted while the service ran (it is then written again); the documentation MUST
  give the reset in that order: stop, delete, start.
- **FR-004**: The service MUST keep working from memory: the map, the page sessions and the logger
  pipeline MUST behave in real time as they do today. The file MUST NOT be read or written while
  the service runs, except at startup and at a graceful stop.

**Loading at startup (User Stories 1 and 2)**

- **FR-005**: At startup, if the file exists, the service MUST read every record and pass each one
  through the same checks as a QSO received from the logger: identity, merge of resends and
  corrections, position rules and lookups. The band selection applies only to what the page sees
  (FR-026).
- **FR-006**: A record that already carries a position MUST keep it with its recorded precision
  and source (HR-7). A position the service derived in an earlier run (marked with the
  application's position source and precision fields) MUST be restored as derived, not treated as
  sent by a logger, and MUST NOT trigger a new lookup.
- **FR-007**: A record with no usable position MUST be placed with the same lookups, order and
  rejection rules as a QSO from the logger (callbook providers, then country data; placeholders
  and outside-entity positions discarded, HR-7). Restored records MUST share the same lookup
  limits as live QSOs.
- **FR-008**: The page and the logger listener MUST be available while restored records are still
  being looked up. Restored QSOs MUST reach the map as soon as each is placed, exactly like live
  QSOs.
- **FR-009**: Restored QSOs MUST keep their original QSO time for ordering, so the map, the recent
  QSOs box and the page's count limit see them in their real order, not in load order.
- **FR-010**: Malformed records MUST be logged and skipped without stopping the load. At the end
  of the load the service MUST log the number of records loaded and skipped.
- **FR-011**: In a normal run, if the file exists but cannot be read (I/O error, or a non-empty file
  with neither an ADIF header nor an ADIF record; an empty file is an empty history), the service MUST move it aside under a name that keeps the original
  name and adds the UTC time of the move (HR-1), MUST log the reason and the new name, and MUST
  start with an empty history. If the file cannot be moved, the service MUST report both reasons
  and MUST NOT start. A file with some malformed records is not unreadable (FR-010).

**Saving at a graceful stop**

- **FR-012**: At a graceful stop (Ctrl+C or the service manager's stop signal) the service MUST
  write every QSO in its history to the file, in QSO time order, as ADIF records with an ADIF
  header naming the program and the time of the save in UTC (HR-1).
- **FR-013**: A saved record MUST contain every field of the QSO as received or loaded, plus the
  derived fields the service added (position, precision, source, rejected-position note,
  lookup-failed note, station position). Values supplied by the logger or present in the loaded file
  MUST NOT be overwritten (HR-11).
- **FR-014**: The save MUST replace the file as a whole: an interrupted or failed save MUST leave
  the previous file intact, never a truncated or mixed one. A failed save MUST be logged.
- **FR-015**: A save MUST complete within the time the service manager allows for a stop, for a
  history of 10 000 QSOs.

**Enrich-only mode (User Story 3)**

- **FR-016**: A command-line option MUST start the program in enrich-only mode. The option MUST
  accept an optional file path: with a path, that file is completed in place; without one, the
  configured history file (FR-002) is. The program loads the file
  as in FR-005 to FR-010 (FR-018 covers an unreadable file), waits until every record has been
  processed, saves as in FR-012 to FR-014 and exits.
- **FR-017**: In enrich-only mode the program MUST NOT listen for the logger and MUST NOT serve the
  page.
- **FR-018**: In enrich-only mode a missing or unreadable file MUST be reported and the program
  MUST exit with a failure status without writing anything. A completed run MUST exit with a
  success status.
- **FR-019**: In enrich-only mode, an interruption (Ctrl+C) MUST save what was completed so far,
  then exit.
- **FR-020**: Enrich-only mode MUST report its progress in the log: records loaded, records placed
  from the record itself, records placed by lookup, records with a lookup-failed note, records
  left without a position.

**Rejected positions**

- **FR-028**: In a normal run, a restored record that carries a rejected-position note MUST NOT be
  looked up again: it keeps the note and its fallback position (FR-006). In enrich-only mode it
  MUST be looked up again; a new accepted position replaces only the derived fields, and a new
  rejection updates the note.

- **FR-029**: When every callbook lookup for a QSO fails without an answer (network error,
  timeout, used-up quota, refused credentials), as opposed to an answer with no usable position,
  the QSO MUST be placed with the country fallback as today and MUST carry a lookup-failed note,
  saved with it. At the next start, in both modes, a restored record with that note MUST be looked
  up again despite FR-006; a successful lookup MUST replace the fallback and clear the note, and a
  new failure MUST keep it.

**Records on bands the service does not consider**

- **FR-026**: A file record on a band the service does not consider, or with no band, MUST be kept
  in the file and MUST NOT be sent to the page. In a normal run it MUST be left untouched: no
  lookup, and it MUST be saved back exactly as it was read. In enrich-only mode it MUST be completed
  like any other record (FR-006, FR-007) and saved back with its derived fields (FR-013); its
  lookups count against the callbook quotas.
- **FR-027**: A QSO from the logger on such a band MUST still be dropped on arrival, as today
  (feature 003), and a correction that moves a QSO onto such a band MUST still remove it.

**Unchanged**

**History size and the page's count limit**

- **FR-023**: The service MUST NOT limit the number of QSOs in its history: every QSO it loads or
  receives MUST be kept until it is removed (feature 003) or the history is reset (FR-003). The
  `history_size` setting MUST be removed from the configuration file, the environment and the
  command line; a configuration that still sets it MUST be reported with a clear message.
- **FR-024**: The page's count menu MUST offer 10, 20, 50, 100, 200, 500 and 1000 QSOs, in that
  order, and no "All" choice. The page MUST never receive or show more than the 1000 newest QSOs
  of the selected bands.
- **FR-025**: A saved "all" choice in the page MUST fall back to 1000 and MUST be saved again as
  1000 (constitution IV). A request for every QSO from an older page MUST be answered with the
  1000 newest.

**Unchanged**

- **FR-021**: With no history file present and the default settings, the service MUST behave as
  today until its first graceful stop, except for FR-023 to FR-025.
- **FR-022**: Apart from the count choices of FR-024 and FR-025, the protocol between the service
  and the page MUST NOT change.

### Key Entities

- **History file**: one ADIF file holding the service's QSO history between runs — a header, then
  one record per QSO. It is the only persistent state the feature adds.
- **Restored QSO**: a QSO read from the history file at startup. It goes through the same checks as
  a QSO from the logger and is then indistinguishable from one on the map.
- **Derived fields**: the fields the service adds to a record (position with precision and source,
  station position, rejected-position note, lookup-failed note), all with the `APP_LIVEQSOMAP_`
  prefix except the standard position fields they accompany (HR-7, HR-11). They let the next start
  restore a position without a new lookup.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After a graceful restart, 100% of the QSOs on the map before the stop are back on the
  map, at the same positions, with the same precision shown.
- **SC-002**: After a graceful restart, zero callbook lookups are made for QSOs that already had a
  position before the stop.
- **SC-003**: With a history of 10 000 QSOs, the page is available within 5 seconds of startup,
  and the save at a graceful stop finishes within 5 seconds.
- **SC-007**: With 50 000 QSOs in the history, every snapshot the page receives holds at most 1000
  QSOs, and answering a count change takes under 100 ms.
- **SC-004**: Starting from an empty history needs exactly one step between the stop and the
  start: deleting one file.
- **SC-005**: After an enrich-only run on a file whose records lack positions, every record either
  has a position with its precision and source, or carries the rejected-position or lookup-failed
  note; no field
  present in the input is changed.
- **SC-006**: An interrupted or failed save never leaves an unreadable file: in every tested
  failure, the file on disk is either the previous one or the new one, complete.

## Assumptions

- **Only graceful stops save**, as requested. QSOs received after the last graceful stop are lost
  on a crash or power loss; the logger can resend them. A periodic save is out of scope.
- **Default file name**: `qso-history.adi`, next to the downloaded country data.
- **One service per file**: two instances sharing the same file are not supported; the last one to
  stop wins.
- **The file is the service's own store.** An operator may place or edit it while the service is
  stopped; edits made while it runs are overwritten at the next graceful stop.
- **Enrich-only mode** works on the file named with the option, or on the configured history file
  (FR-016). It uses the same callbook credentials and lookup timeout as the
  service, completes records on every band (FR-026), and its lookups count against the callbook
  quotas.
- **Records are processed in file order**; the band selection and the merge rules are the ones in
  the configuration of the run that loads them.
- **The contest score, page settings and lookup results not attached to a QSO are not saved.**
- **Memory grows with the log.** Without a size limit, memory use grows with the number of QSOs;
  a contest log (tens of thousands of QSOs at most) fits comfortably. Deleting the file and
  restarting is the way to shrink it.

## Follow-up (out of scope)

- A periodic or on-change save to limit losses on a crash.
- Importing an ADIF file while the service runs (for example from the page).
- Keeping several history files (one per contest) and switching between them.
