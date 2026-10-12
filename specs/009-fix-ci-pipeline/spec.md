# Feature Specification: Working, Safe and Proven CI Pipeline

**Feature Branch**: `feature/009-fix-ci-pipeline`
**Created**: 2026-10-12
**Status**: Draft
**Input**: User description: "Create a new spec for fixing pipelines. Acquire CIRCLECI-REVIEW.md and
move it into this new spec folder, in the proper directory. Analyze also why the pipeline fails.
There is "circleci" tool installed, use it and acquire info about the last pipeline run, and find
why it fails. Describe the problems and list them, suggest fixes and prepare the spec in order to
fix the pipeline"

**Reference**: [`reference/circleci-review.md`](reference/circleci-review.md) holds the full
review: evidence, line references and suggested fixes for problems 0–17. This spec cites them as
**R0–R17**.

## Problem Summary

Until 2026-10-12 the pipeline **had never completed a single job**. All three runs ever triggered
(`feature/008…`, `main` and tag `v2.0.0`, 2026-10-11) failed after about 20 seconds at the code
checkout step. The project used the legacy OAuth connection, which clones over SSH, and GitHub had no
key for it (R0). As a result, `main` was never gated by CI, and the `v2.0.0` release produced nothing.

R0 was fixed outside the repository on 2026-10-12. The project now has a GitHub App pipeline, which
clones over HTTPS without any key, and the OAuth trigger was removed. The first run through it (run
`5de2ea62…` on `main`) passed `lint`, `test-rust` and `test-tools`. It also exposed the first
failure that checkout had been hiding: `test-web` fails on the CI's Node version (R16). The release
path (builds, packages, image, publishing) has still never run.

| Ref | Problem | Handled by |
|-----|---------|------------|
| R0 | Checkout over SSH rejected: no key on GitHub (**blocker**) | **Resolved** 2026-10-12; US1 keeps it documented |
| R16 | Frontend tests fail on CI: test command not understood by Node 24 (**blocker**) | US1 |
| R17 | Runs start on every branch push; tag pushes not confirmed to start a run | US1 |
| R1 | Publishing secrets readable by every job | US3 |
| R2 | Re-running a release deletes and recreates it | US3 |
| R3 | Build tool downloaded unverified, outdated | US3 |
| R4 | Dependency caches frozen, unbounded, not toolchain-aware | US4 |
| R5 | Packaging tool rebuilt from source in every packaging job | US4 |
| R6 | No caching on the macOS and Windows builds | US4 |
| R7 | Full VM used for a registry-only step | US4 |
| R8 | ARM64 package and ARM64 image never exercised | US5 |
| R9 | 32-bit ARM container image | Out of scope (decided in 008) |
| R10 | Release-candidate image tags never cleaned up | US3 |
| R11 | macOS binary not signed or notarised | Out of scope (decided in 008) |
| R12 | No dependency vulnerability, licence or documentation checks | US5 |
| R13 | No early warning against the newest stable toolchain | US5 |
| R14 | Pre-release tags ignored | Out of scope: replaced by the rehearsal (US2) |
| R15 | Build environments never proven to exist | US2 |

## Clarifications

### Session 2026-10-12

- Q: How should the pipeline get the code? The project is public, and HTTPS is expected. → A: Through
  the CI service's GitHub App, which clones over HTTPS with no SSH or deploy key. This is already
  done: GitHub App pipeline `f5014549-bbc3-4712-901f-e2bd315c9e6e` with trigger `2f62efc0…`, and
  the OAuth trigger `72e306b5…` deleted. The OAuth pipeline itself can't be deleted, and stays
  without triggers.
- Q: When must the pipeline run? → A: The quality gates run on every push to `main` and on every
  pull request. The release builds run only on release tags.
- Q: What happens to the existing `v2.0.0` tag, which points at a commit whose pipeline
  configuration doesn't have these fixes? → A: Leave `v2.0.0` as it is, unreleased, and release
  `v2.0.1` from the commit that completes this feature. (A first answer, moving the `v2.0.0` tag,
  was changed the same day: no existing tag is rewritten.)
