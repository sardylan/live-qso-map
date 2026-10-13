---
maturity: growing
type: tradeoffs
updated: 2026-10-13
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
| TR-004 | proposed | JSON interface rules in docs/http-api.md, referenced by the constitution | 2026-10-09 |
| TR-005 | proposed | Entity extents generated from Natural Earth by a dev-only script | 2026-10-09 |
| TR-006 | proposed | The page requests max(map Last N, 20) QSOs to feed both the map and the box | 2026-10-09 |
| TR-007 | proposed | Score staleness is judged from a server-supplied age, not the browser clock | 2026-10-09 |
| TR-008 | proposed | Cross-compile only in the pipeline; gate every main commit, build releases on tags | 2026-10-11 |
| TR-009 | proposed | Feature branches named feature/NNN-title and deleted after merge | 2026-10-11 |
| TR-010 | accepted | Purge deletes the service account, guarded by a GECOS marker | 2026-10-11 |
| TR-011 | accepted | Windows waits for Enter after an error whenever input is interactive | 2026-10-11 |
| TR-012 | accepted | Checkout through the CircleCI GitHub App with one only-build-prs trigger | 2026-10-12 |
| TR-013 | accepted | The version lives in Cargo.toml and the tag follows it; two-step release with cargo-release | 2026-10-12 |
| TR-014 | accepted | A manual release rehearsal instead of pre-release tags | 2026-10-12 |
| TR-015 | accepted | Positions are judged against entity rectangles, excluding only fully nested ones | 2026-10-13 |

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

## TR-004: JSON interface rules in docs/http-api.md, referenced by the constitution

- Status: proposed
- Date: 2026-10-09
- Phase: plan
- Deciders: project owner
- Links: docs/http-api.md, .specify/memory/constitution.md (1.3.0), specs/004-qth-autozoom/spec.md (FR-027)

### Context & problem

The owner asked for a constraint that every HTTP payload (REST and WebSocket) is JSON. The rule needs
detail (exemptions for static files and control frames, an error body shape, conventions) that does
not fit the constitution's short principles, yet it must be binding and checked in every plan.

### Decision drivers

- Binding and checked by the Constitution Check
- Keep the constitution short
- Uniform, machine-readable errors for pages and scripts

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Constitution line pointing to docs/http-api.md (chosen) | binding; detail lives next to the other domain docs; same pattern as docs/ham-radio.md | one more document to keep in sync |
| Full rule inside the constitution | single place | long, detailed text in a document meant for principles |
| Doc only, no constitution reference | no constitution bump | not enforced by the Constitution Check |

For errors: JSON bodies on every 4xx/5xx (chosen) over API endpoints only, or no rule.

### Decision

Add a Technical Constraints line (constitution 1.3.0) that makes docs/http-api.md (API-1 to API-5)
binding. Every error response, including 404, 405 and failed WebSocket upgrades, has a JSON
`{"error","message"}` body.

### Y-statement

In the context of fixing the payload format of every HTTP interface, facing detail that does not fit
a principle, we decided for a binding constitution reference to a dedicated document, and neglected
a constitution-only or doc-only rule, to achieve an enforced yet readable rule, accepting one more
document to maintain and custom error handling instead of the framework's default empty or
plain-text error responses.

### Consequences

- Positive: Clients can rely on JSON everywhere, errors included; plans check it explicitly.
- Negative / debt accepted: The 404, 405 and /ws upgrade error responses must be overridden (folded
  into feature 004, FR-027); every new endpoint needs an error-path contract test.

## TR-005: Entity extents generated from Natural Earth by a dev-only script

- Status: proposed
- Date: 2026-10-09
- Phase: plan
- Deciders: project owner
- Links: specs/004-qth-autozoom/research.md#r2-entity-extents-fr-010-sc-003, tools/entity_extents.py, src/web/entity_extents.json

### Context & problem

With no QSOs, auto-zoom must frame the station's whole DXCC entity (feature 004, FR-010). cty.dat
gives only a centre point per entity, so the extents need another source, bundled with the service.

