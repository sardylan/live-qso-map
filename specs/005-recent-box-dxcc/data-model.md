# Data Model: DXCC Entity and Date in the Last-QSO Box

**Feature**: `005-recent-box-dxcc` | **Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Server (Rust)

### `QsoEntity` (new, `src/history.rs`)

| Field | Type | Rule |
|-------|------|------|
| `prefix` | `String` | cty.dat primary prefix, without the WAE `*` (HR-6) |
| `name` | `String` | cty.dat entity name |

### `EnrichedQso` (changed)

- New field: `entity: Option<QsoEntity>`.
- It is resolved by `enrich()` from the record's `CALL` with `CountryData::entity_of`.
- It is `None` when there is no callsign, no country data, or no match.

### `Lookups` (changed, `src/enrich/mod.rs`)

- New field: `country: Option<Arc<CountryData>>`.
- `Lookups::new(sources, country, config)`.

### `QsoView` (changed, `src/web/protocol.rs`)

- New field: `entity: Option<QsoEntity>`, serialised as an object or `null`.
- `PROTOCOL_VERSION` stays 3 (research R3).

### Pipeline

- `build_sources` already returns the country data (feature 004). It is now also passed to
  `Lookups`.
- `start_with_country(config, sources, country)` is the test entry point (R2).

## Frontend (pure, `src/web/assets/recent.js`)

| Function | Result |
|----------|--------|
| `shortName(name, max = 18)` | `name` when it has at most `max` characters, else its first `max − 1` characters, trimmed, followed by `…` |
| `entityLabel(entity)` | `"JA · Japan"`, or `"—"` when `entity` is `null` |
| `rowView(qso)` | `{call, band, date, time, entity, entityTitle, markerKey}`, where `entity` uses `shortName` and `entityTitle` is the full label (empty when unknown); there is no `mode` |