- Q: Can the version in the package manifest be taken from the git tag automatically? → A: No, the
  other way round. The manifest stays the single source of truth, and the tag is derived from it:
  `cargo-release` sets the version, commits, and creates and pushes the matching `vX.Y.Z` tag
  (in two steps, see the protected-`main` answer below). Having CI rewrite the manifest from the tag was rejected: it breaks locked builds, and
  `main` would report a wrong version.
- Q: Can a red quality gate block changes to `main`? → A: Yes. `main` is protected: changes arrive
  only through pull requests, and merging needs every quality gate green. The repository admin
  can bypass the rule in an emergency.
- Q: Do pull requests from forks run CI? → A: No. The first answer, "only after the maintainer
  approves each run", turned out to be impossible during planning: the CI service's GitHub App
  integration never builds fork pull requests, and has no approval setting. Instead, after
  reviewing a fork pull request, the maintainer pushes its commits to a branch of this repository
  and opens a pull request from it. The gates run there, and that pull request is the one merged.
- Q: Where do the release notes come from? → A: From a `CHANGELOG.md` in the repository. Every
  release commit updates it, and the pipeline publishes that version's section as the release
  notes.
- Q: How does the release commit reach the protected `main`? → A: In two steps. The first command
  prepares the version bump and changelog on a release branch and opens a pull request. After it
  is merged with green gates, a second command tags the merged commit on `main` and pushes the
  tag. Nothing uses the admin bypass.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every change to `main` and every pull request is checked (Priority: P1)

As the maintainer, every push to `main` and every pull request runs the quality gates (format,
lint, tests for the program, the page and the tools). I learn within minutes whether the change is
releasable, as the constitution requires (Principle V). Pushes to other branches don't start runs
unless they belong to a pull request. A pull request can't be merged into `main` until its gates
are green. A release tag runs the gates first, then the release.

**Why this priority**: until today no gate had ever run, and the page tests still fail on CI
(R16). The current trigger also runs on every branch push, and nobody has confirmed that it runs on
tags at all (R17).

**Independent Test**: push to `main`, open a pull request, push to a branch without a pull
request, and push a test tag. Then check which of them started a run and which jobs it contained.

**Acceptance Scenarios**:

1. **Given** a push to `main`, **When** the run starts, **Then** the code is fetched over HTTPS
   without any key, and all four quality gates run to completion and pass on the current `main`.
2. **Given** a pull request from a branch of this repository, **When** it is opened or new commits
   are pushed to it, **Then** the quality gates run on its latest commit, and the result is shown
   on the pull request. A pull request from a fork starts no run (FR-010).
3. **Given** a push to a branch with no open pull request, **When** it reaches the code host,
   **Then** no run starts.
4. **Given** a push of a release tag `vX.Y.Z`, **When** it reaches the code host, **Then** one run
   starts: the gates first, then the release jobs.
5. **Given** any of these events, **When** it reaches the code host, **Then** exactly one run
   starts for it, not two.
6. **Given** a change that breaks formatting, lint or a test, **When** it is checked, **Then** the
   run fails at that gate, with a message naming the problem.
7. **Given** the page tests, **When** they run on CI and on a developer's machine, **Then** the same
   documented command runs them in both places, and both run the same set of tests.
8. **Given** a pull request whose quality gates are red or still running, **When** someone tries to
   merge it into `main`, **Then** the merge is refused. A direct push to `main` is refused too,
   except for the repository admin's emergency bypass.
9. **Given** the repository connection is later broken, **When** a run fails at checkout, **Then**
   the project's documentation explains how to recognise and restore it in a few steps.

---

### User Story 2 - Prove the release path without publishing (Priority: P1)

As the maintainer, I can start a release rehearsal by hand. It runs every build, package and test
step of a release (all five targets, packages, archives, container image smoke test), but publishes
nothing: no release, no registry push, no tags. It never starts by itself.

**Why this priority**: the release path has never run. A rehearsal finds hidden failures, such as
missing build environments (R15), an outdated cross-build tool (R3) or missing secrets, before a
version tag is cut. A failed real release burns a version number.

**Independent Test**: start a rehearsal on a branch, then check that every build and packaging job
ran and that nothing appeared on the release page or in the image registry.

**Acceptance Scenarios**:

1. **Given** a branch, **When** the maintainer starts a rehearsal, **Then** every release build,
   package, archive and image job runs, and their outputs can be downloaded from the run.
2. **Given** a rehearsal, **When** it finishes, **Then** no GitHub release, image tag or other
   public artefact has been created or changed.