### Decision drivers

- No runtime download; single binary (Principle III)
- Reviewable, reproducible data with no licence burden
- Coverage of as many entities as possible (SC-003)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Generated from Natural Earth v5.1.2 + hand mapping (chosen) | public domain; reproducible; one mapping line per entity | ~340-entry mapping to write once; Python script and ~55 MB download cache for maintainers |
| Hand-curated full table | no tooling | accuracy cannot be reviewed |
| Hand-curated subset (~60 entities) | small | many entities only get the radius fallback |

### Decision

A dev-only standard-library Python script maps cty.dat prefixes to Natural Earth admin-0 and
admin-1 units and writes a committed JSON table, embedded in the binary. Unmapped entities fall back
to a 500 km radius.

### Y-statement

In the context of framing the station's entity with no QSOs, facing cty.dat holding only centre
points, we decided for extents generated from Natural Earth by a dev-only script, and neglected
hand-curated tables, to achieve reviewable and reproducible coverage, accepting a one-off mapping
effort and a maintainer-only download.

### Consequences

- Positive: Adding or fixing an entity is one mapping line plus a rerun; no runtime fetch.
- Negative / debt accepted: The mapping must follow cty.dat changes; tiny entities missing from
  Natural Earth get only the radius.

## TR-006: The page requests max(map Last N, 20) QSOs to feed both the map and the box

- Status: proposed
- Date: 2026-10-09
- Phase: plan
- Deciders: project owner
- Links: specs/006-recent-box-config/research.md#r1-how-the-page-gets-enough-qsos-for-the-map-and-the-box-fr-009

### Context & problem

Feature 006 makes the last-QSO box independent of the map: the map shows its own "Last N"
(10–500 or All), and the box its own size (3–20), in either order. A page only holds the QSOs the
server sends for its requested window, so with the map at 10 and the box at 15 the box would be
short of rows.

### Decision drivers

- The box always gets its rows (FR-009)
- No protocol change, and no new message (Principle III)
- Little extra traffic

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Request max(map, 20) with the existing `set_limit`, split locally (chosen) | no protocol or server change; 20 and every larger map size are already allowed values | up to 10 extra QSOs per page when the map is below 20 |
| Add 3, 5, 15 to the allowed `set_limit` values and request exactly max(map, box) | sends exactly what is shown | changes the server's allowed set and its tests for a few rows of traffic |
| A second subscription or message for the box window | the two windows are independent on the server | more protocol and per-session state |

### Decision

The page requests `max(map Last N, 20)` (or All), then the map takes its newest `mapLimit` and
the box its newest `size`, locally.

### Y-statement

In the context of a box independent of the map window, facing pages that hold only the
requested window, we decided for always requesting at least 20 QSOs and splitting on the page,
and neglected new allowed sizes or a second subscription, to achieve no protocol change, accepting
up to 10 extra QSOs per page.

### Consequences

- Positive: the server and protocol are unchanged; the box and the map choices stay independent.
- Negative / debt accepted: if the box maximum ever exceeds 20, the floor must follow it.

## TR-007: Score staleness is judged from a server-supplied age, not the browser clock

- Status: proposed
- Date: 2026-10-09
- Phase: plan
- Deciders: project owner
- Links: specs/007-contest-score-layout/research.md#r4-staleness-without-trusting-the-browsers-clock-fr-007-sc-003

### Context & problem

Feature 007 greys out the score line when no score has arrived for 5 minutes. The page and the
service often run on different machines, and a shack PC's clock can be minutes off, so comparing
the server's receive time with the browser clock would grey out live scores or hide stale ones.

### Decision drivers

- A correct stale look whatever the two clocks say (FR-007, SC-003)
- No per-session timers or state on the server (Principle III)
- The tooltip still shows the real UTC time of the last update (HR-1)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| `age_s` in each `score` message; the page subtracts it from its own clock and checks every 5 s (chosen) | immune to clock offsets; no server timers | one extra field; network delay makes the age a little short |
| Compare `received_at` with the browser clock | one field fewer | wrong whenever the clocks differ |
| The server pushes a `stale` flag from a per-session timer | the page holds no timer | a timer, state and another `select!` branch in every session |

