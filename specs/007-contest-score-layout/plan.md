# Implementation Plan: Contest Score and Page Layout

**Branch**: `007-contest-score-layout` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/007-contest-score-layout/spec.md`

## Summary

**Score**: QARTest's once-a-minute `dynamicresults` message, ignored since feature 001, is read by
a small dedicated XML reader (research R1). It takes:
- the score;
- the total QSOs;
- the sum of every multiplier total.

The score skips the band filter and the resend gate (R2). The pipeline keeps the latest one in
memory and fans it out on the existing broadcast channel (R3). Pages get a new `score` message:
- on connect;
- on every update;
- after a lag.

The protocol stays 3 (R7).

**Staleness**: the message carries `received_at` for the tooltip, and `age_s`, so the page can
judge staleness on its own clock. This holds even when the browser's clock is off (R4).

**Page**: a pure `score.js` formats `1.234.567 pts · 1.523 QSO · 183 mults` (R5). The layout
changes (R6):
- the Last QSOs button leaves the map controls and becomes a checkbox in the top-right settings
  panel, using the same storage key;
- a bottom-left column holds the score line above the box;
- auto-zoom keeps clear of the score line.

## Technical Context

**Language/Version**: Rust (edition 2024, stable toolchain; developed on 1.98); browser ES modules
**Primary Dependencies**: none new (existing `quick-xml` reads the score message)
**Storage**: in memory, the latest score only, lost on restart. No new browser keys: the box
checkbox keeps `lqm.recent`.
**Testing**: `cargo test` and `node --test web/`
**Target Platform**: unchanged; the page from 1024 px wide up (spec FR-014)
**Project Type**: single binary (backend plus embedded frontend)
**Performance Goals**:
- a score reaches every page within 2 s (SC-001);
- a new page gets it within 1 s (SC-002);
- the stale look is right within 10 s (SC-003).

**Constraints**:
- Principle IV: the moved box switch keeps its stored choice;
- interface rules: additive message, `snake_case`, UTC with `Z`;
- HR-1: UTC tooltip;
- HR-10: contest-agnostic multipliers;
- plain-text rendering.

**Scale/Scope**: one score per minute per logger, one held value

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests come first.
  - **Rust**:
    - `score::parse`:
      - both reference packets (90/6/10 and 132/7/11);
      - other multiplier types, such as one type only or three types;
      - no multipliers (0);
      - attribute case;
      - per-band and non-`ALL` entries ignored;
      - missing, empty, non-numeric or negative score and QSO total;
      - multiplier overflow;
      - mutated packets never panic.
    - `qartest::parse` gives `Score` for `dynamicresults` and still `None` for `contactdelete`.
    - `events_from_datagram` returns the score with no events.
    - `ingest::run` scores bypass the band filter and the resend gate (an identical repeat is
      still published).
    - `Pipeline::publish_score` stores and broadcasts.
    - Sessions send `score` on connect, on change and after a lag, and never when none is held.
    - The `score` JSON, with `age_s` computed from the send time.
    - End to end over a real socket: a UDP score reaches an open page, a page opened later gets
      it, and a malformed score changes nothing.
  - **JS**:
    - `groupDigits` (0, 999, 1000, 1234567, 12345678, bad input);
    - `scoreText` with and without a score;
    - `isStale` at, below and above 300 s;
    - `updatedTitle` with good and bad times.
- [x] **II. Rust Safety & Quality**:
  - no `unsafe`;
  - parsing errors are `Error::QarTest`, logged and skipped;
  - numbers use `u64` with checked sums;
  - poisoned locks are recovered;
  - fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new crates;
  - one new message and one held value, on the existing broadcast channel;
  - no new configuration setting (the 5-minute threshold is fixed by the spec).
- [x] **IV. Persistent Viewer Settings**: no new viewer setting. The moved Last QSOs switch keeps
  `lqm.recent`, so it survives restarts and the upgrade. The score line has no setting (FR-006).
- [x] **Domain rules**:
  - HR-1: `received_at` and the tooltip are UTC;
  - HR-10: multipliers are summed whatever their type, for any contest.
- [x] **Interface rules**:
  - `score` is a JSON text frame with `type` (API-2);
  - its fields are `snake_case`, and `received_at` is UTC ISO 8601 with `Z` (API-4);
  - the message is additive, so the protocol stays 3.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/007-contest-score-layout/
├── plan.md
├── research.md          # R1–R7
├── data-model.md        # ContestScore, QarTestItem, Ingested, ScoreState, score.js, page elements
├── quickstart.md
├── contracts/
│   └── websocket-protocol.md    # new `score` message; protocol stays 3
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (changes)

```text
src/
├── ingest/score.rs      # NEW: ContestScore, parse() (quick-xml walk with attributes)   [tests]
├── ingest/qartest.rs    # QarTestItem; dynamicresults → Score                            [tests]
├── ingest/mod.rs        # pub mod score; Ingested; run(): score → pipeline, no filter/gate [tests]
├── pipeline.rs          # ScoreState, publish_score, latest_score, HistoryChange::Score  [tests]
└── web/
    ├── protocol.rs      # ServerMessage::Score, built from ScoreState with age_s          [tests]
    ├── session.rs       # score on connect, on change, after lag                         [tests]
    ├── mod.rs           # serve score.js
    └── assets/
        ├── index.html   # #settings panel with #recent-shown; #corner-bl with #score and #recent
        ├── app.css      # #corner-bl column, .score, .score.stale; .recent loses absolute position
        ├── app.js       # glue: score message, 5 s stale tick, checkbox, PANELS, no RecentControl
        └── score.js     # NEW: groupDigits, scoreText, isStale, updatedTitle             [node tests]

web/score.test.js        # NEW
tests/qartest_replay.rs  # the capture's score packets now publish a score
tests/ws_protocol.rs     # score over a real WebSocket: live, on connect, malformed ignored
README.md                # the score line and the new layout
specs/001-live-qso-map/spec.md, specs/004-qth-autozoom/spec.md   # notes pointing to 007
```

**Structure Decision**: unchanged.
- The score reader is a small pure module next to the QARTest parser.
- Page decisions stay in pure modules, and `app.js` remains glue.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

No violations.

## Post-Design Constitution Re-check

- [x] **I**: every rule in R1–R6 maps to a planned test that is written first.
- [x] **II**: malformed scores are errors that are logged and skipped, with checked arithmetic and
  no panics.
- [x] **III**: no new crates, settings or channels. The only new reader exists because `flatten`
  can't see attributes (R1).
- [x] **IV**: the only moved setting keeps its storage key, with tested fallbacks (feature 004).
- [x] **Domain and interface rules**: respected (HR-1, HR-10, API-2, API-4).
- [x] **Security**:
  - score values reach the DOM only through `textContent` and `title`;
  - XML entities in the message can't inject markup;
  - only numbers are kept.