3. **Given** a build environment that the CI service doesn't offer, **When** a rehearsal runs,
   **Then** it fails at that job, and the message names the missing environment.
4. **Given** a rehearsal, **When** it finishes, **Then** its summary states that the publishing
   secrets were not checked, because a rehearsal never receives them (FR-014). The documentation
   gives the command that lists the secrets' names, never their values, so that the maintainer can
   check them before tagging.

---

### User Story 3 - Releases are safe to publish and safe to re-run (Priority: P2)

As the maintainer, publishing credentials are reachable only by the jobs that publish, and only for
release tags. Re-running a release never destroys what was already published. Every tool the
release downloads is verified.

**Why this priority**: today any job could read the image registry and code host tokens, including
a pull request running code that isn't merged yet (R1). Re-running a release would erase its notes
(R2). An unverified download builds every Linux binary (R3).

**Independent Test**: inspect which jobs receive each secret. Then re-run the publishing step of a
completed release and check that its notes and assets are intact. Finally, corrupt a pinned
checksum and check that the build stops.

**Acceptance Scenarios**:

1. **Given** a push to `main`, a pull request or a rehearsal, **When** its jobs run, **Then** none
   of them can read any publishing credential.
2. **Given** a published release whose notes were edited by hand, **When** its publishing step is
   re-run, **Then** the notes are kept and assets are only added or replaced.
3. **Given** a downloaded build tool whose checksum doesn't match the pinned value, **When** the
   build runs, **Then** it stops before using the tool.
4. **Given** a successful release, **When** promotion finishes, **Then** no release-candidate image
   tag for that version remains in the registry (R10).

---

### User Story 4 - Fast and economical runs (Priority: P3)

As the maintainer, repeated runs reuse previous work: dependency caches refresh as the code
changes and are not reused across toolchain versions. Helper tools are not rebuilt in every job, and
lightweight steps don't start full virtual machines.

**Why this priority**: it reduces waiting time and CI credit use, but correctness comes first.

**Independent Test**: run the pipeline twice on unchanged code and compare the durations. Then
change only the toolchain version and check that no cache from the old version is restored.

**Acceptance Scenarios**:

1. **Given** two consecutive runs with no dependency change, **When** the second runs, **Then** it
   restores the cache that the first one saved, including the first run's own build output.
2. **Given** a toolchain version change, **When** a run starts, **Then** it restores no cache built
   by the previous toolchain.
3. **Given** a release, **When** the packaging jobs run, **Then** the packaging tool is built or
   fetched at most once.
4. **Given** the macOS and Windows build jobs, **When** they run a second time, **Then** they reuse
   cached dependencies.

---

### User Story 5 - Releases are tested on every architecture they ship to (Priority: P3)

As a station operator on a Raspberry Pi (ARM64), the package and the container image I install
have been run in CI, not only built. As the maintainer, I'm warned about vulnerable dependencies,
incompatible licences, broken documentation and incompatibilities with the next stable toolchain
before they reach a release.

**Why this priority**: it widens confidence after the pipeline works, but nothing is broken today
that it would fix.

**Independent Test**: on a rehearsal, check that the ARM64 package life-cycle test and the ARM64
image smoke test ran. Then add a dependency with a known advisory in a pull request and check that
the run fails.

**Acceptance Scenarios**:

1. **Given** a release or rehearsal, **When** it runs, **Then** the ARM64 Debian package passes the
   same install, upgrade, save, remove and purge test as the x86-64 one.
2. **Given** a release or rehearsal, **When** the image is built, **Then** the ARM64 image passes the
   same smoke test as the x86-64 one, before anything is promoted.
3. **Given** a dependency with a published vulnerability or a licence not compatible with the
   project's GPL-3.0-or-later licence, **When** a push to `main` or a pull request is checked, **Then** the run fails and
   names the dependency.
4. **Given** documentation warnings, **When** a push to `main` or a pull request is checked, **Then**
   the run fails.
5. **Given** a weekly schedule, **When** it fires, **Then** the quality gates run on the newest
   stable toolchain. A failure there doesn't block releases, but it is visible.

---

### User Story 6 - Cut a release in two documented steps (Priority: P2)

