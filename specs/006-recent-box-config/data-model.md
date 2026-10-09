# Data Model: Configurable Last-QSO Box

**Feature**: `006-recent-box-config` | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Server (Rust)

### `Column` (new, `src/recent_columns.rs`)

`enum Column { Call, Band, Mode, Date, Time, CqZone, Country }`

- Keywords: `call`, `band`, `mode`, `date`, `time`, `cqzone` and `country`.
- `Column::keyword()` returns the keyword.
- It serialises as the keyword string.
- `DEFAULT_COLUMNS = [Call, Band, Time, CqZone, Country]`.

`recent_columns::parse(entries: &[String]) -> Result<Vec<Column>>`:

1. Trim and lower-case each entry; blank entries are skipped.
2. An unknown keyword fails with `Error::Config { setting: "map.recent_columns", message:
   "unknown column 'freq' (call, band, mode, date, time, cqzone, country)" }`.
3. A repeated keyword fails with `… "column 'call' is listed twice"`.
4. No keyword left fails with `… "no columns"`.

### `MapConfig` (changed, `src/config.rs`)

| Field | Type | Default |
|-------|------|---------|
| `recent_columns` | `Vec<Column>` | `DEFAULT_COLUMNS` |
| `recent_header` | `bool` | `true` |

They are layered as research R2 describes. `LQM_RECENT_HEADER` and `--recent-header` accept
`true` or `false`; anything else is `Error::Config { setting: "map.recent_header", .. }`.

### `EntityInfo` and `QsoEntity` (changed)

- `CountryData::entity_info(call) -> Option<EntityInfo { prefix: &str, name: &str, cq_zone: u8 }>`
  replaces `entity_of`. `cq_zone` is the override when present, otherwise the entity's zone.
- `QsoEntity { prefix, name, cq_zone: u8 }` serialises as
  `{"prefix": "UA9", "name": "Asiatic Russia", "cq_zone": 18}`.

### `ServerMessage::Hello` (changed)

- New field: `recent: RecentView { columns: Vec<Column>, header: bool }`.
- `protocol` stays 3.

## Frontend

### Map state (`map-state.js`, changed)

```text
state = { …, limit /* server window, from the snapshot */, mapLimit /* viewer's map Last N */ }
```

| Function | Result |
|----------|--------|
| `visible(state)` | the newest `mapLimit` QSOs (all when `null`), ascending; what the map draws |
| `held(state)` | every QSO the page holds, ascending; what the box draws from |
| `setMapLimit(state, n)` | sets `mapLimit` |
| `serverLimit(mapLimit)` | `null` when `mapLimit` is `null`, else `max(mapLimit, 20)` (R1) |

### Box (`recent.js`, changed)

| Function | Result |
|----------|--------|
| `recentQsos(state, size, order)` | the newest `size` of `held(state)`: ascending for `'bottom'`, descending for `'top'` |
| `headerCells(columns)` | `[{column, text}]`; Call, Band, Mode, Date, UTC, CQ, Country |
| `rowCells(qso, columns)` | `[{column, text, title}]` (R4) |
| `onMap(qso, markers)` | the QSO's marker key is in `markers` |
| `shortName`, `entityLabel` | unchanged (feature 005) |

### Viewer preferences (`view.js` and `connection.js`, changed)

| Key (`localStorage`) | Values | Default | Helpers |
|----------------------|--------|---------|---------|
| `lqm.limit` | 10, 20, 50, 100, 200, 500, `all` | 20 | `loadLimit`/`saveLimit` (now given local storage) |
| `lqm.recent.size` | 3, 5, 10, 15, 20 | 5 | `loadRecentSize`/`saveRecentSize` |
| `lqm.recent.order` | `bottom`, `top` | `bottom` | `loadRecentOrder`/`saveRecentOrder` |

Every load helper returns the default for missing or unknown values and when the storage throws.
