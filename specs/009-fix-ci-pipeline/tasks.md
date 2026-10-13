---
description: "Task list for 009-fix-ci-pipeline"
---

# Tasks: Working, Safe and Proven CI Pipeline

**Input**: Design documents from `/specs/009-fix-ci-pipeline/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the code it covers,
run it, and confirm it fails first.
- **CI scripts:** each is tested by its own `scripts/test-*.sh` (POSIX `sh`, like
  `scripts/test-check-version.sh`).
- **Changelog:** tested by `tests/changelog.rs` (`cargo test`).
- **Pipeline behaviour:** a CI config can't be unit-tested. Its checks are
  `circleci config validate` / `circleci config process` after each config task, and the
  verification tasks marked **Verify**, which follow `quickstart.md` against CircleCI.

**Conventions:**
- **Config:**
  - `.circleci/config.yml` is shared by almost every story, so its tasks run one at a time,
    never [P];
  - after each one, run `circleci config validate .circleci/config.yml`;
  - follow `contracts/pipeline.md` for job names, executors, contexts and requirements.
- **CI service and GitHub settings:** they are outward-facing. Tasks marked **[ext]** change
  CircleCI or GitHub and MUST be confirmed by the maintainer before running. Their exact commands
  are in `contracts/ci-service-setup.md`.
- **Integration tests:** they return `Result<(), Box<dyn std::error::Error>>` and use `?`, never
  `unwrap`/`expect`/`panic!`, and need no attribute (no inline lint suppressions, as in 008).
- **Pinned versions** (research, 2026-10-12):
  - cross 0.2.5
  - cargo-deb 3.8.0
  - cargo-deny 0.20.2
  - gh 2.102.0
  - crane 0.22.1
  - cargo-release 1.1.6 (maintainer only)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US6)

---

## Phase 1: Setup

- [X] T001 Record the current CircleCI and GitHub state in `specs/009-fix-ci-pipeline/reference/circleci-review.md` (append a short "State before implementation" note):
  - `circleci project trigger list` for both pipeline definitions (`f5014549-…`, `baf57313-…`);
  - `gh api repos/sardylan/live-qso-map/rulesets`;
  - `circleci project envvar list`. Note the result even if it's a 403.
- [X] T002 [P] Compute and record the SHA-256 of every pinned download, in a table at the top of `.circleci/config.yml` (comments) and as command parameters:
  - **cross:** v0.2.5 `cross-x86_64-unknown-linux-musl.tar.gz`. It has no published checksum: download once, run `sha256sum`, and note "computed 2026-10-12".
  - **cargo-deny:** 0.20.2 `cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz`, compared with its `.sha256` file.
  - **gh:** `gh_2.102.0_linux_amd64.tar.gz`, from `gh_2.102.0_checksums.txt`.
  - **crane:** `go-containerregistry_Linux_x86_64.tar.gz` 0.22.1, from `checksums.txt`.
  - **rustup-init:** for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`, from the pinned rustup version's `rustup-init.sha256` on static.rust-lang.org.

- [X] T003 Amend `.specify/memory/constitution.md` Principle V (`/speckit-analyze` finding C1):
  - **Rule:** "MUST build release binaries … only for release tags" becomes "MUST publish release binaries … only for release tags. A manually started rehearsal MAY build and test them on any branch, provided it publishes nothing and receives no publishing secrets".
  - **Version:** 1.6.0 → 1.7.0, Last Amended 2026-10-12.
  - **Sync Impact Report:** a new one at the top of the file.

  It MUST be done before Phase 4 (US2).

---

## Phase 2: Foundational (blocking prerequisites)

- [X] T004 Add the pipeline parameters `rehearsal` (boolean, `false`) and `newest-toolchain` (boolean, `false`) next to `rust-version` in `.circleci/config.yml`. Give the existing workflow `test_and_build` the condition `when: { and: [ { not: << pipeline.parameters.rehearsal >> }, { not: << pipeline.parameters.newest-toolchain >> } ] }`, and validate.
- [X] T005 Add the reusable command `fetch-verified` (parameters `url`, `sha256`, `dest`; `curl -fsSL`, then `echo "<sha256>  <dest>" | sha256sum -c -`, failing before the file is used) in `.circleci/config.yml`, per `contracts/pipeline.md` "Commands". Then validate.
- [X] T006 Rewrite `install-rustup` in `.circleci/config.yml` to download the host's `rustup-init` through `fetch-verified` (T002 checksums) instead of `curl https://sh.rustup.rs | sh`. Do the same in `build-windows` (the `rustup-init.exe` download), then validate.

