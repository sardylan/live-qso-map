# Research: Band Filter

**Feature**: `003-band-filter` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items: the owner settled the decisions on 2026-10-09. This file
records the design decisions that follow from the spec and from feature 001's architecture.

## R1. Where band groups and selection parsing live

- **Decision**: a new module, `src/band_filter.rs`, holds the group table (FR-003), selection
  parsing and resolution (FR-002, FR-004, FR-005) and the `BandSet` type. It depends on `band.rs`
  for band names and frequency order (`band_rank`). It is pure code, with no I/O.
- **Rationale**: `band.rs` is the ADIF band plan, i.e. facts. Selection is configuration logic with
  its own errors. Keeping them apart keeps both small and easy to test.
- **Alternatives**: putting groups in `band.rs`, which mixes domain data with configuration rules;
  putting them in `config.rs`, which is already the largest module.

## R2. Representing the effective set

- **Decision**: `BandSet` is the list of band names, sorted by `band_rank` (frequency order, HR-2),
  with no duplicates. `Option<BandSet>` in `Config`: `None` means no filter is active (FR-004,
  FR-009). Membership is a linear search. There are at most 33 bands, so a hash set would add
  nothing.
- **Exclusions win**: resolution is `(union of positive entries, or every band when there are none)
  minus (union of excluded entries)`. This is the confirmed default.

## R3. Drop point, and removal when a correction moves onto an ignored band

- **Decision**:
  1. **Filtering happens in the ingest task** after `normalise`, which fixes the band (including
     the band derived from `FREQ`, HR-3). The check runs **before** the resend gate.
  2. **A QSO on an ignored or unknown band:**
     - If the resend gate already knows its identity, or its previous identity (`oldcall`/
       `oldtimestamp`), it is a correction of a QSO that may be on the map. The gate assigns the
       next revision, and ingest calls `Pipeline::remove(id, rev)`.
     - Otherwise it is dropped (FR-007) and logged at debug level.
  3. **`History::remove(id, rev)`** deletes the entry and keeps a **tombstone** `id → rev`. Later
     upserts for that id with `rev <= tombstone` are `Stale`. A higher revision (a correction back
     onto a considered band) clears the tombstone and is inserted as a new entry. The page sees it
     as a new QSO, which satisfies "treated as new" in FR-008.
  4. **The broadcast** is `HistoryChange::Remove { id }`, sent while holding the history lock, as
     for upserts (feature 001 R4). It is sent only when an entry was actually removed.
- **Rationale**: with several workers, an earlier revision of the same QSO can still be in lookups
  when the removal arrives. If the gate forgot the QSO and the history kept no trace, that stale
  revision would bring the QSO back after its removal. Continuing the revision sequence and keeping
  a tombstone make removals obey the same newest-wins rule as corrections.
- **Alternatives**: forgetting the QSO entirely in both the gate and the history. That is simpler,
  but allows the stale revision to come back. Routing removals through channel 1 to the workers:
  removals would need no lookup, yet would queue behind slow lookups, and ordering against
  in-flight revisions would still need tombstones.
- **Tombstone bound**: tombstones are capped at `history_size` (0 = unbounded) and evicted
  oldest-first. A tombstone only matters while a stale revision can still be in flight, so losing an
  old one is harmless.

## R4. Configuration surface (FR-001)

- **Decision**: one setting, `bands`.
  - Command line: `--bands hf-contest,-60m` (comma-separated, repeatable).
  - Environment: `LQM_BANDS=hf-contest,-60m`.
  - TOML: `bands = ["hf-contest", "-60m"]`.

  The usual precedence applies, and a higher layer replaces the whole list (no merging, so the
  result is predictable). An absent or empty value means no filter.
- **Validation**: errors are `Error::Config { setting: "bands", message }` naming the entry (FR-005).
  The effective set is logged once by `pipeline::start` at info level: `bands considered: 160m 80m
  40m 20m 15m 10m`, or `bands considered: all` (FR-006).

## R5. Legend and colours (FR-010)

- **Decision**: `ServerMessage::hello(map, filter)` keeps feature 002's `protocol` (2) and
  `basemap` unchanged and sends only palette entries whose band is in the
  effective set, still in frequency order (`bands_in_order`). Colours configured for ignored bands
  stay in `Config` (accepted, the confirmed default) and are never sent. A considered band without a
  palette entry is not listed in the legend and draws in the fallback colour, as in feature 001.

## R6. Protocol addition (FR-011)

- **Decision**: a new server message, `{"type":"remove","id":"…"}`. The page applies it with
  `applyRemove(state, id)` in `map-state.js` (pure, tested), then `diffLayers` as usual. Removals
  are not tied to the page's window: a page that never showed the QSO ignores them. The protocol
  version stays 2 (set by feature 002), because pages already ignore unknown message types.
- **Open popups** (analyze C1): on the MapLibre page (feature 002), popups are static snapshots. When
  a removal deletes the marker whose popup is open, the page closes the popup, and it refreshes the
  content when that marker changes. The decision is the pure `view.js::popupToClose(openKey, diff)`,
  so a removed QSO never stays on screen (FR-012).
- **Snapshots**: they are built from the history, so a removed QSO is absent from them.

## R7. Dependencies

No new crates. Everything uses the existing modules and the standard library.