As the maintainer, I cut a release in two documented `cargo-release` steps, and I type the version
only once.
1. **Prepare:** the first command sets the new version in the package manifest and the lock
   file, turns the changelog's "Unreleased" entries into the version's section, commits this on a
   release branch, and opens a pull request into `main`.
2. **Tag:** once that pull request is merged with green gates, the second command creates the
   matching `vX.Y.Z` tag on the merged commit of `main` and pushes it, which starts the release.

The release commit goes through the same checks as any other change, and nothing needs the admin
bypass.

**Why this priority**: a mismatch between the tag and the manifest fails the release at the
version gate. Today, avoiding it depends on editing the manifest and typing the tag by hand
identically. The 2.0.1 release (FR-028) is the first one cut this way.

**Independent Test**: run both steps without their "execute" confirmation, and check that they
report what they would do without changing anything. Then cut 2.0.1 with them, and check that the
version gate passes on the first run.

**Acceptance Scenarios**:

1. **Given** an up-to-date, clean `main` whose changelog has entries under "Unreleased", **When**
   the maintainer runs the prepare step for a patch, minor or major bump, **Then** a release branch
   holds one commit with the new version in the manifest and the lock file and the dated (UTC)
   changelog section, and a pull request into `main` is opened for it. No tag is created.
2. **Given** the release pull request has been merged with green gates, **When** the maintainer runs
   the tag step on the updated `main`, **Then** a `vX.Y.Z` tag matching the manifest version points
   at the merged commit, and it is pushed.
3. **Given** either step without its "execute" confirmation, **When** it runs, **Then** it only
   shows what it would do, and changes nothing.
4. **Given** either step, **When** it runs, **Then** it doesn't publish anything to a package
   registry, and doesn't push to `main`. Publishing is the pipeline's job, and `main` changes only
   through the merged pull request.
5. **Given** uncommitted changes, an out-of-date `main`, or a branch other than `main`, **When** a
   step runs, **Then** it refuses, and says why.
6. **Given** a `main` whose manifest version already has a tag, or whose changelog has no section
   for that version, **When** the tag step runs, **Then** it refuses, and says why.
7. **Given** a tag created by the tag step, **When** the pipeline's version gate checks it,
   **Then** the gate passes.
8. **Given** a published release, **When** its page is opened on the code host, **Then** its notes
   are exactly that version's changelog section.

---

### Edge Cases

- **Repository connection broken later**: if the GitHub App loses access to the repository, runs
  stop starting or fail at checkout. User Story 1 scenario 8 covers how to recognise and restore
  the connection.
- **OAuth trigger recreated by mistake**: every event would start two runs again, and the OAuth one
  would fail over SSH. FR-006 forbids it, and the documentation (FR-008) says why.
- **Tag pushes and the branch trigger**: a release tag must start exactly one run. If the
  "pushes" trigger doesn't cover tags, a separate tag trigger is needed. If it does cover them, a
  separate one would create duplicates (FR-005, FR-006).
- **Further failures behind the first ones**: later jobs may fail for reasons never seen before
  (missing environments, outdated tool images, missing secrets). The rehearsal (US2) must be green
  before any release tag is cut.
- **Pull requests from forks**: they start no run and never receive secrets (FR-010, FR-014). Their
  required gates never report, so they can't be merged as they are (FR-009). The maintainer
  reviews the code, pushes it to a branch of this repository (keeping the contributor as author),
  opens a pull request from that branch, and closes the fork pull request with a link to it.
- **New commits to a fork pull request after the re-push**: they reach the repository only when the
  maintainer reviews and re-pushes them to the internal branch.
- **Pushing to a pull request's branch**: the gates run once, for the pull request. The branch push
  must not start a second run.
- **The unreleased `v2.0.0` tag**: it stays on the code host with no release, packages or image.
  The release list goes from 1.0.0 to 2.0.1. The 2.0.1 changelog section MUST say that 2.0.0 was
  never published, because of the CI failure, and that 2.0.1 contains the 2.0.0 program plus the
  pipeline fixes.
- **Tag with no changelog section**: a hand-made tag whose version has no section in the
  changelog MUST fail before any build, like a version mismatch. No release is published with
  empty notes.
- **Release re-run after a partial failure**: if publishing failed after the release-candidate
  image was pushed, re-running must complete the release, not fail on the existing candidate or
  the existing release.
