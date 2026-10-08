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
