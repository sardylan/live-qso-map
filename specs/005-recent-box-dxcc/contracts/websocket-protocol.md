# Contract: WebSocket Protocol (delta to feature 004)

- **Protocol version**: **3**, unchanged. The change only adds a field (`docs/http-api.md`
  API-4).
- **QSO object**: in `snapshot.qsos[]` and `upsert.qso`, it gains `entity`.

```json
{
  "id": "L|IS0MZK|3809734F4",
  "call": "JA1XYZ",
  "band": "20m",
  "datetime": "2026-10-09T15:04:00Z",
  "entity": { "prefix": "JA", "name": "Japan" }
}
```

- `entity` is `null` when the callsign matches no cty.dat entity or no country data is loaded.
- `prefix` is the cty.dat primary prefix without the WAE `*` (Sicily is `IT9`).
- `name` is the cty.dat entity name.
- Every other field is unchanged.