- **Rehearsal on a commit whose version doesn't match any tag**: the version gate only applies to
  real release tags. A rehearsal must not require a tag.
- **Other changes merged between the two steps**: the tag step tags the commit of `main` it runs
  on, as long as its manifest version is the prepared one and has no tag yet. Changes merged after
  the release pull request are included in the release, but not in its changelog section. The
  documentation tells the maintainer to tag right after the merge.
- **Release pull request abandoned**: closing it without merging leaves `main` untouched. The
  release branch can be deleted, and the prepare step can run again.
- **Gate names change**: if a quality-gate job is renamed, the protection rule must be updated in
  the same change. Otherwise merges wait forever for a check that no longer exists.
- **Release step interrupted**: if a push fails after the local commit or tag was made, the
  maintainer can push again, or delete the local branch or tag and run the step again. The tag
  always points at a commit already on `main`, so a tag can't reach the code host without its
  version commit.
- **Tag created by hand**: still possible. The version gate stays in the pipeline as the second
  check, so a hand-made tag that doesn't match the manifest stops before any build.
- **Scheduled newest-toolchain run fails**: the result is reported, but the pinned toolchain isn't
  changed automatically. Raising the pin stays a deliberate change (`docs/rust-guidelines.md`
  §2.2).

## Requirements *(mandatory)*

### Functional Requirements

**Fetch the code and decide when to run (R0, R16, R17)**

- **FR-001**: The CI service MUST fetch the repository over HTTPS, through its GitHub App
  connection, with no SSH or deploy key.
- **FR-002**: Every push to `main` MUST start a run of the quality gates (format, lint, program
  tests, page tests, tool tests). After this feature, that run MUST be green on the current `main`.
- **FR-003**: Opening a pull request, and every push to it, MUST start a run of the quality gates
  on its latest commit. The result MUST be reported on the pull request. Fork pull requests are
  handled by FR-010.
- **FR-004**: A push to a branch other than `main` with no open pull request MUST NOT start a run.
- **FR-005**: The release jobs (version gate, builds, packages, archives, image, publishing,
  promotion) MUST run only for tags matching `vX.Y.Z`, after the quality gates on the same commit
  have passed. The only exception is the manual rehearsal (FR-011), which publishes nothing.
- **FR-006**: Each event (push to `main`, pull request update, release tag) MUST start exactly one
  run. The legacy OAuth pipeline MUST have no trigger.
- **FR-007**: The page tests MUST run on CI with the same documented command a developer uses
  locally, and that command MUST work on the Node version used by CI. Both MUST run every page
  test file.
- **FR-008**: The project documentation MUST describe the CI service's setup outside the
  repository: the GitHub App connection, the pipeline and its triggers, the publishing secrets and
  their scope, and the protection rule on `main` (FR-009). It MUST also describe how to diagnose a run that doesn't start or that fails at
  checkout.

- **FR-009**: `main` MUST be protected on the code host. Changes reach it only through pull
  requests, a merge requires every quality gate of FR-002/FR-003 to have passed on the pull
  request's latest commit, and direct pushes are refused. Only the repository admin may bypass
  the rule, for emergencies.

- **FR-010**: A pull request from a fork MUST NOT start any run, and MUST NOT be merged directly. Its
  changes reach `main` only through a pull request from a branch of this repository, opened by the
  maintainer after reviewing them. The documentation (FR-008) MUST describe these steps.

**Rehearse releases (R15)**

- **FR-011**: The maintainer MUST be able to start a rehearsal by hand, on any branch. It runs every
  release build, package, archive and image job, with the same steps as a real release. A
  rehearsal MUST never start automatically.
- **FR-012**: A rehearsal MUST NOT publish, push, tag, promote or delete anything outside the run
  itself. Its outputs MUST be downloadable from the run.
- **FR-013**: On a release tag, the release path MUST check that each publishing secret is present
  before any build starts. A missing one MUST be reported by name, without revealing any value. A
  rehearsal receives no secrets, so it MUST NOT claim to have checked them (US2 scenario 4).

**Secure releases (R1, R2, R3, R10)**

- **FR-014**: Publishing credentials (image registry, code host) MUST be available only to the jobs
  that publish, and those jobs run only for release tags. Jobs for `main`, pull requests (including
  forks) and rehearsals MUST NOT receive them.
