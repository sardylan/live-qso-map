# Contract: WebSocket Protocol (delta to feature 006)

- **Protocol version**: **3**, unchanged. This feature adds one server message type; nothing that
  exists changes (`docs/http-api.md` API-4, research R7).
- Pages built for protocol 3 already ignore message types they don't know.

## New server message: `score`

```json
{
  "type": "score",
  "score": 132,
  "qsos": 7,
  "mults": 11,
  "received_at": "2026-10-08T15:04:29Z",
  "age_s": 0
}
```

| Field | Type | Meaning |
|-------|------|---------|
| `score` | integer ≥ 0 | the contest score the logger reports (`<score>`) |
| `qsos` | integer ≥ 0 | the logger's total QSO count |
| `mults` | integer ≥ 0 | the sum of the logger's multiplier totals of every type |
| `received_at` | string, UTC (API-4) | when the service received the score |
| `age_s` | integer ≥ 0 | seconds between `received_at` and the moment this message was sent |

`age_s` lets the page judge staleness on its own clock (research R4). The page MUST NOT compare
`received_at` with its own clock.

## When the server sends it

| Moment | Sent |
|--------|------|
| A page connects or reconnects | after `hello` and `snapshot`, when the service holds a score |
| The service receives a valid score message | to every connected page, within 2 s |
| A page fell behind the broadcast (lag) | after the resent `snapshot`, when the service holds a score |

With no score held, nothing is sent. The page shows `Score —` from `hello` on.

## Page behaviour

- On `hello`: the page forgets any score it showed, because the service may have restarted.
- On `score`:
  - it shows `<score> pts · <qsos> QSO · <mults> mults`, with dot thousands separators;
  - it sets the tooltip to `Last updated HH:MMZ`;
  - it records `Date.now() - age_s × 1000`.
- Every 5 s: the line is greyed out when that recorded time is more than 300 s old, and back to
  normal otherwise.

## Unchanged

`hello`, `snapshot`, `upsert`, `remove` and `set_limit` are as in features 001 to 006.
