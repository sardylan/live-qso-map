---
maturity: growing
type: lessons
updated: 2026-10-12
---

# Lessons Learned

Insights, failed approaches, rework, and blockers, captured blamelessly
so the team replicates what worked and avoids what did not.

## Active index

| ID | Status | Title | Date |
|----|--------|-------|------|
| LL-001 | open | Inline lint suppressions are blocked by a project hook | 2026-10-11 |
| LL-002 | open | A valid pipeline config had never run | 2026-10-12 |
| LL-003 | open | cargo-release step subcommands skip the safety checks | 2026-10-12 |

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

## LL-002: A valid pipeline config had never run

- Status: open
- Date: 2026-10-12
- Phase: execute
- Category: process
- Owner: maintainer
- Links: specs/009-fix-ci-pipeline/reference/circleci-review.md (item 0); TR-012; docs/ci.md
<!-- key: 954273315d69 -->

### What happened

Feature 008 built a full CircleCI release path and `circleci config validate` reported it valid. The v2.0.0 tag produced nothing. The run history showed the pipeline had never passed a single job: every job failed at `Checkout code` with `Permission denied (publickey)`. Once checkout worked, a second hidden failure appeared (the page tests ran nothing on Node 24).

### What went well / where we got lucky

The review asked for the run history before planning fixes, so the real blocker was found first. The CircleCI CLI exposes runs, jobs and step logs, so the cause was confirmed from the failing step's output rather than guessed.

### What did not work

Trusting static validation: a config can be schema-valid and still never execute. The release path was designed and documented without one green run behind it.

### Root cause

The project used CircleCI's legacy OAuth connection, which clones over SSH with a checkout key, and GitHub had no matching key. SSH needs an authorised key even for public repositories; only HTTPS allows anonymous reads. Nothing checked the run results after the project was followed, and failures were only visible on CircleCI.

### Recommendation / action item

- Action: before trusting a pipeline, check `circleci run list` and the checkout step log, not only `circleci config validate`. Owner: maintainer. Status: documented in docs/ci.md (Diagnosis).
- Action: rehearse release paths (`--param rehearsal=true`) before tagging whenever they change. Owner: maintainer. Status: done (feature 009).
- Action: keep checkout on the GitHub App (HTTPS); never re-add an OAuth trigger. Owner: maintainer. Status: done (TR-012).

## LL-003: cargo-release step subcommands skip the safety checks

- Status: open
- Date: 2026-10-12
- Phase: execute
- Category: technology
- Owner: maintainer
- Links: specs/009-fix-ci-pipeline/research.md (R7); specs/009-fix-ci-pipeline/contracts/release-process.md; docs/releasing.md; TR-013
<!-- key: fb12120f2618 -->

### What happened

The planned prepare step ran cargo-release 1.1.6 as separate steps (`cargo release version`, `replace`, `commit`). Tested in a scratch clone (T041), `version` and `replace` changed `Cargo.toml` and `Cargo.lock` even with uncommitted changes and on a branch outside `allow-branch`, with `--execute`.

### What went well / where we got lucky

The release flow was dry-run in a throw-away clone with a local bare remote before being documented, so the gap was found without touching the real repository or tags.

### What did not work

Assuming each step subcommand applies the same checks as the full `cargo release` command. A refused prepare (empty "Unreleased") also leaves the version already bumped.

### Root cause

In cargo-release, the dirty-tree and allowed-branch checks run in the full release command (and in some steps, such as `commit` and `tag`), not in `version` or `replace`. Replacements run after the version bump, so a failing `exactly = 1` replacement aborts with the bump already applied.

### Recommendation / action item

- Action: prepare with `cargo release <level> --no-tag --no-push --no-publish --execute`, which checks the tree and the branch first. Owner: maintainer. Status: done (docs/releasing.md).
- Action: after a refused prepare, run `git restore Cargo.toml Cargo.lock`. Owner: maintainer. Status: documented.
- Action: dry-run release tooling in a scratch clone before documenting a process. Owner: maintainer. Status: open.

## LL-004: A page file added to src/web/assets/ must also be served

- Status: open
- Date: 2026-10-13
- Phase: review
- Category: testing
- Owner: maintainer
- Links: specs/012-page-fixes-browser-tests/spec.md (Context, B1); tests/ws_protocol.rs (every_page_asset_on_disk_is_served); docs/ci.md section 11; TR-018
<!-- key: c7847d69d754 -->

### What happened

Release 3.0.0 shipped with four new page modules (`themes.js`, `sun.js`, `panel.js`, `legend.js`) that the server never served. Chrome got a 404 for each, `app.js` failed to load, and the map page lost its buttons, its settings button and its legend. It was found by opening the page in Chrome after the release.

### What went well / where we got lucky

Driving Chrome over the DevTools Protocol from a short Node script found the cause in minutes (four 404s in the console), and the same approach became the release-time browser tests.

### What did not work

The page tests import modules straight from the source tree, so they passed. The server test checked a fixed list of files that nobody updated. The feature 011 browser checks (T008, T015, T019, T030) were still open when 3.0.0 was tagged.

### Root cause

Page files are embedded and routed one by one in `src/web/mod.rs`, and no test compared that list with the files on disk. No check opened the page through the server, in a browser, before publishing.

### Recommendation / action item

- Action: `every_page_asset_on_disk_is_served` fetches every `.js` and `.css` in `src/web/assets/` on every change. Owner: maintainer. Status: done (feature 012).
- Action: the `browser-test` job opens the release build's page in Chrome before `publish-github-release`. Owner: maintainer. Status: done (feature 012).
- Action: finish a feature's manual checks before tagging a release that contains it. Owner: maintainer. Status: open.

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
