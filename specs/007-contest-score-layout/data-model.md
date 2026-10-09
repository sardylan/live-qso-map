# Data Model: Contest Score and Page Layout

**Feature**: `007-contest-score-layout` | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Server (Rust)

### `ContestScore` (new, `src/ingest/score.rs`)

`struct ContestScore { score: u64, qsos: u64, mults: u64 }`

`score::parse(xml: &str) -> Result<ContestScore>` reads a `dynamicresults` document (research R1).

| Field | Source | Rule |
|-------|--------|------|
| `score` | `<score>`, a child of the root | required, a non-negative integer |
| `qsos` | `<breakdown><qso band="total" mode="ALL">` | required, a non-negative integer |
| `mults` | the sum of every `<breakdown><mult band="total" mode="ALL" type="…">` | 0 when there is none; each must be a non-negative integer |

- Attribute values are compared ignoring case. Every other entry is ignored.
- Errors are `Error::QarTest { element: "score" | "qso" | "mult", message }`. Examples:
  - "missing or invalid";
  - "the multiplier total overflows".

### `QarTestItem` (new, `src/ingest/qartest.rs`)

`enum QarTestItem { Qso(QarTestMessage), Score(ContestScore) }`

`qartest::parse` now returns `Result<Option<QarTestItem>>`:
- `contactinfo` and `contactreplace` give `Qso`;
- `dynamicresults` gives `Score`;
- `contactdelete` gives `None`.

### `Ingested` (new, `src/ingest/mod.rs`)

`struct Ingested { events: Vec<QsoEvent>, score: Option<ContestScore> }`

This is the return value of `events_from_datagram`. In `run`:
- `events` take today's path: band filter, resend gate, channel 1;
- `score` goes straight to `Pipeline::publish_score`, with no band filter and no gate (R2).

### `ScoreState` (new, `src/pipeline.rs`)

`struct ScoreState { score: ContestScore, received: Instant, received_at: String }`

- `received` is the monotonic receive time, for `age_s`.
- `received_at` is the UTC receive time in ISO 8601 with seconds and a `Z` suffix.

There is one value per service: the latest one received. It is lost on restart.

### `Pipeline` (changed)

| Member | Change |
|--------|--------|
| `score: Arc<Mutex<Option<Arc<ScoreState>>>>` | new; starts as `None` |
| `publish_score(&self, score: ContestScore)` | stores a fresh `ScoreState`, then broadcasts `HistoryChange::Score` while holding the score lock |
| `latest_score(&self) -> Option<Arc<ScoreState>>` | new; read by sessions on connect and after a lag |

A poisoned lock is recovered, as for the history.

### `HistoryChange` (changed)

The new variant `Score(Arc<ScoreState>)`. The doc comment now says "a change pages must hear about".

### `ServerMessage` (changed, `src/web/protocol.rs`)

The new variant `Score { score: u64, qsos: u64, mults: u64, received_at: String, age_s: u64 }`.
It is built from a `ScoreState` at send time, which computes `age_s` (see the contract).

### `Session` (changed)

| Method | Messages sent |
|--------|---------------|
| `on_connect(history, score, …)` | `hello`, `snapshot`, then `score` when one is stored |
| `on_change(Score)` | `score` |
| `on_lagged(history, score)` | `snapshot`, then `score` when one is stored |

## Page (browser)

### Score state (in `app.js`)

`scoreState = null | { score, qsos, mults, receivedAt, localReceived }`

- On `hello`, it is reset to `null`, so a restarted service shows `Score —`.
- On `score`, it is set, with `localReceived = Date.now() - age_s × 1000`.
- A 5 s interval re-renders the line, so the stale look follows the clock.

### `score.js` (new, pure, tested with `node --test web/`)

| Function | Result |
|----------|--------|
| `groupDigits(n)` | `1234567` → `"1.234.567"`; `0` → `"0"`; a non-integer or negative value → `"?"` |
| `scoreText(s)` | `null` → `"Score —"`; otherwise `"<score> pts · <qsos> QSO · <mults> mults"`, every number grouped |
| `isStale(localReceived, now, limitMs = 300_000)` | `now - localReceived > limitMs` |
| `updatedTitle(receivedAt)` | `"2026-10-08T15:04:29Z"` → `"Last updated 15:04Z"`; `""` when it is unreadable |

### Viewer settings

There are no new storage keys. The Last QSOs checkbox in the settings panel uses the existing
`lqm.recent` key (feature 004), so a viewer's choice survives the move (Principle IV).

### Page elements

| Element | Corner | Notes |
|---------|--------|-------|
| `.maplibregl-ctrl-top-left` | top left | zoom, Globe/Flat, Auto-zoom; `RecentControl` removed |
| `#settings` (was `.limit-control`) | top right | Map last N; Last QSOs checkbox `#recent-shown`; Box last N; Newest |
| `#legend` | bottom right | unchanged |
| `#corner-bl` | bottom left | column: `#score`, then `#recent` |
| `#score` | in `#corner-bl` | one line; `.stale` class greys it; `title` holds the last update |

`PANELS` (auto-zoom) gains `#score`, and `.limit-control` becomes `#settings`.
