# Data Model: Band Filter

**Feature**: `003-band-filter` | **Date**: 2026-10-09

The entities from feature 001 apply unchanged unless listed here.

## BandGroup (static)

| Name | Bands (frequency order) |
|------|-------------------------|
| `lf-mf` | 2190m, 630m, 560m |
| `hf` | 160m, 80m, 60m, 40m, 30m, 20m, 17m, 15m, 12m, 10m |
| `hf-contest` | 160m, 80m, 40m, 20m, 15m, 10m |
| `warc` | 30m, 17m, 12m |
| `vhf` | 8m, 6m, 5m, 4m, 2m, 1.25m |
| `uhf` | 70cm, 33cm, 23cm |
| `shf` | 13cm, 9cm, 6cm, 3cm, 1.25cm |
| `ehf` | 6mm, 4mm, 2.5mm, 2mm, 1mm, submm |

Every group member MUST be an ADIF band (a test checks this) and MUST be listed in frequency order
(a test checks this too, HR-2).

## SelectionEntry (parsed)

```text
SelectionEntry { exclude: bool, target: Band(&'static str) | Group(&'static str) }
```

Parsing rules:

- Input is trimmed and matched case-insensitively.
- A leading `-` means exclude.
- Empty entries (e.g. a trailing comma) are skipped.
- Anything else that is neither a band nor a group is an error naming the entry.

## BandSet (effective)

```text
BandSet { bands: Vec<&'static str> }   // sorted by band_rank, no duplicates, never empty
```

- `resolve(entries)`: (union of positive entries, or every band when there are none) minus the union
  of excluded entries. An empty result is an error.
- `contains(band: &str) -> bool`: case-insensitive.
- `Display`: the band names separated by spaces, in frequency order.

`Config.band_filter: Option<BandSet>`. `None` means no filter is active.

## History (changes)

```text
History { …001 fields…, tombstones: HashMap<String, u32>, tombstone_order: VecDeque<String> }
```

| Operation | Rule |
|-----------|------|
| `remove(id, rev)` | If `id` is present and `rev > stored.rev`: remove it, record `tombstones[id] = rev` (capped like the history) and return `Removed`. Otherwise, record or raise the tombstone and return `NotPresent`. |
| `upsert(q)` (amended) | When `tombstones[q.id] >= q.rev`, return `Stale`. When `q.rev` is higher, clear the tombstone and insert `q` as a new entry with its own sort key. |

## ResendGate (changes)

`contains(event) -> bool`: true when the event's identity, or its previous identity for a correction
without a logger id, has an entry. Ingest uses it to decide between removal (R3) and a silent drop.

## HistoryChange (changes)

```text
HistoryChange = Upsert(Arc<EnrichedQso>) | Remove { id: String }
```

## Frontend state (changes)

`applyRemove(state, id)` deletes the QSO from `state.qsos`. A later upsert with the same id is
treated as new, regardless of `rev`, because the page no longer holds it.

`popupToClose(openKey, diff)` in `view.js`: `true` when the marker of the open popup is in
`diff.remove`. When it is in `diff.update`, the glue refreshes the popup content instead (FR-012).
