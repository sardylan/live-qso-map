---
maturity: growing
type: tradeoffs
updated: 2026-10-08
---

# Tradeoffs

Decisions made over this project's lifetime, and the alternatives
we deliberately gave up. Records the *why*, not just the *what*.

## Active index

| ID | Status | Title | Date |
|----|--------|-------|------|
| TR-001 | proposed | Drop identical resends before enrichment, order versions by revision | 2026-10-08 |
| TR-002 | proposed | Removals continue the revision sequence and leave a tombstone | 2026-10-08 |
| TR-003 | proposed | Pin MapLibre 5.24.0 instead of 6.x | 2026-10-08 |

## Decisions

## TR-001: Drop identical resends before enrichment, order versions by revision

- Status: proposed
- Date: 2026-10-08
- Phase: plan
- Deciders: project owner
- Links: specs/001-live-qso-map/research.md#r3-order-of-duplicate-detection-and-enrichment
<!-- key: e3bcfc020575 -->

### Context & problem

QSOs are resent often by loggers (duplicate packets, edits). Lookups are not cached (FR-016) and count against QRZ.com/HamQTH quotas. Enrichment workers may run in parallel, so versions of the same QSO can finish out of order.

### Decision drivers

- Save provider lookup quota
- Corrections must replace, never duplicate (FR-008/FR-009)
- Newest version must win with W>1 workers

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Resend gate before enrichment + per-identity revision (chosen) | identical resends cost no lookups; out-of-order completions resolved by rev | two places hold per-identity state (gate and history); gate needs its own eviction |
| Deduplicate only in history after enrichment | single source of truth, simpler | every resend triggers lookups on all providers |

### Decision

Resend gate keyed by identity + fingerprint of the pre-enrichment record, assigning rev; history and pages accept only higher rev.

### Y-statement

In the context of uncached lookups and frequent logger resends, facing quota cost and parallel workers, we decided for a pre-enrichment resend gate with revision numbers, and neglected history-only dedupe, to achieve zero lookups for identical resends and newest-version-wins, accepting duplicated per-identity state in gate and history.

### Consequences

- Positive: Identical resends are free; corrections are deterministic regardless of worker count.
- Negative / debt accepted: Gate and history must share the same capacity rule; extra state to test.

## TR-002: Removals continue the revision sequence and leave a tombstone

- Status: proposed
- Date: 2026-10-08
- Phase: plan
- Deciders: project owner
- Links: specs/003-band-filter/research.md#r3-drop-point-and-removal-when-a-correction-moves-onto-an-ignored-band
<!-- key: 7ab95e3fb9a8 -->

### Context & problem

Feature 002 removes a QSO when a correction moves it onto an ignored band. With several enrichment workers, an older revision of the same QSO can still be in lookups when the removal arrives.

### Decision drivers

- A removed QSO must not reappear (FR-008, FR-012)
- Removals must obey the same newest-wins rule as corrections
- A correction back onto a considered band must reappear

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Continue the revision sequence + history tombstone (chosen) | stale in-flight revisions cannot resurrect a removed QSO; a later correction back has a higher rev and reappears | extra bounded tombstone state and one more rule in upsert |
| Forget the QSO in gate and history | simplest | an older revision still being enriched can put the removed QSO back on the map |

### Decision

The gate assigns the removal the next rev; History::remove keeps a tombstone id→rev (capped like the history); upserts with rev <= tombstone are stale.

### Y-statement

In the context of removals racing parallel enrichment, facing stale revisions resurrecting removed QSOs, we decided for revision-numbered removals with tombstones, and neglected forgetting the QSO, to achieve consistent newest-wins semantics, accepting a bounded tombstone map.

### Consequences

- Positive: Removals and corrections share one ordering rule; works with any worker count.
- Negative / debt accepted: Tombstones must be capped and tested; upsert gains a check.

## TR-003: Pin MapLibre 5.24.0 instead of 6.x

- Status: proposed
- Date: 2026-10-08
- Phase: plan
- Deciders: project owner
- Links: specs/002-maplibre/research.md#r1-maplibre-version-and-delivery-fr-011
<!-- key: b9d7d1e5c046 -->

### Context & problem

Feature 002 replaces Leaflet with MapLibre GL JS, loaded from a CDN with full Subresource Integrity (FR-011). MapLibre 6.x is ES-module only: its entry imports a shared chunk and starts a module worker, which SRI cannot cover.

### Decision drivers

- Full SRI on the map library (FR-011, clarified)
- Globe projection and GPU layers needed (FR-010, SC-002)
- Keep the CDN decision (no embedding)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| MapLibre 5.24.0 UMD from cdnjs (chosen) | single bundle, JS and CSS fully SRI-checked; has globe, setDOMContent, circle-sort-key | 5.x line no longer updated |
| MapLibre 6.11.2 with partial SRI via import map | current major | worker module loads unverified; relies on import-map integrity support |
| MapLibre 6.11.2 embedded in the binary | no CDN, no SRI needed | reverses the clarified CDN decision; ~1.1 MB larger binary |

### Decision

Pin MapLibre GL JS 5.24.0 from cdnjs with SHA-512 SRI verified against the downloaded files.

### Y-statement

In the context of replacing Leaflet with full-SRI CDN delivery, facing MapLibre 6 being ESM-only with an unverifiable worker, we decided for the last 5.x UMD release, and neglected 6.x partial-SRI or embedding, to achieve fully integrity-checked delivery, accepting a frozen 5.x line and a later migration.

### Consequences

- Positive: Every byte of the map library is integrity-checked; same delivery model as Leaflet.
- Negative / debt accepted: No upstream updates on 5.x; migrating to 6 later touches index.html and the app.js glue.

## Archive

Superseded or deprecated entries sink here; nothing is deleted (git keeps history).

<!-- ENTRY TEMPLATE: copy a block into the Decisions section above the
Archive heading, assign the next TR-NNN id, and fill it in. The journal_append
helper does this automatically; this block is the fallback for hand-editing.

## TR-NNN: <short decision title>

- Status: proposed
- Date: YYYY-MM-DD
- Phase: brainstorm | specify | plan | execute | review
- Deciders: <names/roles>
- Links: <PR/commit/issue>, <code paths>

### Context & problem

<the situation forcing a choice>

### Decision drivers

- <competing quality / constraint>

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| A (chosen) | ... | ... |
| B | ... | ... |

### Decision

We chose A.

### Y-statement

In the context of <X>, facing <concern>, we chose A over B,
to achieve <quality>, accepting <the sacrifice / road not taken>.

### Consequences

- Positive: <what gets easier>
- Negative / debt accepted: <what gets harder; revisit trigger>
-->
