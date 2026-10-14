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
| LL-004 | open | A page file added to src/web/assets/ must also be served | 2026-10-13 |
| LL-005 | open | Look at a map effect in the browser before designing around it | 2026-10-14 |
| LL-006 | open | Lookup sources carry no identity, so failed lookups could not be named | 2026-10-14 |
| LL-007 | open | Browser steps that never ran in CI blocked the release three times | 2026-10-14 |

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

## LL-005: Look at a map effect in the browser before designing around it

- Status: open
- Date: 2026-10-14
- Phase: execute
- Category: process
- Owner: maintainer
- Links: specs/013-atmosphere-follows-globe/research.md (R1–R2, R7, revised); TR-019, TR-020, TR-021
<!-- key: 27af3a38f154 -->

### What happened

Feature 013 planned two rendering designs that passed analysis, review and their unit tests, and failed on first sight in Chrome:
- the Earth-fixed light for the globe's glow put a pale haze over the whole day side (MapLibre's scattering shader lights the disc, not only the rim), dimming land, labels and markers;
- the grayline as 25 stacked GeoJSON caps made the globe 3× slower to drag.

Both were redesigned during implementation (TR-021, TR-020), and the spec, plan, research, contracts and tasks rewritten to match.

### What went well / where we got lucky

The release browser runner from feature 012 was already there: a screenshot and a frame-rate number showed each problem within minutes of wiring, before anything shipped. The pure maths (subsolar point, Sun height, replay tests) carried over to the new designs unchanged.

### What did not work

Research checked the geometry numerically (the light direction, the caps against `solarElevation`), but nothing looked at the rendered result or timed it until implementation. The research even rejected the final designs on assumptions that turned out wrong: that a per-move light lags a frame, and that raster tiles flash on refresh.

### Root cause

Rendering behaviour (shader lighting, how the globe subdivides polygons) isn't visible in source code or unit tests. The planning steps had no step that ran the effect in a browser.

### Recommendation / action item

- Action: for a new map rendering effect, run a throwaway browser spike during planning (one screenshot and one frame-rate reading through `tests/browser/`), and record what it showed in research before choosing. Owner: maintainer. Status: open.
- Action: write down untested assumptions in research as assumptions, with how to check them. Owner: maintainer. Status: open.

## LL-006: Lookup sources carry no identity, so failed lookups could not be named

- Status: open
- Date: 2026-10-14
- Phase: execute
- Category: design
- Owner: -
- Links: specs/014-adif-persistence/research.md (R7); src/enrich/resolve.rs (resolve_remote names); src/enrich/mod.rs (Lookups::with_names); spec FR-029
<!-- key: d440be00908e -->

### What happened

Feature 014 needed APP_LIVEQSOMAP_LOOKUP_FAILED = source:kind for every online source that failed. During implementation it turned out that a LookupSource is an anonymous closure: a timeout or an HTTP error carries no provider name, and a not-found answer (Ok(None)) cannot be told apart from the country data's. Research R7 had assumed the names were available.

### What went well / where we got lucky

TDD on resolve_remote caught it before any integration code; the fix stayed local: an optional name list per source index (Lookups::with_names, filled by build_sources), falling back to the provider named in Error::Provider, then to 'lookup'.

### What did not work

The plan treated 'which source failed' as a given; the existing test (slow sources dropped within 300 ms) also ruled out giving providers their own, shorter timeout.

### Root cause

The source abstraction was designed for answers (LookupCandidate carries its source), not for failures; the plan did not check the types it relied on.

### Recommendation / action item

When a plan needs a new attribute of an existing abstraction (here: a failure's source), check the type carries it during /speckit-plan, not at implementation. Consider replacing the parallel name list with a named source type if more per-source data is needed.

## LL-007: Browser steps that never ran in CI blocked the release three times

- Status: open
- Date: 2026-10-14
- Phase: execute
- Category: testing
- Owner: -
- Links: PRs #19, #21; tags v3.1.0, v3.1.1 (unpublished), v3.1.2; tests/browser/run.mjs; docs/ci.md section 7
<!-- key: 9a2b725a9501 -->

### What happened

Feature 013's browser steps 10-11 were written and tuned on a local GPU. Browser tests run only on release tags, so CI first ran them when 3.1.0 was cut. On CI's software renderer (3-5 frames/s) several assumptions broke in turn: drag size (fixed 0.5 deg/px), fixed repaint delays before on/off captures, a 3-frame minimum for a one-second flight, judging the Sun symbol by brightness (it matched the land under it), and auto-zoom refits during theme switches. 3.1.0 also failed at step 08, whose 'empty map after a restart' assumption feature 014 had made obsolete.

### What went well / where we got lucky

Release rehearsals (docs/ci.md section 7) proved each fix in CI without burning tags; the last tag (3.1.2) was published on the first try after a green rehearsal. CI screenshots and report.json pinpointed every cause.

### What did not work

Tagging a fix and hoping (3.1.1); each failure only revealed the next timing-sensitive check. CPU throttling locally reproduced the zoom but not the slow rendering (it does not reach the GPU process).

### Root cause

The release-only browser tests are the only place those steps run, and no rehearsal was run when they were added (013) or when behaviour they assert changed (014).

### Recommendation / action item

Run a release rehearsal before merging any change to tests/browser/ or to behaviour a browser step asserts. In browser steps prefer settled readings over fixed delays, colour over brightness, and interactions sized from what they measurably did.

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
