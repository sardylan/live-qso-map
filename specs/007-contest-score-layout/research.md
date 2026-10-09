# Research: Contest Score and Page Layout

**Feature**: `007-contest-score-layout` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items. The owner settled five decisions while the spec was written
and clarified, and confirmed the defaults. The spec left two points to the plan:
- the message that carries the score;
- whether the protocol version changes.

## R1. Reading the score message (FR-001, FR-003)

- **Decision**: a dedicated reader in `src/ingest/score.rs` walks the `dynamicresults` document
  with `quick-xml` and reads the element attributes.
  - The score is the text of `<score>`, a direct child of the root: a non-negative integer, required.
  - The QSO count is the `<qso>` entry inside `<breakdown>` whose attributes are `band="total"` and
    `mode="ALL"` (case-insensitive). It is required.
  - The multipliers are the sum of every `<mult>` entry in `<breakdown>` with `band="total"` and
    `mode="ALL"`, whatever its `type`. No such entry means 0 multipliers, because some contests
    have none.
  - `<point>` and the per-band entries are not read (out of scope).
  - A missing or non-numeric score or QSO total, a negative number, or a sum that overflows `u64`
    is `Error::QarTest { element: "score" | "qso" | "mult", .. }`. The message is then logged and
    skipped (FR-003).
- **Rationale**: the shared `xml::flatten` keeps one text per child name and drops attributes, so
  the 8 `<mult>` entries of a CQ WW message would overwrite each other. A small walk is simpler
  than teaching `flatten` about attributes and repeated elements for one caller.
- **Checked against the reference capture** (`specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt`):
  - packet 4: score 90, QSOs 6, mults 6 + 4 = 10;
  - packet 10: score 132, QSOs 7, mults 6 + 5 = 11.
- **Alternatives**:
  - extend `xml::flatten` with attributes and repeated elements. Every other caller would carry
    the extra shape for nothing;
  - sum the per-band entries. This double-counts multipliers that count once per contest, and the
    logger already gives the totals.

## R2. The score's path through the service (FR-002, FR-004)

- **Decision**:
  - `qartest::parse` returns `Option<QarTestItem>`, where `enum QarTestItem { Qso(QarTestMessage),
    Score(ContestScore) }`. `contactdelete` still gives `None`.
  - `events_from_datagram` returns `Ingested { events: Vec<QsoEvent>, score: Option<ContestScore> }`.
  - `ingest::run` hands a score straight to `Pipeline::publish_score`.
- **Not applied to scores**:
  - **the band filter** (feature 003): the score is the logger's own total for the whole contest;
  - **the resend gate**: QARTest repeats an unchanged score every minute, and each repeat must
    still refresh the "last updated" time, or a quiet but live station would turn grey after
    5 minutes.
- **Rationale**: scores never become QSO events, so the history, the map and the box are untouched
  (FR-004). Only QARTest sends them. ADIF datagrams never carry a score.
- **Alternatives**: a second parse of every datagram just for scores. That means two
  format detections per datagram and two places that know QARTest roots.

## R3. Keeping and broadcasting the latest score (FR-002)

- **Decision**:
  - `Pipeline` gains `score: Arc<Mutex<Option<Arc<ScoreState>>>>`.
  - `publish_score(score)`:
    1. stamps it with the time received (`Instant` and UTC);
    2. stores it;
    3. broadcasts `HistoryChange::Score(Arc<ScoreState>)` on the existing channel 2, while still
       holding the score lock.
  - The enum keeps its name. Its doc comment becomes "a change pages must hear about".
  - Session (`src/web/session.rs`):
    - `on_connect` sends `hello`, `snapshot`, then a `score` message when one is stored;
    - `on_change(Score)` sends a `score` message;
    - `on_lagged` resends the snapshot and the stored score.
- **Rationale**:
  - one channel keeps the order of events, and lag recovery already exists;
  - pages subscribe before they read the stored score, as for the history, so no update is lost
    between the two (feature 001 R4).
- **Alternatives**:
  - a separate `watch` channel for the score. Each session would need a third `select!` branch, and
    a second lag path;
  - store the score inside `History`. It is not a QSO.

## R4. Staleness without trusting the browser's clock (FR-007, SC-003)