**Checkpoint**: the config validates, and every existing job still has the same steps, apart from
verified downloads.

---

## Phase 3: User Story 1 - Every change to `main` and every pull request is checked (Priority: P1) 🎯 MVP

**Goal**:
- a push to `main` or a pull request runs the gates, which are green on `main`;
- a branch without a pull request runs nothing, and a tag runs the gates and then the release;
- exactly one run per event;
- `main` is protected.

**Independent Test**: `quickstart.md` §2 (event table) and §3 (ruleset).

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T007 [US1] Reproduce R16 before fixing it, recording the outputs in the task's commit message: run `node --test web/` on a Node 24 runtime and confirm it fails with `Cannot find module …/web`. If no local Node 24 exists, use `docker run --rm -v "$PWD":/w -w /w node:24 node --test web/`, or cite job `9296c78a-1ab8-41c5-aaa2-26d4bde853a5`. Then confirm `node --test 'web/*.test.js'` passes on Node 24 and 26 with the same test count.
- [X] T008 [US1] Write `tools/test_ci_config.py`, a regression test for R16 (unittest, stdlib only, reading files as text so it runs on any host). `test_page_tests_use_portable_glob` asserts that:
  - `.circleci/config.yml` contains `node --test 'web/*.test.js'` and no `node --test web/`;
  - `README.md` documents the same command.

  Run `python3 -m unittest tools/test_ci_config.py` and confirm it FAILS.

### Implementation for User Story 1

- [X] T009 [US1] Change the `test-web` job command to `node --test 'web/*.test.js'` in `.circleci/config.yml`. Change the `test-tools` command to `python3 -m unittest discover -s tools -p 'test_*.py'`, so that `tools/test_ci_config.py` runs next to `test_entity_extents.py`. Validate.
- [X] T010 [P] [US1] Change the documented page-test command to `node --test 'web/*.test.js'` in `README.md` ("Development" block), with the comment "frontend logic, no npm packages; quoted so Node expands it". With T009, this makes T008 pass.
- [X] T011 [P] [US1] Create `deploy/github/main-ruleset.json` per `contracts/ci-service-setup.md` "Ruleset on main". It lists the four existing gates only (`ci/circleci: lint`, `test-rust`, `test-web`, `test-tools`); `audit` is added in T053.
- [X] T012 [P] [US1] Write `docs/ci.md`, sections:
  1. **Overview:** run kinds, from `data-model.md`.
  2. **Repository connection:** GitHub App, HTTPS, no keys, plus the OAuth pipeline that can't be deleted.
  3. **Triggers:** with the `circleci` commands.
  4. **Protection of `main`:** the ruleset file, applying and updating it, renaming gates.
  5. **Pull requests from forks:** the re-push procedure (FR-010).
  6. **Diagnosis:**
     - no run starts;
     - two runs per event;
     - checkout over SSH;
     - a red gate.
  7. **Local reproduction:** what runs on any host (`cargo`, `node`, `python3`), and what is CI-only:
     - the `scripts/*.sh` helpers, which need a POSIX shell;
     - `audit`, which needs cargo-deny 0.20.2 (`cargo install cargo-deny --version 0.20.2 --locked`, optional).

  Sections for secrets, rehearsal and the weekly run are added by T028, T022 and T055. Link it from `README.md`.
- [X] T013 [US1] [ext] Replace the event trigger: create the `only-build-prs` GitHub App trigger, then delete trigger `2f62efc0-51a0-461d-96c2-f5750aea445d` (commands in `contracts/ci-service-setup.md`). Confirm with `circleci project trigger list --pipeline-definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e`.
- [X] T014 [US1] Push the branch, open the feature pull request into `main`, and check that its run executes all four gates green, checking out over HTTPS (job step 101).
- [X] T015 [US1] [ext] Apply `deploy/github/main-ruleset.json` with `gh api -X POST repos/sardylan/live-qso-map/rulesets --input deploy/github/main-ruleset.json`, after T014 is green. That way the required contexts exist, and the feature pull request itself is mergeable. Record the ruleset id in `docs/ci.md`. Check `bypass_actors` and the admin role id (research R9).
- [X] T016 [US1] **Verify** `quickstart.md` §2 (branch without a pull request: no run; pull request open and push: one run each; draft pull request: one run; tag `v0.0.0`: gates and failing `check-version`, no build) and §3 (direct push to `main` refused for a non-bypass actor; merging blocked while a gate is red). Record the results in `specs/009-fix-ci-pipeline/reference/circleci-review.md` (item 17 resolution). If the draft pull request starts no run, change the spec's assumption "Draft pull requests are checked as well" to "checked once marked ready for review", and record it in research R1.

**Checkpoint**: CI gates `main` and pull requests, which is the MVP. Releases still use the old
chain.

---

## Phase 4: User Story 2 - Prove the release path without publishing (Priority: P1)

**Goal**: a manual rehearsal runs every build, package, archive and image-test job, and publishes
nothing.

**Independent Test**: `quickstart.md` §4.

### Tests for User Story 2

- [X] T017 [US2] Before T018–T021, write the expected job list in `specs/009-fix-ci-pipeline/contracts/pipeline.md` ("Rehearsal workflow"). For now it holds the amd64 packages and the amd64 image test only; arm64 is added in US5. Expected job names after T021 (US5 adds the arm64 `test-deb-*` jobs and `image-test-arm64`):
  - `build-linux-x86_64-unknown-linux-musl`, `build-linux-aarch64-unknown-linux-musl`, `build-linux-armv7-unknown-linux-musleabihf`
  - `build-macos`, `build-windows`
  - `package-deb-x86_64-unknown-linux-musl`, `package-deb-aarch64-unknown-linux-musl`, `package-deb-armv7-unknown-linux-musleabihf`
  - `test-deb-debian-12-amd64`, `test-deb-debian-13-amd64`, `test-deb-ubuntu-22-04-amd64`, `test-deb-ubuntu-24-04-amd64`
  - `package-archives`, `image-test-amd64`, `rehearsal-summary`

  The check, recorded in `quickstart.md` §4:

  ```
  p=$(circleci config process .circleci/config.yml --pipeline-parameters 'rehearsal: true' | sed -n '/^  rehearsal:$/,/^  [a-z_]*:$/p')
  for j in <names above>; do printf '%s\n' "$p" | grep -Eq "(name: |- )$j:?\$" || echo "missing: $j"; done
  ```

  Also read the `rehearsal` workflow and check that it lists none of `check-version`, `image-push`, `publish-github-release` or `docker-promote`. Confirm the check reports missing jobs now (there is no rehearsal workflow yet).

### Implementation for User Story 2

- [X] T018 [US2] Split `docker-image` in `.circleci/config.yml` into `image-test` (job parameter `arch`, `--load` build for `linux/<arch>` + `scripts/test-image.sh`; for now only `amd64`, on `linux-machine`) and `image-push` (the multi-platform `-rc` push, requires `image-test-amd64`). Keep `docker-promote` requiring `publish-github-release`. Validate.
- [X] T019 [US2] (Implemented as the reusable command `set-release-version`, used by these jobs, instead of a job parameter: it uses the tag when there is one, otherwise `Cargo.toml`. Both workflows share the jobs unchanged.) Add a job parameter `version-source` (`tag` | `cargo`) to `build-linux`, `package-deb`, `package-archives` in `.circleci/config.yml`:
  - with `cargo`, the version is read from `Cargo.toml`, with the suffix `-rehearsal.<short sha>` in archive names (`contracts/pipeline.md`);
  - the `.deb` version check compares with `Cargo.toml` instead of `CIRCLE_TAG`.

  Then validate.
- [X] T020 [US2] Add the job `rehearsal-summary` in `.circleci/config.yml`:
  1. attaches the workspace;
  2. runs `ls -l ws/release`;
  3. stores `ws/release` as artefacts;
  4. prints `publishing secrets: not checked (rehearsal)`.

  It uses no context. Validate.
- [X] T021 [US2] Add the workflow `rehearsal` (`when: << pipeline.parameters.rehearsal >>`) in `.circleci/config.yml`:
  - **Jobs:** `build-linux` ×3, `build-macos`, `build-windows`, `package-deb` ×3, `test-deb` amd64 ×4, `package-archives`, `image-test-amd64`, `rehearsal-summary` (requires all).
  - **Settings:** `version-source: cargo`, no filters, no context, no `check-version`, `image-push`, `publish-github-release` or `docker-promote`.
  - **Check:** run T017's command and confirm the job list matches.
- [X] T022 [P] [US2] Add a "Rehearsal" section to `docs/ci.md`:
  - how to start it (`circleci pipeline run --definition-id f5014549-… --branch <b> --param rehearsal=true`, and the API fallback with a JSON boolean from `quickstart.md` §4);
  - what it proves;
  - that it never publishes, and doesn't check secrets.
- [X] T023 [US2] **Verify** `quickstart.md` §4 on this branch: the rehearsal is green, artefacts are stored, and `gh release list` and `crane ls sardylan/live-qso-map` are unchanged.
  - **If an executor or cross image fails:** fix it here (R11 fallback, R15) and record it in `research.md`.
  - **If `--param rehearsal=true` doesn't select the workflow:** switch `docs/ci.md` to the API form.

**Checkpoint**: the whole release path has been proven once, without publishing.

---

## Phase 5: User Story 3 - Releases are safe to publish and safe to re-run (Priority: P2)

**Goal**:
- secrets go only to the four tag-only publishing jobs, and are checked before any build;
- re-runs keep the release notes;
- downloads are verified;
- the `-rc` image tag is removed after promotion.

**Independent Test**: `quickstart.md` §6. At the first release (T062): a notes-preserving re-run
and no `-rc` tag.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T024 [P] [US3] Write `scripts/test-check-secrets.sh`, in the style of `scripts/test-check-version.sh`. It covers:
  - every variable set: exit 0, and the output names them as present;
  - one unset and one empty: exit 1, and stderr names both;
  - a value containing `s3cr3t` never appears in stdout or stderr;
  - no argument: exit 1, with usage.

  Run it and confirm it fails, because the script doesn't exist yet.

### Implementation for User Story 3

- [X] T025 [US3] Write `scripts/check-secrets.sh` (POSIX `sh`, `set -u`, reads variables through `eval "v=\${$name-}"` without echoing values) to make T024 pass.
- [X] T026 [US3] Add `sh scripts/test-check-secrets.sh` to the `test-tools` gate. Move `sh scripts/test-check-version.sh` from `check-version` into `test-tools` too (`contracts/pipeline.md`). Validate.
- [X] T027 [US3] In `.circleci/config.yml`:
  - give `check-version`, `image-push`, `publish-github-release` and `docker-promote` `context: live-qso-map-release` (in the workflow entries), and no other job;
  - make `check-version` run `sh scripts/check-secrets.sh GITHUB_TOKEN DOCKERHUB_USERNAME DOCKERHUB_TOKEN` after the version gate;
  - make `cargo-deb` and every build job require `check-version` on tags.

  Validate, and check `circleci config process … | grep -c 'context: live-qso-map-release'` = 4.
- [X] T028 [P] [US3] Add a "Secrets" section to `docs/ci.md`:
  - the context, its two restrictions and the token scopes (`contracts/ci-service-setup.md`);
  - listing the variable names (`circleci context env-var list`);
  - rotating the tokens.

  In `README.md`, replace the "CircleCI project needs these variables" table with a pointer to it.
- [X] T029 [US3] [ext] Create the context `live-qso-map-release` with its project and expression restrictions. The maintainer enters the three values: a fine-grained GitHub token (this repository only, `contents: write`) and a Docker Hub personal access token (Read/Write/Delete). Delete any project-level variables with these names. Check with `quickstart.md` §6.
- [X] T030 [US3] Add `test_release_rerun_keeps_notes` to `tools/test_ci_config.py`, a regression test for R2. It asserts that the config:
  - contains no `ghr` and no ` -delete`;
  - contains `gh release view` before `gh release create` with `--verify-tag` and `--notes-file`;
  - contains `gh release upload` with `--clobber`.

  Run it and confirm it FAILS.
- [X] T031 [US3] Replace `publish-github-release` in `.circleci/config.yml`:
  - **Executor:** `cimg/base:current`.
  - **Tool:** install gh 2.102.0 through `fetch-verified`.
  - **Steps:**
    1. `SHA256SUMS`;
    2. `sh scripts/changelog-section.sh "${CIRCLE_TAG#v}" > notes.md` (the script comes in T037);
    3. `gh release view "$CIRCLE_TAG"`, then either `gh release create "$CIRCLE_TAG" --verify-tag --title "$CIRCLE_TAG" --notes-file notes.md ws/release/*` or `gh release upload "$CIRCLE_TAG" --clobber ws/release/*` (research R5).
  - **Remove** the `github` executor and `ghr`.

  Validate, and confirm T030 passes.
- [X] T032 [US3] Replace `docker-promote` in `.circleci/config.yml`:
  - **Executor:** `cimg/base:current`.
  - **Tool:** install crane 0.22.1 through `fetch-verified`.
  - **Promote:** `crane auth login index.docker.io -u "$DOCKERHUB_USERNAME" --password-stdin`; `crane tag sardylan/live-qso-map:${v}-rc ${v}`; `crane tag … latest`.
  - **Delete the `-rc` tag:** get a token from `POST https://hub.docker.com/v2/auth/token`, then `DELETE https://hub.docker.com/v2/namespaces/sardylan/repositories/live-qso-map/tags/${v}-rc`.
  - **Check:** fail if `crane ls` still lists `${v}-rc`, and never print the token.

  This also covers FR-022 (no VM). Validate.
- [X] T033 [US3] Install cross 0.2.5 in `build-linux` through `fetch-verified` (T002). In `Cross.toml`, pin each of the three targets' images by digest (`[target.<triple>] image = "ghcr.io/cross-rs/<triple>:0.2.5Tsha256:…"`; get the digests with `crane digest ghcr.io/cross-rs/<triple>:0.2.5`), with a comment naming the date. Validate.
- [X] T034 [P] [US3] Update `specs/009-fix-ci-pipeline/reference/circleci-review.md` items 1, 2, 3 and 10, each with a "Resolution" line naming the task.

**Checkpoint**: on a tag, secrets reach only the publishing jobs. Publishing and promotion are
re-runnable, and every download is verified.

---

## Phase 6: User Story 6 - Cut a release in two documented steps (Priority: P2)

**Goal**: `CHANGELOG.md` drives the release notes. cargo-release prepares a release pull request,
then tags the merged commit. The version is typed once.

**Independent Test**: `quickstart.md` §1 (changelog checks) and §5 (dry run and refusals).

### Tests for User Story 6 (write first, confirm they FAIL)

- [X] T035 [P] [US6] Write `tests/changelog.rs`. It reads `CHANGELOG.md` from `env!("CARGO_MANIFEST_DIR")`, and asserts:
  - exactly one `## [Unreleased]` heading, before any version heading;
  - a heading `## [<CARGO_PKG_VERSION>] - YYYY-MM-DD` (date shape `\d{4}-\d{2}-\d{2}`, checked with `std` only);
  - that section has at least one `### ` subsection and one `- ` entry before the next `## `;
  - version headings are unique.

  Return `Result<(), Box<dyn std::error::Error>>`. Run `cargo test --test changelog` and confirm it fails, because `CHANGELOG.md` is missing.
- [X] T036 [P] [US6] Write `scripts/test-changelog-section.sh`. It covers:
  - printing the body of a middle section and of the last section;
  - `2.0.1` vs `2.0.10` not confused;
  - a missing version: exit 1;
  - a section with headings but no `- ` entry: exit 1;
  - CRLF line endings handled;
  - the `[Unreleased]` section never returned for a version.

  Run it and confirm it fails.

### Implementation for User Story 6

- [X] T037 [US6] Write `scripts/changelog-section.sh` (POSIX `sh` + `awk`; usage `changelog-section.sh <X.Y.Z> [CHANGELOG.md]`, prints the body without its heading) to make T036 pass.
- [X] T038 [US6] Create `CHANGELOG.md` ("Keep a Changelog" layout, `data-model.md` "CHANGELOG.md").
  - **`## [Unreleased]`:**
    - `### Fixed`: this feature's fixes (checkout over HTTPS, page tests on Node 24, release re-runs keep notes, verified downloads);
    - `### Changed`: triggers, protected `main`, secrets in a restricted context, new `audit` gate, ARM64 tests, release process;
    - `### Notes`: 2.0.0 was never published because of the CI failure, and 2.0.1 is the 2.0.0 program plus these fixes.
  - **`## [2.0.0] - 2026-10-11`:** with `### Notes: tagged but never released`, so that `tests/changelog.rs` passes for the current version 2.0.0.
  - **A closing line:** points to the GitHub history for 1.0.0 and earlier.

  Make T035 pass.
- [X] T039 [US6] Add `sh scripts/test-changelog-section.sh` to `test-tools`. Add `sh scripts/changelog-section.sh "${CIRCLE_TAG#v}" > /dev/null` to `check-version`, before the secrets check, in `.circleci/config.yml`. Validate.
- [X] T040 [P] [US6] Create `release.toml` exactly as in `contracts/release-process.md`.
- [X] T041 [US6] (Done 2026-10-12 in a scratch clone. Result: the prepare step is one `cargo release <level> --no-tag --no-push --no-publish --execute`, because `version` and `replace` alone don't check the tree or the branch; `Cargo.lock` is updated; tag refusals confirmed. See research R7.) Dry-run the steps with cargo-release 1.1.6 on a throw-away branch (`quickstart.md` §5):
  - `version patch` and `replace` show the planned changes;
  - an empty "Unreleased" makes `replace` fail;
  - a dirty tree and a wrong branch name are refused.

  Then run `version patch --execute` on that throw-away branch. If `Cargo.lock` isn't updated, add a `cargo check` step to the documented prepare sequence (research R7). Delete the branch afterwards.

  **Tag-step refusals**, on a scratch clone's `main`:
  - with the tag already present, `cargo release tag` refuses;
  - with `main` one commit behind `origin/main` (`git reset --hard HEAD~1`), check whether `cargo release tag`/`push` refuse.

  If the second case isn't refused, the documented `git pull --ff-only` is the only guard. Record that in `docs/releasing.md`, and confirm that FR-029's wording ("brought up to date by the documented steps") matches.
- [X] T042 [P] [US6] Write `docs/releasing.md`:
  - **Requirements:** cargo-release 1.1.6 install command; `gh`.
  - **Changelog rules:** keeping "Unreleased" up to date in every pull request.
  - **The two steps:** prepare and tag, from `contracts/release-process.md`, including the refusals.
  - **Before tagging:** what to check (secrets names, `docs/ci.md`).
  - **What happens after the tag:** the release chain.
  - **Fixing a failed release:** re-run from failed; never move a tag.

  Point the "Releases" paragraph of `README.md` to it.

**Checkpoint**: the changelog is checked on every gate run, and a release can be prepared and
tagged without typing the version twice.

---

## Phase 7: User Story 4 - Fast and economical runs (Priority: P3)

**Goal**:
- caches refresh every run and are per toolchain;
- macOS and Windows are cached;
- cargo-deb is installed once;
- no VM for registry-only steps (done in T032).

**Independent Test**: `quickstart.md` §9.

### Tests for User Story 4

- [X] T043 [US4] Measure the baseline before changing caches: record the `lint` and `test-rust` durations of a cold run and of a re-run, from `circleci run get` / `circleci job get`, in `specs/009-fix-ci-pipeline/reference/circleci-review.md` (item 4).

### Implementation for User Story 4

- [X] T044 [US4] Rewrite `with-cargo-cache` in `.circleci/config.yml` per `contracts/pipeline.md` "Commands":
  - **Keys:** `cargo-v2-<key>-<< pipeline.parameters.rust-version >>-{{ arch }}-{{ checksum "Cargo.lock" }}-{{ epoch }}`; restore by the two prefixes.
  - **Paths:** `/usr/local/cargo/registry`, `/usr/local/cargo/git`, `target`.
  - **Before saving:** run `rm -rf target/*/incremental target/debug/incremental`.

  Validate.
- [X] T045 [US4] Add caches to `build-macos` (`$HOME/.cargo/registry`, `$HOME/.cargo/git`, `target`) and `build-windows` (`$USERPROFILE/.cargo/registry`, `…/git`, `target`) in `.circleci/config.yml`, with the same key scheme. The key parameter becomes a path list. Validate.
- [X] T046 [US4] Add the job `cargo-deb` in `.circleci/config.yml`:
  - restores the cache `cargo-deb-3.8.0-<< pipeline.parameters.rust-version >>`;
  - runs `cargo install cargo-deb --version 3.8.0 --locked` only if the binary is missing;
  - saves the cache, and persists `tools/cargo-deb` to the workspace.

  Make the three `package-deb` jobs require it and use `ws/tools/cargo-deb` (in `test_and_build` and `rehearsal`). Validate.
- [X] T047 [US4] **Verify** `quickstart.md` §9: the second gate run is at least 30% faster than T043's cold run, and a branch with a different `rust-version` restores no old-toolchain key. Record the numbers next to T043's.

**Checkpoint**: repeat runs reuse their caches; packaging builds no tool more than once.

---

## Phase 8: User Story 5 - Releases tested on every architecture they ship to (Priority: P3)

**Goal**:
- the ARM64 package and image are tested on ARM64;
- vulnerability, licence and documentation checks run on every gate run;
- a weekly run on the newest stable Rust.

**Independent Test**: a rehearsal with the arm64 jobs green; a pull request adding a dependency
with a known advisory fails `audit`; `quickstart.md` §7.

### Tests for User Story 5 (write first, confirm they FAIL)

- [X] T048 [US5] Create `deny.toml` (research R14):
  - **Advisories:** default `[advisories]`.
  - **Licences:** `[licenses] confidence-threshold = 0.8` and the allow list from R14.
  - **Check:** run `cargo deny check advisories licenses` locally with cargo-deny 0.20.2, and confirm it passes on the current tree.

  Then remove `ISC` from the list, confirm it fails naming the ISC crate, and restore it. This proves the check bites.

### Implementation for User Story 5

- [X] T049 [US5] Add the gate job `audit` in `.circleci/config.yml`, with tag filters like the other gates:
  1. install cargo-deny 0.20.2 through `fetch-verified`;
  2. run `cargo deny check advisories licenses`;
  3. run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked`, inside `with-cargo-cache`.

  Make the build jobs also require `audit`. Validate. Then run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked` locally, and fix any warnings in `src/` doc comments (no behaviour change).
- [X] T050 [US5] Extend `test-deb` in `.circleci/config.yml` with a job parameter `arch` (`amd64` → Docker default resource class; `arm64` → `resource_class: arm.medium`) and a matrix over the four images × two architectures, named `test-deb-<distro>-<arch>`. The arm64 jobs require `package-deb-aarch64-unknown-linux-musl` and install `ws/release/live-qso-map_*_arm64.deb`. Add them to `test_and_build` and `rehearsal`, and to `publish-github-release`'s requirements. Validate.
- [X] T051 [US5] Add `image-test-arm64` in `.circleci/config.yml` (machine, `resource_class: arm.medium`, image `ubuntu-2204:current`, `--load` for `linux/arm64` + `scripts/test-image.sh`). Make `image-push` require both image tests. Add it to `rehearsal`. Validate.
- [X] T052 [US5] Add the workflow `newest_toolchain` (`when: << pipeline.parameters.newest-toolchain >>`) with the jobs `lint-stable` and `test-rust-stable` (`rustup toolchain install stable --profile minimal -c rustfmt,clippy`, `RUSTUP_TOOLCHAIN=stable`, then the `lint` / `test-rust` commands) in `.circleci/config.yml`. Their names must differ from the gates' (research R2). Validate.
- [X] T053 [US5] Add `{ "context": "ci/circleci: audit", "integration_id": 302869 }` to `deploy/github/main-ruleset.json`. [ext] After `audit` has reported once on the pull request, apply the update with `gh api -X PUT repos/sardylan/live-qso-map/rulesets/<id> --input deploy/github/main-ruleset.json`.
- [X] T054 [US5] [ext] Create the weekly schedule trigger (Monday 04:00 UTC, `main`, `{"newest-toolchain": true}`) with the `circleci api` command in `contracts/ci-service-setup.md`. Use the UI fallback if the API refuses it, and note which one worked in `docs/ci.md`.
- [X] T055 [P] [US5] Add the sections "Weekly newest-toolchain run" (what runs, where failures show, how to raise the pin per `docs/rust-guidelines.md` §2.2) and "Audit gate" (`deny.toml`, how to handle a new advisory or licence) to `docs/ci.md`.
- [ ] T056 [US5] **Verify**:
  - a rehearsal (`quickstart.md` §4) now includes eight `test-deb` jobs and two `image-test` jobs, all green;
  - `quickstart.md` §7: only `lint-stable` and `test-rust-stable` run, and the commit's `ci/circleci: lint` status is unchanged;
  - on a scratch pull request, adding a dependency with a known RustSec advisory (for example an old `time` 0.1) makes `audit` fail. Close it without merging.

**Checkpoint**: ARM64 is exercised, and the new checks run on every gate run.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T057 [P] Change the image label to `org.opencontainers.image.licenses="GPL-3.0-or-later"` in `deploy/docker/Dockerfile.release`, and the matching row in `specs/008-platform-packaging/contracts/release-artifacts.md` (research R17).
- [X] T058 [P] Mark every item of `specs/009-fix-ci-pipeline/reference/circleci-review.md` with its resolution, or "out of scope (decision)" for 9, 11 and 14. Mark item 15 resolved by T023/T056.
- [X] T059 [P] Add a lessons-learned entry to `docs/lessons-learned.md`, following the file's format and lock rules. The lesson: a pipeline that validates can still never have run. Check the run history (`circleci run list`), not only `config validate`, before trusting a release path. OAuth checkout needs an SSH key even for public repositories.
- [X] T060 Run the local gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `node --test 'web/*.test.js'`, `sh scripts/test-*.sh`, and `circleci config validate .circleci/config.yml`.
- [X] T061 Run a final rehearsal and the event checks (`quickstart.md` §2, §4, §6) on the final branch head, then merge the feature pull request with the gates green. Delete the feature branch locally and remotely (constitution, Development Workflow).

---

## Phase 10: First release, 2.0.1 (after the merge)

- [X] T062 [US6] [ext] Cut 2.0.1 with `docs/releasing.md`:
  1. on `main`, the prepare step (patch);
  2. the release pull request is green and merged;
  3. the tag step pushes `v2.0.1`.

  **Verify** `quickstart.md` §8:
  - the release chain is green on the first attempt;
  - the notes equal the changelog section;
  - Docker Hub has `2.0.1` and `latest` with the same digest, and no `2.0.1-rc`;
  - a re-run of `publish-github-release` keeps hand-edited notes;
  - `v2.0.0` is untouched.

  Record the outcome in `specs/009-fix-ci-pipeline/reference/circleci-review.md`. If the Docker Hub tag deletion fails, remove the tag in the Hub UI, and record it (research R8 risk).

---

## Dependencies & Execution Order

- **Setup (T001–T003)** → **Foundational (T004–T006)** → stories. T003 (constitution amendment) MUST be done before US2.
- **US1 (T007–T016)** first: it is the MVP, and the ruleset (T015) needs green gates on the
  feature pull request.
- **US2 (T017–T023)** after US1. The rehearsal needs the verified downloads (T005–T006). T023
  may expose executor or cross failures, which are fixed before US3–US5 build on the same jobs.
- **US3 (T024–T034)** after US2: T027 attaches contexts to jobs created by T018. T031 needs
  T037 (`changelog-section.sh`), so do T036–T037 (US6) before T031, or stub the notes step and
  finish it after T037.
- **US6 (T035–T042)** can start right after Phase 2; it touches mostly new files. T039 edits
  `check-version`, so do it after T027.
- **US4 (T043–T047)** after US2 (T046 changes `package-deb`, which T019 also changes).
- **US5 (T048–T056)** after US2 and US3: T050 and T051 extend the rehearsal and `image-push`.
  T053 needs T015.
- **Polish (T057–T061)** last. T061 needs everything.
- **Release (T062)** only after T061's merge.
- **Within each story**: tests → implementation → docs → [ext] settings → verify.
- **`.circleci/config.yml`** is edited by T004–T006, T009, T018–T021, T026–T027, T031–T033,
  T039, T044–T046, T049–T052: strictly one at a time.

## Parallel Opportunities

- **Setup**: T002 alongside T001.
- **US1**: T010 (README), T011 (ruleset JSON) and T012 (docs) alongside T009.
- **US3**: T024 (test script) alongside the config tasks; T028 and T034 are docs.
- **US6**: T035 (Rust test), T036 (shell test) and T040 (`release.toml`) in parallel; T042 docs
  alongside T037–T038.
- **US5**: T048 (`deny.toml`) and T055 (docs) alongside the config tasks.
- **Polish**: T057, T058 and T059 touch different files.
- **Across stories**: US6's new files (`CHANGELOG.md`, `tests/changelog.rs`, `release.toml`,
  `scripts/*changelog*`, `docs/releasing.md`) don't overlap with anything until T039.

### Parallel example: User Story 6

```text
Task: "T035 [US6] tests/changelog.rs"
Task: "T036 [US6] scripts/test-changelog-section.sh"
Task: "T040 [US6] release.toml"
```

## Implementation Strategy

1. **MVP (US1)**: Phases 1–3. CI checks every pull request and `main` again, the page tests are
   fixed, and `main` is protected. Stop and validate with quickstart §2–§3.
2. **US2**: the first rehearsal proves (or exposes) the release path, before anything else
   changes in it.
3. **US3 and US6**: safe publishing and the release process, together, because publishing reads
   the changelog.
4. **US4, then US5**: speed, then wider checks. Each is validated by a rehearsal.
5. **Polish, merge, then release 2.0.1 (T062)**: the first release cut with the new process.
