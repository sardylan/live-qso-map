---
maturity: growing
type: lessons
updated: 2026-10-11
---

# Lessons Learned

Insights, failed approaches, rework, and blockers, captured blamelessly
so the team replicates what worked and avoids what did not.

## Active index

| ID | Status | Title | Date |
|----|--------|-------|------|
| LL-001 | open | Inline lint suppressions are blocked by a project hook | 2026-10-11 |

## Lessons

## LL-001: Inline lint suppressions are blocked by a project hook

- Status: open
- Date: 2026-10-11
- Phase: execute
- Category: process
- Owner: -
- Links: specs/008-platform-packaging/research.md; tests/config_export.rs; tests/cli_help.rs
<!-- key: 09d74eff2483 -->

### What happened

Two planned designs for feature 008 relied on inline allow attributes: the Windows console check used unsafe code under #[allow(unsafe_code)], and new integration tests opened with #![allow(clippy::unwrap_used, clippy::expect_used)]. A hook rejects any inline lint suppression, even inside research notes, so both had to be redesigned mid-plan and mid-implementation.

### What went well / where we got lucky

The hook caught the suppressions before they reached a commit; the replacements are simpler and need no unsafe code.

### What did not work

Designing first and checking against the hook later: the Windows pause had to be redesigned and FR-019 relaxed (the user chose to pause whenever input is interactive).

### Root cause

The no-inline-suppression rule lives only in the hook, not in docs/rust-guidelines.md, so research and planning did not take it into account.

### Recommendation / action item

Check new designs against the hook during planning. Integration tests return Result and use ? instead of unwrap/expect. Platform checks use the standard library before reaching for unsafe or FFI.

## Archive

Superseded or deprecated entries sink here; nothing is deleted (git keeps history).

<!-- ENTRY TEMPLATE: copy a block into the Lessons section above the Archive
heading, assign the next LL-NNN id, and fill it in. The journal_append helper
does this automatically; this block is the fallback for hand-editing.

## LL-NNN: <short lesson title>

- Status: open
- Date: YYYY-MM-DD
- Phase: execute | review
- Category: process | technology | requirements | testing | communication
- Owner: <who carries the follow-up>
- Links: <PR/commit/issue>, <related TR-NNN>

### What happened

<blameless, factual: the situation/activity>

### What went well / where we got lucky

<successes worth replicating>

### What did not work

<the gap or failure>

### Root cause

<5 Whys / contributing factors>

### Recommendation / action item

- Action: <specific change>. Owner: <name>. Due: <date>. Status: <...>
-->