- **Decision**: the `score` message carries two time fields:
  - `received_at`: UTC with seconds (API-4), for the tooltip;
  - `age_s`: whole seconds since the service received the score, computed when the message is sent.

  The page then works out the staleness itself:
  1. it records `localReceived = Date.now() - age_s × 1000` on its own clock;
  2. it checks every 5 s whether `Date.now() - localReceived > 300 000` ms, and greys the line out
     when that is true;
  3. a new `score` message resets the check.

  The tooltip reads `Last updated HH:MMZ`, taken from `received_at`. It is always set, so it is
  there when the line is grey.
- **Rationale**:
  - the page and the service often run on different machines, and a shack PC's clock can drift by
    minutes. A drift of minutes against a 5-minute threshold would grey out live scores, or hide
    stale ones;
  - `age_s` makes the measure depend only on the elapsed time on each side;
  - a 5 s check meets SC-003 (within 10 s).
- **Alternatives**:
  - compare `received_at` with the browser clock. This is wrong under drift;
  - the server pushes a `stale` flag from a per-session timer. That adds a timer and state to
    every session, and a fourth `select!` branch.

## R5. Formatting on the page (FR-005, FR-008)

- **Decision**: a pure module `score.js` holds:
  - `groupDigits(n)`: dot thousands separators, done on the digit string, never with
    `toLocaleString`, so the browser locale can't change it;
  - `scoreText(score)`: `Score —` when there is no score, otherwise
    `1.234.567 pts · 1.523 QSO · 183 mults`;
  - `isStale(localReceived, now)`;
  - `updatedTitle(received_at)`: `Last updated 15:04Z`.

  `app.js` sets the text with `textContent` and the tooltip with `title`, never markup.
- **Rationale**: the same pure-module pattern as `recent.js`, tested with `node --test`.

## R6. The new layout (FR-009 – FR-014)

- **Top left**: the NavigationControl, ProjectionControl and AutoZoomControl stay, in that order.
  The `RecentControl` map control is removed.
- **Top right**: the existing `.limit-control` panel becomes the settings panel `#settings`. Its
  order:
  1. "Map: last N QSOs";
  2. a separator;
  3. a "Last QSOs" checkbox (`#recent-shown`), which replaces the removed button;
  4. "Box: last N" and "Newest", as today.

  The checkbox:
  - reads and writes the same `lqm.recent` key, so viewers keep their choice across the upgrade
    (Principle IV);
  - on change, re-renders the box and tells auto-zoom that the panels moved, like the old button.

  The box size and order stay usable while the box is hidden, so they can be set before showing
  it.
- **Bottom left**: a new container `#corner-bl` holds `#score`, then `#recent`, in a column.
  - It takes the box's old position: `left: 10px; bottom: 30px`, or `56px` when `body.scale-on`.
  - It also takes the box's old `max-width: 45%`.
  - `#recent` loses its own absolute positioning.
  - When the box is hidden, the score line drops to the bottom by itself, with no JavaScript
    positioning.
  - `#score` is `white-space: nowrap`. At 13 px, the longest line in SC-005 is about 280 px,
    within 45% of 1024 px (460 px).
- **Bottom right**: the legend and attribution are unchanged (FR-011).
- **Auto-zoom**: `#score` joins `PANELS` (FR-013). Its rectangle is measured like the others.
- **Rationale**: a flex column stacks the two boxes without overlap whatever their heights, and
  keeps feature 004's rule that the bottom-left stack clears the scale bar and the attribution.
- **Alternatives**:
  - position the score with JavaScript from the box's measured height. This needs re-measuring on
    every render and resize;
  - keep the Last QSOs button among the map controls. That goes against the owner's top-left/top-
    right split.

## R7. Protocol version (spec Assumptions)

- **Decision**: a new server message, `{"type": "score", …}`. The protocol stays **3**.
- **Rationale**:
  - the message is additive (API-4: only a breaking change bumps the version);
  - pages built for protocol 3 already ignore message types they don't know (`app.js` `onMessage`
    returns on anything else);
  - every page re-handshakes after a reconnect, and on `hello` the page resets the score to "none",
    so a restarted service shows `Score —` until the next broadcast.
- **Alternatives**: put the score into `hello` and repeat `hello` on each update. `hello` would then
  mean two things.
