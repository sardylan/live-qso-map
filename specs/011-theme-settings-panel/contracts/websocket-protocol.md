# Contract: WebSocket Protocol Changes (feature 011, protocol version 4)

It extends the protocol of features 001–007, and follows `docs/http-api.md`: JSON text frames
with `type`, `snake_case` fields, arrays where order matters.

## `hello` (server → page): new field

```json
{
  "type": "hello",
  "protocol": 4,
  "bands": [{ "band": "160m", "color": "#…" }, …],
  "band_groups": [
    { "name": "hf", "label": "HF", "bands": ["160m", "80m", "40m", "20m", "15m", "10m"] },
    { "name": "hf-contest", "label": "HF contest", "bands": ["160m", "80m", "40m", "20m", "15m", "10m"] }
  ],
  …
}
```

- **Group order:** `lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`, `uhf`, `shf`, `ehf`. A group is
  present only when at least one of its bands is considered.
- **`bands` of a group:** only considered bands, in frequency order (HR-2).
- **Labels:**

  | `name` | `label` |
  |--------|---------|
  | `lf-mf` | LF/MF |
  | `hf` | HF |
  | `hf-contest` | HF contest |
  | `warc` | WARC |
  | `vhf` | VHF |
  | `uhf` | UHF |
  | `shf` | SHF |
  | `ehf` | EHF |

- **Everything else in `hello` is unchanged.**

## `set_limit` (page → server): optional `bands`

```json
{ "type": "set_limit", "limit": 20, "bands": ["160m", "80m", "40m", "20m", "15m", "10m"] }
```

- **`bands` absent or `null`:** every QSO, as in version 3.
- **`bands` a list:** the session remembers it, and every following `snapshot` holds the newest
  `limit` QSOs whose `BAND` is in the list (case-insensitive). QSOs without a `BAND` are
  excluded.
- **Invalid `bands`** (not an array of strings, or unknown bands): unknown band names are
  ignored. A list left empty is treated like `null`. It is logged at debug level, never an
  error.
- **Unchanged:** the limit rules (allowed values, behaviour on invalid values).

## Unchanged

- **`snapshot`, `upsert`, `remove`, `score`:** same shape. `upsert` and `remove` reach every page,
  whatever its `bands`; the page filters what it draws.
- **Old pages** (sending no `bands`) keep working; they just see `protocol: 4`.
