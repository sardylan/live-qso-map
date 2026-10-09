# Research: Configurable Last-QSO Box

**Feature**: `006-recent-box-config` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items: the owner settled seven decisions while the spec was
written and clarified. The spec left one point to the plan: how a page gets enough QSOs for both
the map and the box (R1).

## R1. How the page gets enough QSOs for the map and the box (FR-009)

- **Decision**: the page asks the server for `max(map Last N, 20)`, or "All" when the map is set
  to All, with the existing `set_limit` message.
  - 20 is the largest box size, so the box always has its rows.
  - The page then splits locally:
    - the map uses the newest *map Last N*;
    - the box uses the newest *box size* of everything the page holds.
- **Rationale**:
  - there is no protocol change, and `set_limit` keeps its allowed values, since 20 and every
    larger map value are already allowed;
  - with the map at 10, the page receives 10 extra QSOs. They are small JSON objects.
- **Change on the page**: `map-state.js` keeps the server window, the snapshot's `limit`, apart
  from the map's own limit (`mapLimit`).
  - `visible(state)` uses `mapLimit`.
  - A new `held(state)` returns every QSO the page holds, sorted, for the box.
- **Alternatives**:
  - add the box sizes 3, 5 and 15 to the allowed `set_limit` values and request exactly
    `max(map, box)`. This changes the server's allowed set for little gain;
  - a second subscription for the box. This means two windows to track and more protocol.

## R2. Operator settings (FR-001–FR-003, FR-006a)

- **Decision**: two settings under `[map]`, layered like every other setting (command line, then
  environment, then TOML, then the default):

  | TOML key | CLI | Env | Default |
  |----------|-----|-----|---------|
  | `map.recent_columns` | `--recent-columns` (comma-separated or repeated) | `LQM_RECENT_COLUMNS` (comma-separated) | `["call", "band", "time", "cqzone", "country"]` |
  | `map.recent_header` | `--recent-header <true\|false>` | `LQM_RECENT_HEADER` | `true` |

- **Validation**: a pure parser, `recent_columns::parse(&[String]) -> Result<Vec<Column>>`, in a
  new `src/recent_columns.rs`.
  - It trims and lower-cases each keyword.
  - It rejects unknown keywords, an empty list (including a list of blanks) and repeats, with
    `Error::Config { setting: "map.recent_columns", .. }`.
  - Like `bands`, a higher layer replaces the whole list.
- **To pages**: `hello` gains `recent: {"columns": [...], "header": true}`. The list is ordered,
  as an array, per API-4. It is an additive field, so the protocol stays 3.

## R3. CQ zone (FR-005, FR-013, SC-007)

- **Decision**: the feature-005 entity gains the zone.
  - `CountryData::entity_of` becomes `entity_info(call) -> Option<EntityInfo { prefix, name,
    cq_zone }>`, using the same exact-call and longest-prefix rules.
  - `cq_zone` applies the per-callsign override, `overrides.cq.unwrap_or(entity.cq)`, the same
    value `lookup` writes to `CQZ`.
  - `QsoEntity` gains `cq_zone: u8`. On the wire it is `entity: {prefix, name, cq_zone}`.
- **Rationale**: the zone comes from the same cty.dat match as the entity, so storing it with the
  entity keeps one lookup and one rule. Corrections update it with the entity (feature 005).
- **Alternative**: use the record's `CQZ` field. It is present only when cty.dat gave the
  position, and a logger value may disagree.

## R4. Box rendering (FR-004–FR-006a, FR-010)

- **Decision**: in `recent.js`, pure functions return plain data and the glue builds the table:
  - `recentQsos(state, size, order)` gives the newest `size` of `held(state)`. `order` is
    `'bottom'` (oldest first, newest last) or `'top'` (newest first);
  - `headerCells(columns)` gives `[{column, text}]` with the labels Call, Band, Mode, Date, UTC,
    CQ and Country;
  - `rowCells(qso, columns)` gives `[{column, text, title}]`:
    - `call`, `band`, `mode` (with submode) and `date`/`time` follow feature 005;
    - `cqzone` is the number, or `—`;
    - `country` uses the feature-005 `shortName`/`entityLabel` rules, with the full label as
      `title`;
  - `onMap(qso, markers)` is true when the QSO's marker is in the map's `markers()`.
- **Glue**: one `<table>`, with an optional `<thead>` row, styled as now (no borders).
  - Class `call` is bold, class `country` is grey, and the header is a smaller grey row.
  - Rows not on the map get the class `off-map` (opacity 0.55) and the title
    `Not on the map (outside its last N)`. They have no click handler and no `tabindex`.
- **Why a real table**: it gives the alignment of FR-006 and SC-005 for free, at any width.

## R5. Viewer controls and storage (FR-007, FR-008, FR-011, FR-012, Principle IV)

- **Decision**: the top-right `.limit-control` becomes a small panel with three selects:
  - "Map: last [10 · 20 · 50 · 100 · 200 · 500 · All] QSOs";
  - "Box: last [3 · 5 · 10 · 15 · 20]";
  - "Newest: [bottom · top]".
- **Storage**: everything goes to `localStorage` (Principle IV):
  - `lqm.limit`: the existing key, now in local storage instead of the tab's session storage, so
    a tab-only value is not carried over (FR-011);
  - `lqm.recent.size` (default 5);
  - `lqm.recent.order` (default `bottom`).

  Helpers in `view.js` follow the `loadAutoZoom` pattern, and fall back to the default on any
  unknown value or storage error.
- **On change**:
  - the box size and order re-render the box at once;
  - the map limit updates `mapLimit`, re-renders and sends `set_limit` with `max(map, 20)` (R1).
