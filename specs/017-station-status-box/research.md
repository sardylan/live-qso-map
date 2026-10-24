# Research: Station Status Box

**Feature**: `017-station-status-box` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

The spec has no NEEDS CLARIFICATION items left (one settled on 2026-10-24). This file records the
design decisions the plan needs.

## R1. Where the operator comes from (FR-002, FR-010)

- **Decision**: the `OPERATOR` field of the QSO. QARTest already writes its `<operator>` element
  there (`src/ingest/qartest.rs`); ADIF datagrams carry `OPERATOR` directly. Nothing new is parsed.
- **Rationale**: the reference capture (`specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt`)
  has no operator-change message, and `dynamicresults` has only `<call>` and `ops='MULTI-OP'`.
  OPON shows up only as the operator stamped on the QSOs logged after it.
- **Alternatives**: read `<call>` from the score message as the operator. It is the station
  callsign, not the person at the radio.

## R2. Who computes the status (FR-002, FR-004, FR-005, FR-009)

- **Decision**: the service computes it from the history and sends it as a new `station_status`
  WebSocket message (contract in [contracts/websocket-protocol.md](contracts/websocket-protocol.md)).
  The most recent QSO is `History::latest(Some(1))`, the last one by sort key (QSO date and time,
  `enrich::sort_timestamp`).
- **Rationale**:
  - the page cannot work it out from what it gets: its `snapshot` holds only its own window, filtered
    by the page's band choice (`History::latest_matching`), so the newest QSO may be missing (FR-009);
  - staleness must not use the browser's clock (spec edge cases). The service knows both the QSO time
    and its own clock, so it can send an age, as the score does (feature 007, R4);
  - after a restart the loaded history is already in `History` (FR-005), with nothing extra to do.
- **Alternatives**:
  - derive it in the page from `snapshot` and `upsert`: wrong under a band filter, and needs the
    browser's clock;
  - store a separate "current status" in `Pipeline` updated on publish: a second copy of a value the
    history already holds, which a correction or remove must keep in step.

## R3. When the status is sent (FR-003, FR-004)

- **Decision**: `Session` sends `station_status`:
  - on connect, after `hello` and `snapshot` (and before `score`);
  - after every `upsert` and `remove` it forwards;
  - after the resent `snapshot` on lag.

  It is always sent, also when unchanged, and also with an empty history (all fields `null`).
  `drive` locks the history for change events too, to build it.
- **Rationale**: recomputing from the history on every change covers new QSOs, corrections of the
  latest QSO, corrections of old QSOs (status unchanged) and removes, with one rule. `latest(Some(1))`
  is one map lookup. One status message per QSO per page is negligible next to the `upsert` itself.
- **Alternatives**:
  - send only when the status changes: needs per-session memory of the last one sent, for no
    visible gain;
  - a new `HistoryChange::Status` broadcast: one more variant on the channel that every session
    already turns into messages; it saves a history lock per page per QSO, which does not matter at
    contest rates.

## R4. The operator shown (FR-007, clarification 2026-10-24)

- **Decision**: the first non-empty of: the QSO's `OPERATOR`, its `STATION_CALLSIGN`, the configured
  `[station] callsign` (the same fallback `ingest` gives `Identity::from_record`); `null` when none.
  The value is trimmed and upper-cased.
- **Rationale**: a single-op station never uses OPON, so its QSOs have an empty operator. Showing
  the station callsign there is what the owner chose.
- **Alternatives**: a dash, or hiding the line (rejected in the clarification).

## R5. Staleness (FR-008, SC-004)

- **Decision**:
  - the message carries `qso_at` (UTC, the QSO's date and time) and `age_s`, the whole seconds
    between `qso_at` and the service's clock when the message is built, 0 when `qso_at` is in the
    future;
  - the page records `Date.now() - age_s × 1000` and greys the box when that is more than 900 s old,
    checked on the 5 s tick the score line already uses;
  - the tooltip reads `Last QSO HH:MMZ`.
- **Rationale**: same mechanism as the score (feature 007, R4), so the page never compares a UTC
  time with its own clock. 15 minutes is the spec's fixed threshold.
- **Alternatives**: the page compares `qso_at` with its clock: wrong on a kiosk with a bad clock.

## R6. Page placement and shape (FR-001, FR-011)

- **Decision**:
  - a `<div id="station" class="score station">` in the bottom-left corner stack, directly above
    the score line, so it takes the score line's theme colours, size and staleness style;
  - it reads `40m · IK0ABC`, and `— · —` when a value is missing; `aria-live="polite"`;
  - its text helpers go in a new pure module `src/web/assets/station.js`, with `node --test` tests in
    `web/station.test.js`;
  - `isStale` from `score.js` is reused with its `limitMs` argument;
  - a text change calls `autozoom.change()`, as the score line does.
- **Rationale**:
  - the corner stack already keeps clear of the controls and is counted by auto-zoom;
  - one line keeps the corner as short as possible; band first, as the spec lists it;
  - `id="status"` is taken by the reconnect banner, hence `station`.
- **Spec note**: FR-001 said "two lines"; it now says "two values", shown on one line.
- **Alternatives**: a separate top-right box. It would compete with the legend and the settings
  button, and auto-zoom would need another panel.

## R7. Protocol version (docs/http-api.md)

- **Decision**: stays **4**. One server message type is added and nothing existing changes; pages
  ignore types they do not know. Same choice as feature 007.

## R8. Browser test (LL-007)

- **Decision**: the release browser steps (`tests/browser/run.mjs`) gain a `station` step:
  - on a fresh history, a QSO timed 20 minutes ago greys the line at once, since its `age_s` is
    already past 900 s. Waiting 15 minutes is not an option. The test clock that steps 10 and 11
    set on the page is no matter: the page measures age from `age_s` against its own clock;
  - two current QSOs with different bands and operators: the line shows the second, not greyed;
  - the line overlaps neither the score line nor the last-QSO box (FR-011).

  Per LL-007 the new step runs in a manual rehearsal pipeline before the feature is merged.