- **FR-015**: Re-running a release MUST keep the existing release and its notes, and only add or
  replace assets.
- **FR-016**: Every tool the pipeline downloads outside a package manager MUST be pinned to an exact
  version and verified against a recorded checksum before use.
- **FR-017**: The cross-build tool MUST be a version that builds all three Linux targets with the
  pinned toolchain. A rehearsal proves this (FR-011).
- **FR-018**: After a successful promotion, the release-candidate image tag for that version MUST
  be removed from the registry.

**Faster runs (R4–R7)**

- **FR-019**: Dependency caches MUST be keyed by the toolchain version and the lock file. They
  MUST be refreshed by every run, so that a run restores the latest build output for its key.
- **FR-020**: The packaging tool MUST be obtained at most once per release, not once per
  architecture.
- **FR-021**: The macOS and Windows builds MUST cache dependencies, under the same keying rules as
  FR-019.
- **FR-022**: Steps that only talk to the image registry MUST NOT need a full virtual machine.

**Wider checks (R8, R12, R13)**

- **FR-023**: The ARM64 Debian package MUST pass the same life-cycle test as the x86-64 one, on
  ARM64 hardware.
- **FR-024**: The ARM64 container image MUST pass the same smoke test as the x86-64 one, before
  promotion.
- **FR-025**: Every quality-gate run MUST check for dependencies with published vulnerabilities and
  licences not compatible with the project's GPL-3.0-or-later licence, and for documentation warnings. Any finding fails the
  run.
- **FR-026**: A weekly scheduled run MUST execute the quality gates on the newest stable toolchain,
  reported separately from the pinned-toolchain gates and never blocking a release.

**Unchanged**

- **FR-027**: The release outputs (file names, contents, packages, image tags `<version>` and
  `latest`, checksums file) and the release order (test, build, package, publish, promote) MUST
  stay as defined by feature 008.

**Release 2.0.1**

- **FR-028**: The first release built by the fixed pipeline MUST be version 2.0.1. It is cut with
  the two release steps (FR-029) once this feature is merged: the prepare step sets the version to
  `2.0.1`, and the tag step creates `v2.0.1` on the merged release commit. The existing `v2.0.0` tag MUST NOT be moved, deleted or
  re-run. It stays as a tag with no release.

**Release command (option chosen 2026-10-12)**

- **FR-029**: A release MUST be cut with `cargo-release`, configured in the repository, in two
  steps. The **prepare** step sets the version in the package manifest and the lock file, updates
  the changelog (FR-031), commits on a release branch and opens a pull request into `main`. The
  **tag** step, run on `main` after that pull request is merged, creates the `vX.Y.Z` tag on the
  merged commit and pushes only the tag. Neither step publishes to a package registry or pushes
  to `main`. Both run only from a clean `main` that the documented steps first bring up to date
  (`git pull --ff-only`), and without explicit confirmation they
  only show what they would do.
- **FR-030**: The package manifest MUST stay the single source of the version. The pipeline MUST
  NOT rewrite it, and the version gate (tag equals manifest version) MUST stay.
- **FR-031**: The repository MUST keep a `CHANGELOG.md`, with an "Unreleased" section at the top
  and one section per tagged version, newest first, each dated in UTC. The prepare step
  MUST turn "Unreleased" into the new version's section in the release commit, and refuse to run
  when "Unreleased" is empty.
- **FR-032**: The pipeline MUST publish the tag's changelog section as the release notes. The
  release gate MUST fail before any build if that section is missing or empty. A re-run keeps
  the notes already on the release (FR-015).
- **FR-033**: Both release steps MUST be documented next to the release process, including how
  to install `cargo-release`. It MUST NOT be needed to build, test or lint the project.

### Key Entities

- **Pipeline run**: one execution, started by a push to `main`, a pull request, a release tag, the
  weekly schedule or a manual rehearsal. Its kind (gate, release, scheduled, rehearsal) decides which
  jobs run and which secrets they receive.
- **Trigger**: the rule on the CI service that turns a code-host event into a run. There are two,
  both on the GitHub App pipeline: one event trigger covering pushes to `main`, pull requests and
  tags, and one weekly schedule.
- **Repository connection**: the CI service's GitHub App installation that grants read access to
  this repository. It replaces the legacy OAuth connection and its SSH key.
