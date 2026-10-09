# Contract: Configuration (delta to features 001 and 003)

| TOML key | CLI flag | Env var | Default | Notes |
|----------|----------|---------|---------|-------|
| `map.recent_columns` | `--recent-columns` (comma-separated, repeatable) | `LQM_RECENT_COLUMNS` (comma-separated) | `["call", "band", "time", "cqzone", "country"]` | keywords: `call`, `band`, `mode`, `date`, `time`, `cqzone`, `country`; order is the display order; a higher layer replaces the whole list |
| `map.recent_header` | `--recent-header <true\|false>` | `LQM_RECENT_HEADER` | `true` | shows the box header row |

## Validation (startup)

The service exits with a non-zero status when:

- a column keyword is unknown:
  `invalid setting map.recent_columns: unknown column 'freq' (call, band, mode, date, time, cqzone, country)`;
- a column is listed twice: `invalid setting map.recent_columns: column 'call' is listed twice`;
- no column is left: `invalid setting map.recent_columns: no columns`;
- the header value is neither `true` nor `false`: `invalid setting map.recent_header: …`.

## Example

```toml
[map]
recent_columns = ["call", "band", "mode", "date", "time", "cqzone", "country"]
recent_header = true
```
