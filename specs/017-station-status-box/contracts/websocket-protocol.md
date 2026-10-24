# Contract: WebSocket Protocol (delta to feature 014)

- **Protocol version**: **4**, unchanged. This feature adds one server message type; nothing that
  exists changes (`docs/http-api.md` API-4, research R7).
- Pages built for protocol 4 already ignore message types they don't know.

## New server message: `station_status`

```json
{
  "type": "station_status",
  "band": "40m",
  "operator": "IK0ABC",
  "qso_at": "2026-10-08T15:04:29Z",
  "age_s": 42
}
```

With an empty history:

```json
{ "type": "station_status", "band": null, "operator": null, "qso_at": null, "age_s": null }
```

| Field | Type | Meaning |
|-------|------|---------|
| `band` | string or null | the ADIF band of the most recent QSO |
| `operator` | string or null | its `OPERATOR`, else its `STATION_CALLSIGN`, else the configured station callsign |
| `qso_at` | string, UTC (API-4), or null | the date and time of the most recent QSO |
| `age_s` | integer ≥ 0 or null | seconds between `qso_at` and the service's clock when this message was sent |

"Most recent" is by QSO date and time, not by arrival (research R2). The page's band choice and
window size do not change the message.

## When the server sends it

| Moment | Sent |
|--------|------|
| A page connects or reconnects | after `hello` and `snapshot`, before `score` |
| The service forwards an `upsert` or a `remove` | right after it, to every connected page |
| A page fell behind the broadcast (lag) | after the resent `snapshot` |

It is sent even when nothing changed and when the history is empty.

## Page behaviour

- On `hello`: the page forgets the status it showed and shows `— · —`.
- On `station_status`:
  - it shows `<band> · <operator>`, with `—` for a null value;
  - it sets the tooltip to `Last QSO HH:MMZ` from `qso_at`, or empty when null;
  - it records `Date.now() - age_s × 1000` when `age_s` is not null.
- Every 5 s: the line is greyed out when that recorded time is more than 900 s old, and back to
  normal otherwise.
- The page MUST NOT compare `qso_at` with its own clock.

## Unchanged

`hello`, `snapshot`, `upsert`, `remove`, `score` and `set_limit` are as in features 001 to 014.
