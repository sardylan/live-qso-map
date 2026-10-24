# Contract: Configuration (delta to feature 015)

## `[map]` gains `globe_border_deg`

```toml
[map]
# Auto-zoom keeps the globe only while every QSO marker can sit at least this many degrees of arc
# inside the globe's visible horizon; otherwise it shows the flat map. 0 to 45.
# globe_border_deg = 10
```

| Source | Name |
|--------|------|
| CLI | `--globe-border-deg <DEG>` |
| Environment | `LQM_GLOBE_BORDER_DEG` |
| File | `[map] globe_border_deg` |
| Default | `10` |

Errors (service refuses to start, `docs/rust-guidelines.md` config errors):

- `map.globe_border_deg: cannot parse LQM_GLOBE_BORDER_DEG=<raw>`
- `map.globe_border_deg: <value> is not between 0 and 45`

`--save-config` writes `globe_border_deg` in `[map]` alongside `fallback_color`, `recent_columns`
and `recent_header`.