### Decision

Each `score` message carries `received_at` (UTC, for the tooltip) and `age_s`. The page records
`Date.now() - age_s × 1000` and greys the line out when that is more than 300 s old.

### Y-statement

In the context of a stale score indicator, facing page and service clocks that may differ by
minutes, we decided for a server-supplied age measured on each side's own clock, and neglected
comparing absolute times or server-pushed staleness, to achieve a correct stale look with no
server timers, accepting one extra field and a sub-second error from network delay.

### Consequences

- Positive: staleness is right regardless of clock offsets; sessions stay timer-free.
- Negative / debt accepted: the age is short by the network delay (well under 1 s), and every
  `score` message carries one extra field.

## TR-008: Cross-compile only in the pipeline; gate every main commit, build releases on tags

- Status: proposed
- Date: 2026-10-11
- Phase: plan
- Deciders: project owner
- Links: .specify/memory/constitution.md (v1.5.0, Principle V); .circleci/config.yml
<!-- key: 8c97aedbb8d3 -->

### Context & problem

Contributors develop on Linux, macOS or Windows; releases target static Linux (x86_64, aarch64, armv7), macOS and Windows. Until now the README told developers to cross-compile locally with `cross`.

### Decision drivers

- Develop on any host without extra toolchains
- Reproducible release binaries
- Keep every commit on main releasable at a reasonable CI cost

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Cross-compile only in CI; quality gates on every main commit; release builds on tags (chosen) | No local cross/Docker/emulation; one reproducible place for releases; cheap CI on day-to-day commits | A target that stops building is found only when a release is tagged; the fix needs a new tag |
| Developers cross-compile locally with cross (previous README) | Release binaries without CI | Needs Docker and cross on every machine; behaves differently per host; builds not reproducible |
| Build every target on each change to main | A broken target is found right after the merge | macOS and Windows build minutes on every push |

### Decision

Constitution 1.5.0, Principle V: development is portable and host-only; cross-compiling and release binaries happen only in the pipeline. The pipeline runs the quality gates on every commit to main and builds release targets only for vX.Y.Z tags.

### Consequences

- Positive: Any developer host works with cargo, node and python3; releases are reproducible; CI stays cheap for normal commits.
- Negative / debt accepted: Target breakage surfaces at release time; cfg gating and the per-target dependency rule are the only guard on main.

## TR-009: Feature branches named feature/NNN-title and deleted after merge

- Status: proposed
- Date: 2026-10-11
- Phase: plan
- Deciders: project owner
- Links: .specify/memory/constitution.md (v1.6.0, Development Workflow); .specify/scripts/bash/common.sh; .specify/scripts/bash/create-new-feature.sh
<!-- key: ff2b30d7914e -->

### Context & problem

spec-kit created feature branches named NNN-short-name, mixed with every other branch, and finished branches were kept or removed by hand.

### Decision drivers

- Tell feature branches apart from main and future release or fix branches
- Keep the branch list showing only work in progress
- Keep the spec-kit scripts working

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| feature/NNN-title branches, deleted locally and on the remote after merge (chosen) | Feature branches group together in git tools; main and other branch types stay distinct; the branch list shows only open work | The scripts strip the prefix to find specs/NNN-title; after deletion, the merged commits and the spec folder are the only record of the feature |
| spec-kit default NNN-title branches | No script changes | Mixed with every other branch |
| Keep finished branches | Easy to look up a past feature's work | The branch list grows with every feature |

### Decision

Constitution 1.6.0, Development Workflow: features are developed on feature/NNN-title, matching specs/NNN-title/, and the branch is deleted locally and on the remote once merged into main.

### Consequences

- Positive: Feature branches are recognisable at a glance and the branch list stays short.
- Negative / debt accepted: The spec-kit scripts depend on a prefix-stripping helper; the plugin's own instructions still describe NNN-title branches.

