# Contract: WebSocket Protocol (delta to features 004 and 005)

- **Protocol version**: **3**, unchanged. Both changes add fields (`docs/http-api.md` API-4).
- **`set_limit`**: unchanged. The page now requests `max(map Last N, 20)`, or `null` for All
  (research R1).

## `hello`: new field `recent`

```json
{
  "type": "hello",
  "protocol": 3,
  "recent": { "columns": ["call", "band", "time", "cqzone", "country"], "header": true }
}
```

`columns` is the operator's ordered list (default shown); `header` shows the header row.

## QSO object: `entity` gains `cq_zone`

```json
"entity": { "prefix": "UA9", "name": "Asiatic Russia", "cq_zone": 18 }
```

- `cq_zone` is the cty.dat CQ zone for this callsign, with per-callsign overrides applied.
- `entity` stays `null` when the callsign matches nothing.
