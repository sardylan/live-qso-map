# Data Model: QSOs Kept in a Local ADIF File

Feature 014. Extends the data model of features 001, 003, 005 and 010. Names in `code` are the
planned Rust names.

## History file (on disk)

One ADIF file (contracts/history-file.md).

| Part | Content |
|------|---------|
| Header | a comment line, `ADIF_VER`, `PROGRAMID`, `PROGRAMVERSION`, `CREATED_TIMESTAMP` (UTC), `<EOH>` |
| Records | one per QSO, one per line, in QSO time order (`QSO_DATE` + `TIME_ON`) |

Lifecycle: read once at startup (or by enrich-only mode), replaced as a whole at a graceful stop
(write to `<name>.tmp`, then rename). Deleted by the operator to reset (between stop and start).
Moved aside to `<name>.unreadable-YYYYMMDDTHHMMSSZ` when unreadable at a normal startup.

## Fields added to a saved record

All new fields follow HR-11 (`APP_LIVEQSOMAP_` prefix, uppercase).

| Field | When present | Value | Rule |
|-------|--------------|-------|------|
| `APP_LIVEQSOMAP_ADDED` | the service enriched the record | names of the fields the service added, comma-separated, in name order | R3 |
| `APP_LIVEQSOMAP_LOOKUP_FAILED` | every online source failed on the logged call | `source:kind` entries in configured order, `kind` ∈ `error`, `timeout` | R7, FR-029 |

Existing fields that matter for restoring: `APP_LIVEQSOMAP_POS_SOURCE`,
`APP_LIVEQSOMAP_POS_PRECISION` (feature 001, HR-7), `APP_LIVEQSOMAP_POS_REJECTED` (feature 010),
`APP_LIVEQSOMAP_STATION_POS_SOURCE`, `APP_LIVEQSOMAP_LOGGER_ID` (QARTest).

Invariant: removing the fields listed in `APP_LIVEQSOMAP_ADDED`, and `APP_LIVEQSOMAP_ADDED` itself,
gives the record as received from the logger or as found in an external file. Every field not
listed is never changed by the service (HR-11).

## Restored record → what happens to it

`restore::classify(record, mode, filter) -> RestoreAction`:

| Condition (checked in order) | Normal run | Enrich-only |
|------------------------------|-----------|-------------|
| `CALL`, `QSO_DATE` or `TIME_ON` invalid | skipped, counted, logged | same |
| band not considered (filter active) | `Park` — written back verbatim | (filter not applied) |
| has `APP_LIVEQSOMAP_LOOKUP_FAILED` | `Enrich` from as received | `Enrich` from as received |
| has `APP_LIVEQSOMAP_POS_REJECTED` | `Reuse` if it has a saved position, else `Enrich` | `Enrich` from as received |
| has saved position (source, precision, `LAT`/`LON`) | `Reuse` | `Reuse` |
| otherwise | `Enrich` from as received | `Enrich` from as received |

- **Reuse**: no lookup. Remote position = saved `LAT`/`LON` with saved precision and source.
  Station position = `apply_station` on the record as received with the current configuration.
  Entity = current country data. Record = as saved.
- **Enrich**: `enrich()` on the record as received, exactly like a live QSO.
- **Park**: never queued; kept in `Parked`.

## In-memory structures

### `QsoEvent` (extended)

| Field | Type | New | Meaning |
|-------|------|-----|---------|
| `saved` | `Option<AdifRecord>` | yes | the full saved record, for `Reuse`; `None` for live events and `Enrich` |

`record` is always the record as received (R3), so the gate fingerprints match a logger resend.

### `History` (changed)

- `capacity` removed: never trimmed (FR-023). `upsert`, `remove`, tombstones unchanged.
- new `iter_in_order() -> impl Iterator<Item = &Arc<EnrichedQso>>` (oldest first), for the save.

### `ResendGate` (changed)

- unbounded (it was sized like the history).

### `Pending` (new, shared)

`Arc<Mutex<HashMap<String, AdifRecord>>>`: id → record to write if the QSO is still waiting at
save time (the saved record for restored events, the record as received for live ones). Inserted
when an event is queued; removed when a worker publishes or drops it as stale.

### `Parked` (new)

`Vec<(String, AdifRecord)>`: id and verbatim record of file records on bands not considered
(normal run only). Built once at load; read once at save.

### `LoadReport` (new)

Counts logged at the end of the load and, in enrich-only mode, at the end of the run:
`loaded`, `skipped_malformed`, `reused`, `to_enrich`, `parked`, then (after enrichment)
`placed_by_record`, `placed_by_lookup`, `lookup_failed`, `without_position`.

### `Config` (changed)

| Field | Type | Default |
|-------|------|---------|
| `history_size` | removed | — (accepted and ignored with a warning, R14) |
| `history_file` | `PathBuf` | `qso-history.adi` in the folder of `country_data.path` (R1) |

### `Cli` (changed)

| Option | Type |
|--------|------|
| `--history-file <PATH>` | `Option<PathBuf>` |
| `--enrich-only [FILE]` | `Option<Option<PathBuf>>`, conflicts with `--save-config` |
| `--history-size <N>` | hidden, ignored with a warning |

## Page limit (protocol 4, unchanged shape)

| Item | Before | After |
|------|--------|-------|
| `LIMITS` (server and page) | 10, 20, 50, 100, 200, 500, `null` (All) | 10, 20, 50, 100, 200, 500, 1000 |
| `set_limit.limit: null` | every QSO | treated as 1000 |
| `lqm.limit` = `"all"` | All | read as 1000, saved back as `"1000"` |