- **Publishing secret**: a credential for the image registry or the code host's releases. It is
  scoped to publishing jobs on release tags.
- **Release candidate**: the image pushed before the GitHub release, then promoted and removed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of pushes to `main` and of pull request updates after the fix produce exactly
  one run, in which all quality gates run to completion. None of them fails at checkout.
- **SC-002**: Pushes to branches with no pull request produce zero runs.
- **SC-003**: After this feature, no commit reaches `main` without a green gate run on its pull
  request, apart from documented admin bypasses.
- **SC-004**: The page tests report the same number of tests on CI as on a developer's machine.
- **SC-005**: A rehearsal on the fixed branch is green on all five targets, all three packages, both
  archives and the image, and publishes zero artefacts.
- **SC-006**: The first real release after the fix, `v2.0.1`, completes end to end on the first
  attempt: every artefact from feature 008 is published, and the image is available under
  `<version>` and `latest`.
- **SC-007**: No job outside a release tag's publishing jobs has access to any publishing
  credential. An inspection of every job's configuration confirms it.
- **SC-008**: Re-running the publishing step of a completed release leaves its hand-edited notes
  unchanged.
- **SC-009**: A second gate run with no dependency change finishes at least 30% faster than a
  cold-cache run.
- **SC-010**: The ARM64 package and the ARM64 image are each exercised by at least one test in
  every release and rehearsal.
- **SC-011**: After a release, the registry holds no release-candidate tag for that version.
- **SC-012**: Cutting a release takes two documented commands and one merged pull request, and the
  version is typed once. Every release from
  2.0.1 on passes the version gate on its first run.

## Assumptions

- **Repository connection**: the move to the GitHub App was done on 2026-10-12, through the CI
  service's settings and command-line tool. The project keeps its `gh/sardylan/live-qso-map` slug,
  and the organisation has both integrations (OAuth and GitHub App). Removing OAuth from the
  organisation entirely would mean creating a new CI organisation, so it isn't required.
- **Trigger configuration lives on the CI service**: which events start a run (FR-002 to FR-006)
  is set on the CI service's triggers, not only in the pipeline configuration file. The plan must
  describe both, and the documentation (FR-008) records the triggers.
- **"Pull request"**: this is GitHub's name for what other hosts call a merge request. Draft pull
  requests are checked as well.
- **The CI service, code host and registry stay the same**: CircleCI, GitHub and Docker Hub,
  as chosen in feature 008.
- **Rehearsal instead of pre-release tags** (R14): a rehearsal started by hand proves the release
  path without consuming version numbers. It isn't a "real build", because it publishes nothing, so
  it doesn't conflict with "release builds only on tags".
- **Earlier decisions stand**: the container image targets x86-64 and ARM64 only (008
  clarification Q2, R9). The macOS archive stays unsigned and explains the "Open anyway" step (008
  assumptions, R11).
- **32-bit ARM package test**: there is no native 32-bit ARM executor, so the armhf package keeps
  its static checks (architecture, version, contents). Emulated life-cycle tests are not required.
- **ARM64 executors**: the CI plan offers ARM64 executors (Docker or machine) for FR-023 and
  FR-024.
- **Newest-toolchain run**: weekly, on `main`, as a scheduled trigger of the CI service.
- **Changelog format**: the "Keep a Changelog" layout (Added / Changed / Fixed … under each
  version), in English, with UTC dates (`docs/ham-radio.md` §1: UTC everywhere). It starts with
  "Unreleased", then 2.0.1 once released, and a 2.0.0 section that says the tag was never released
  (CI failure). Versions up to 1.0.0 aren't back-filled; a line points to the code host's
  history for them.
- **Release tool**: `cargo-release` is a maintainer-only tool, installed with Cargo like any other
  Cargo subcommand. It runs on Linux, macOS and Windows. Contributors don't need it, so
  Principle V ("only `cargo`, `node` and `python3` to build, test and lint") is unaffected. The
  plan pins its version in the documentation.
- **Constitution Principle V**: the gates still run on every commit to `main`, and release builds
  still run only for release tags (plus rehearsals, which don't publish). Cross-compilation still
  happens only in the pipeline. No developer workflow changes. Branch pushes without a pull request
  are no longer checked; the constitution doesn't require it.