## TR-010: Purge deletes the service account, guarded by a GECOS marker

- Status: accepted
- Date: 2026-10-11
- Phase: plan
- Deciders: project owner
- Links: specs/008-platform-packaging/research.md#r3-the-service-account; specs/008-platform-packaging/contracts/debian-package.md
<!-- key: 9d2db38afe61 -->

### Context & problem

The Debian package creates a dedicated unprivileged account live-qso-map for the service. The operator asked that purge removes it. Debian policy discourages deleting system accounts, because a reused UID could own leftover files.

### Decision drivers

- Purge leaves nothing behind (SC-002)
- Never delete an account the package did not create
- No files left owned by a deleted UID

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| postinst creates the account with GECOS 'live-qso-map service'; postrm purge deletes /etc and /var/lib first, then the account only if the GECOS marker matches, and with it the group if empty (chosen) | Clean purge as asked; foreign accounts are safe; no orphaned files | Goes against Debian's advice to keep system accounts; relies on the GECOS field not being edited |
| systemd-sysusers | Declarative, no shell logic | Never removes accounts, so purge leaves the account |
| systemd DynamicUser (today's unit) | No persistent account at all | No dedicated account, which the operator asked for; file ownership changes between runs |

### Decision

Purge deletes the service account and group after removing /etc/live-qso-map and /var/lib/live-qso-map, guarded by the GECOS marker of an account this package created.

### Consequences

- Positive: Purge leaves no account, files or service behind; an account created by someone else survives.
- Negative / debt accepted: Deviates from Debian's keep-the-account advice; an operator who edits the GECOS keeps the account after purge.

## TR-011: Windows waits for Enter after an error whenever input is interactive

- Status: accepted
- Date: 2026-10-11
- Phase: plan
- Deciders: project owner
- Links: specs/008-platform-packaging/research.md#r11-keeping-the-windows-window-open-after-an-error; spec FR-019
<!-- key: 9ed5ae50b7e2 -->

### Context & problem

On Windows the program is mainly started by double-click; an error closes the console before it can be read. Telling a double-click from a terminal start needs the GetConsoleProcessList API, an unsafe call, while the project forbids inline lint suppressions and unjustified unsafe code.

### Decision drivers

- Errors stay readable after a double-click (FR-019)
- No unsafe code and no lint exceptions (Principle II)
- No new crates (Principle III)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Wait for Enter after an error whenever standard input is an interactive console (std IsTerminal) (chosen) | Standard library only; no unsafe; works for double-click | Terminal users press Enter once after an error |
| Helper workspace crate holding the unsafe GetConsoleProcessList call | Exact: waits only after a double-click | Extra crate and workspace structure for one call; an unsafe exception |
| A .cmd launcher that runs the exe and pauses | No unsafe | Double-clicking the .exe still closes at once on error |
| Guess from environment variables (PROMPT, WT_SESSION) | No unsafe | Wrong in PowerShell and other terminals |

### Decision

On Windows only, after an error, the program waits for Enter when its standard input is interactive, and exits at once when input is redirected. Spec FR-019 relaxed accordingly.

### Consequences

- Positive: No unsafe code, no new crate, no lint exception; double-click errors stay visible.
- Negative / debt accepted: One extra Enter after an error when run from a terminal.

## TR-012: Checkout through the CircleCI GitHub App with one only-build-prs trigger

- Status: accepted
- Date: 2026-10-12
- Phase: plan
- Deciders: project owner
- Links: specs/009-fix-ci-pipeline/research.md#r1-which-events-start-a-run; specs/009-fix-ci-pipeline/reference/circleci-review.md; spec FR-001, FR-006, FR-010
<!-- key: 5a08e1e7a7be -->

### Context & problem

Every CircleCI run failed at checkout: the project used the legacy OAuth integration, which clones over SSH, and GitHub had no deploy key for it. The repository is public and HTTPS cloning was expected. The run policy also changed: gates on pushes to main and on pull requests, release builds on tags only, one run per event.

### Decision drivers

- The pipeline must fetch the code without managing keys (FR-001)
- Exactly one run per event, none for branches without a pull request (FR-004, FR-006)
- Publishing secrets never reach untrusted code (FR-014)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| GitHub App pipeline (HTTPS checkout) with a single `only-build-prs` trigger; OAuth trigger deleted (chosen) | No SSH or deploy key; one rule covers main, pull requests and tags; CircleCI's current model | Fork pull requests never run (App limit); the OAuth pipeline lingers without triggers because it can't be deleted |
| Add a read-only deploy key to the OAuth project | Smallest fix, no config change | Keeps SSH and a key to rotate; OAuth is the legacy integration |
| Several App triggers (`default-branch-pushes` + PR preset + `only-tags`) | Each rule reads clearly | Overlapping matches start duplicate runs |

### Decision

We moved the project to a GitHub App pipeline with one `only-build-prs` trigger, deleted the OAuth trigger, and handle fork pull requests by re-pushing reviewed commits to an internal branch.

### Y-statement

In the context of a CI that could never check out the code, facing the choice between restoring SSH keys and changing integration, we chose the GitHub App with a single only-build-prs trigger over a deploy key or several triggers, to achieve keyless HTTPS checkout and exactly one run per event, accepting that fork pull requests can't run CI and must be re-pushed by the maintainer.

### Consequences

- Positive: no keys to manage; the run policy lives in one trigger plus config filters.
- Negative / debt accepted: fork contributions need a manual re-push; revisit if CircleCI adds fork builds for GitHub App pipelines.

## TR-013: The version lives in Cargo.toml and the tag follows it; two-step release with cargo-release

- Status: accepted
- Date: 2026-10-12
- Phase: plan
- Deciders: project owner
- Links: specs/009-fix-ci-pipeline/research.md#r7-release-process-with-cargo-release; specs/009-fix-ci-pipeline/contracts/release-process.md; spec FR-029, FR-030
<!-- key: 82269a0093fa -->

### Context & problem

Releases are tagged vX.Y.Z and the pipeline refuses a tag that differs from the Cargo.toml version. Keeping the two equal depended on editing the manifest and typing the tag by hand. main is now protected: changes only through pull requests with green gates.

### Decision drivers

- One source for the version; no typing it twice (SC-012)
- Locked builds keep working (`--locked`)
- No direct pushes to the protected main (FR-009)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Cargo.toml is the source; cargo-release prepares a release/next pull request, then tags the merged commit and pushes only the tag (chosen) | Version typed once; the release commit passes the same gates; no bypass needed | One extra pull request per release; two commands instead of one |
| CI rewrites Cargo.toml from the tag | Release = push a tag | Cargo.lock changes break `--locked`; main reports a wrong version; a tag typo becomes the published version |
| One-step `cargo release patch --execute` pushing to main | Single command | Needs the admin bypass on every release |

### Decision

We keep the version in Cargo.toml and cut releases with cargo-release in two steps: prepare a pull request, then tag the merged commit. The CI version gate stays as a second check.

### Y-statement

In the context of tag-driven releases on a protected main, facing tag/manifest mismatches and locked builds, we chose a manifest-driven two-step cargo-release flow over deriving the version from the tag or a one-step push, to achieve a single version source and gated release commits, accepting one extra pull request per release.

### Consequences

- Positive: tags can't disagree with the manifest; the changelog section is checked before the tag exists.
- Negative / debt accepted: maintainers need cargo-release installed; the release takes two commands and a merge.

## TR-014: A manual release rehearsal instead of pre-release tags

- Status: accepted
- Date: 2026-10-12
- Phase: plan
- Deciders: project owner
- Links: specs/009-fix-ci-pipeline/plan.md#constitution-check; .specify/memory/constitution.md (1.7.0); specs/009-fix-ci-pipeline/research.md#r2-workflows-and-pipeline-parameters; spec FR-011, FR-012
<!-- key: aa68cd0b67d8 -->

### Context & problem

The release path (five targets, packages, archives, image, publishing) had never run. Proving it on a real tag consumes a version number when it fails. Constitution Principle V says release binaries are built only for release tags.

### Decision drivers

- Prove executors, cross images and packaging before tagging (R15)
- Publish nothing outside a release tag
- Don't burn version numbers

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Manual rehearsal: a pipeline parameter runs the whole build, package and image-test chain on a branch, with no secrets and no publishing (chosen) | Same jobs as a release; no version consumed; can't publish (no context) | Needed a constitution amendment (Principle V, 1.7.0); costs credits per rehearsal |
| Pre-release tags (vX.Y.Z-rc.N) | Within the principle's letter | Consumes version numbers; tag patterns change everywhere |
| Rehearse in a fork | No change to this project | Needs a separate CircleCI project and secrets; doesn't prove this project's executors or plan |

### Decision

We added a manually started rehearsal workflow that builds and tests everything a release does but publishes nothing, and amended Principle V (constitution 1.7.0) so that a manual, non-publishing rehearsal may build release binaries.

### Y-statement

In the context of a never-exercised release path, facing the cost of failed real releases, we chose a manual non-publishing rehearsal over pre-release tags or a fork, to achieve a proven pipeline before tagging, accepting a constitution amendment (Principle V: release binaries are published only for release tags).

### Consequences

- Positive: release problems surface before a version is tagged.
- Negative / debt accepted: costs CI credits per rehearsal. The Principle V deviation was resolved by the constitution amendment to 1.7.0 (2026-10-12).

## TR-015: Positions are judged against entity rectangles, excluding only fully nested ones

- Status: accepted
- Date: 2026-10-13
- Phase: plan
- Deciders: project owner
- Links: specs/010-portable-callsigns/research.md#r5-entity-areas-with-nesting; specs/010-portable-callsigns/spec.md (FR-003); src/web/entity_extents.json
<!-- key: 5986164d6d55 -->

### Context & problem

Feature 010 accepts an online position only if it lies inside the callsign's entity. The only offline area data is one rectangle per entity, bundled for the map's auto-zoom (feature 004), and rectangles overlap. Italy's contains Sardinia's, while neighbours such as Germany and Poland overlap along their border. The clarified rule is that a locator in Cagliari is not inside Italy.

### Decision drivers

- Keep nested entities separate (HR-6): Sardinia, Corsica, San Marino and the like are not their host country
- Never reject a station's own country because of a neighbour's overlapping rectangle
- No new data files or generators in this feature (Principle III)

### Options considered

| Option | Pros | Cons / what it sacrifices |
|--------|------|---------------------------|
| Entity rectangle minus rectangles fully nested inside it (chosen) | Uses existing data; separates Sardinia, Corsica, San Marino, Vatican, Monaco, Slovenia from Italy, Luxembourg from France; neighbours stay apart | Sicily's rectangle is not fully inside Italy's, so northern Sicily still counts as Italy; borders stay coarse |
| Smallest rectangle containing the point wins | Simple | Assigns eastern Germany to Poland (smaller, overlapping rectangle) |
| Overlap threshold (a smaller rectangle that is mostly inside counts as nested) | Would catch Sicily | Also makes Belgium swallow southern Netherlands (about 29% overlap) |
| Entity outlines (simplified polygons) | Correct borders, Sicily included | Larger bundled data and a new generator; out of this feature's scope |

### Decision

We chose the entity rectangle minus the rectangles fully nested inside it, precomputed when the rectangles are loaded.

### Y-statement

In the context of judging online positions against the callsign's entity, facing overlapping rectangles, we chose "rectangle minus fully nested rectangles" over the smallest containing rectangle, an overlap threshold or entity outlines, to keep nested entities separate without hurting neighbours or adding data, accepting that Sicily is only partly separated from Italy and that borders stay coarse.

### Consequences

- Positive: Cagliari is not Italy; Germany and Poland, the Netherlands and Belgium stay apart; no new data.
- Negative / debt accepted: northern Sicily counts as Italy for I/ callsigns. Revisit with entity outlines, a follow-up recorded in spec 010 next to the sub-area positions.

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
