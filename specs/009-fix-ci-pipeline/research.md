# Research: Working, Safe and Proven CI Pipeline

Feature: `009-fix-ci-pipeline` · Spec: [spec.md](spec.md) · Date: 2026-10-12

**Versions checked on 2026-10-12:**
- **Release and CI tools:**
  - cargo-release 1.1.6 (2026-09-16);
  - cargo-deny 0.20.2 (2026-07-09);
  - gh 2.102.0 (2026-09-30);
  - crane (go-containerregistry) 0.22.1;
  - cross 0.2.5 (2023-02-04, still the latest release);
  - cargo-deb 3.8.0 (unchanged).
- **Runtimes:** Node 24 (CI image) and 26.11.1 (development).
- **CircleCI:**
  - images `ubuntu-2604:current` (x86) and `xcode: "27.0"` with `m4pro.medium`;
  - Arm Docker and machine `arm.medium`;
  - CLI 1.5.0.

**Project state read on 2026-10-12:**
- **Pipelines:**
  - GitHub App pipeline `f5014549-bbc3-4712-901f-e2bd315c9e6e` with one trigger, `all-pushes`
    (`2f62efc0…`);
  - the OAuth pipeline `baf57313-…` has no triggers.
- **Project settings:** `build-forked-pull-requests=false`, `auto-cancel-builds=true`,
  `oss=true`.
- **GitHub:**
  - `main` has no protection and no ruleset;
  - CircleCI reports one commit status per job, `ci/circleci: <job>`, from the CircleCI App
    (integration id 302869).
- **Licence:** `Cargo.toml` declares `GPL-3.0-or-later`.

## R1. Which events start a run

- **Decision**: replace the `all-pushes` trigger with **one** GitHub App trigger using the
  **`only-build-prs`** preset. It starts a run for:
  - pushes to the default branch;
  - pull requests being opened or re-opened, and pushes to a branch with an open pull request;
  - tag pushes.

  A branch with no pull request starts nothing. The OAuth pipeline keeps no trigger.
- **Rationale**:
  - It is exactly the policy (FR-002 to FR-006) in one rule. CircleCI documents it as "PR opened
    or pushed to, default branch and tag pushes", the GitHub App equivalent of OAuth's "only
    build pull requests".
  - **Avoiding duplicates:** a pipeline can have several triggers, and each one that matches
    starts its own run. Adding `default-branch-pushes` or `only-tags` next to it would run `main`
    or tags twice (FR-006).
  - The config file's filters stay as the second line of defence: release jobs only for
    `/^v\d+\.\d+\.\d+$/` tags.
- **Alternatives**:
  - **`all-pushes` (today)**: runs every branch push and doesn't react to pull request events.
  - **`default-branch-pushes` + `only-build-pushes-to-non-draft-prs` + `only-tags`**: three
    rules, overlapping on some events, and it skips draft pull requests, which the spec wants
    checked.
- **To verify** (quickstart §2): one run each for a push to `main`, a pull request update and a
  test tag, and none for a branch without a pull request.

## R2. Workflows and pipeline parameters

- **Decision**: one `.circleci/config.yml` with three pipeline parameters and three workflows.

  | Parameter | Type | Default | Set by |
  |-----------|------|---------|--------|
  | `rust-version` | string | `"1.99"` | config (pinned, `docs/rust-guidelines.md` §2.2) |
  | `rehearsal` | boolean | `false` | the maintainer: `circleci pipeline run … --param rehearsal=true` |
  | `newest-toolchain` | boolean | `false` | the weekly schedule trigger (R12) |

  | Workflow | Runs when | Jobs |
  |----------|-----------|------|
  | `test_and_build` | neither parameter set (every trigger event) | the gates on every event; the release chain on `vX.Y.Z` tags only (existing map filters) |
  | `rehearsal` | `rehearsal` | the whole build, package, archive and image-test chain on the branch, with no context, no push, no publish, no promote |
  | `newest_toolchain` | `newest-toolchain` | `lint-stable` and `test-rust-stable`: the same commands on `rustup` stable |

- **Rationale**:
  - **Detecting run kinds:** pipeline parameters are the documented way for GitHub App
    pipelines, because `pipeline.trigger_source` is deprecated (support ended 2026-08-01) and
    `pipeline.schedule.*` isn't available.
  - **Sharing:** the rehearsal reuses the same job definitions with a `publish: false` style
    parameter where a job differs (only the image job). It is therefore "the same steps as a real
    release" (FR-011).
  - **Filters:** map-style job `filters` still work for GitHub App pipelines (they are marked
    deprecated in favour of string expressions). Keeping them limits the change. Moving to
    expression filters is a later, separate clean-up.
  - **Distinct job names:** the weekly jobs must not reuse the gate names. Otherwise their
    `ci/circleci: lint` status would overwrite the required check on `main`'s head commit (R9).
- **Alternatives**:
  - **A second config file and a second pipeline definition for rehearsals**: duplicates every
    job definition.
  - **A `rehearsal/*` branch-name convention**: any push to such a branch would start a
    rehearsal, but FR-011 requires rehearsals to start by hand only.

## R3. Publishing secrets

- **Decision**: a CircleCI **context** `live-qso-map-release` holding `GITHUB_TOKEN`,
  `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`, with two restrictions:
  - a **project restriction** to this project;
  - an **expression restriction** `pipeline.git.tag starts-with "v"`.

  Only `check-version`, `image-push`, `publish-github-release` and `docker-promote` declare
  `context: live-qso-map-release`. Any project-level variables with these names are deleted.
- **Rationale**:
  - **Tags only, enforced by CircleCI:** expression restrictions are evaluated by CircleCI, not
    by our config. A branch, pull request, rehearsal or scheduled run has an empty
    `pipeline.git.tag`, so it fails closed and can't receive the context even if a job declared it
    (FR-014, SC-007).
  - **Forks:** they never run (R10).
  - **Who can change it:** only org admins can change restrictions.
  - **Least privilege:**
    - `GITHUB_TOKEN` becomes a fine-grained token limited to this repository with
      `contents: write`;
    - `DOCKERHUB_TOKEN` is a personal access token with Read, Write and Delete on
      `sardylan/live-qso-map`. Delete is needed for R8.
- **Presence check (FR-013)**: `scripts/check-secrets.sh VAR…` exits 1 and names each variable
  that is empty or missing, never printing a value. `check-version` runs it, and every build
  requires `check-version`, so a missing secret stops the release before any build. A rehearsal
  receives no context, so it prints "publishing secrets: not checked (rehearsal)".
  `docs/ci.md` gives `circleci context env-var list` (names only) for a manual check.
- **Alternatives**:
  - **Security groups**: need a GitHub organisation team, but `sardylan` is a user account.
  - **Project variables with branch-only filtering in the config**: anyone editing the config in
    a pull request could read them.

## R4. Page tests on Node 24

- **Decision**: run `node --test 'web/*.test.js'` (quoted glob) on the `node:24` image, and use
  the same command in `README.md`.
- **Rationale**:
  - **Why Node 24 fails:** since Node 21, `node --test` treats its arguments as glob patterns, so
    a bare directory matches itself and fails with `MODULE_NOT_FOUND`. Directory search came back
    only in Node 26.8.0 (commit `1ad7ca310a`).
  - **Why the quoted glob:** Node expands it itself, identically on 24 and 26 and on every host
    shell (Principle V). It selects the same 10 files and 131 tests (SC-004).
  - **Why keep Node 24:** it is the active LTS, the conservative CI choice.
- **Alternatives**:
  - **`node:26` with `web/`**: works, but ties CI to a non-LTS release, and a developer on
    Node 24 would hit the same bug.

## R5. Publishing the GitHub release

- **Decision**: replace `ghr -delete` with the **GitHub CLI 2.102.0** (tarball verified against
  `gh_2.102.0_checksums.txt`) on `cimg/base`. The job:
  1. extracts the notes: `sh scripts/changelog-section.sh X.Y.Z > notes.md` (R6);
  2. if `gh release view vX.Y.Z` fails, runs
     `gh release create vX.Y.Z --verify-tag --title vX.Y.Z --notes-file notes.md ws/release/*`;
  3. otherwise runs `gh release upload vX.Y.Z --clobber ws/release/*`, which keeps the existing
     notes untouched (FR-015, SC-008).
- **Rationale**:
  - **Re-runs keep the notes:** the commands' behaviour is explicit, and a re-run only adds or
    replaces assets.
  - **No moving tag:** `--verify-tag` refuses to create a tag, so the release always uses the
    pushed one.
- **Accepted risk**: `--clobber` deletes and then re-uploads a same-named asset. If the upload
  fails midway, that asset is missing until the job is re-run, which is idempotent.
- **Alternatives**:
  - **`ghr -replace`**: whether it rewrites the body on an existing release isn't documented.
  - **The GitHub API with `curl`**: more code for the same result.

## R6. Changelog and release notes

- **Decision**:
  - **Format:** `CHANGELOG.md` in "Keep a Changelog" layout. The top is
    `## [Unreleased]`, then one `## [X.Y.Z] - YYYY-MM-DD` (UTC) section per release, newest
    first, each containing `### Added|Changed|Fixed|Removed|Security` subsections.
  - **`scripts/changelog-section.sh <X.Y.Z> [CHANGELOG.md]`** (POSIX `sh`) prints that
    section's body. It exits 1 if the section is missing or has no list entry.
  - **Version gate:** `check-version` calls it on tags, before any build (FR-032).
  - **Rust test:** `tests/changelog.rs` checks that `CHANGELOG.md` has an `## [Unreleased]`
    heading and a non-empty section for `env!("CARGO_PKG_VERSION")`.
- **Rationale**:
  - **The Rust test catches errors before the tag:**
    - on `main`, the package version is always the last released one, so its section exists;
    - on a release pull request, the version is the new one, and the prepare step has just
      written its section.

    So a release pull request with a broken changelog fails its gates and can't merge (FR-009).
    The tag step therefore never meets a missing section (US6 scenario 6).
  - **Portability:** the test runs on every host (Principle V). The shell script is only for CI.
- **Alternatives**:
  - **A cargo-release hook running a script**: hooks run shell commands, and a `sh` script isn't
    portable to Windows (Principle V).
  - **`git-cliff` or generated notes**: rejected by the clarification (notes come from the
    changelog).

## R7. Release process with cargo-release

- **Decision**: cargo-release **1.1.6**, configured in `release.toml`:

  ```toml
  publish = false
  tag-name = "v{{version}}"
  allow-branch = ["main", "release/next"]
  pre-release-commit-message = "chore(release): {{version}}"
  pre-release-replacements = [
    { file = "CHANGELOG.md", search = "## \\[Unreleased\\]\n\n###", replace = "## [Unreleased]\n\n## [{{version}}] - {{date}}\n\n###", exactly = 1 },
  ]
  ```

  The two steps are documented in `docs/releasing.md`. They are plain commands, the same in any
  shell on Linux, macOS and Windows.

  **Prepare** (on an up-to-date `main`):
  ```
  git switch -c release/next
  cargo release version <patch|minor|major> --execute
  cargo release replace --execute
  cargo release commit --execute
  git push -u origin release/next
  gh pr create --base main --fill
  ```

  **Tag** (after the pull request is merged):
  ```
  git switch main && git pull --ff-only
  cargo release tag --execute
  cargo release push --execute
  ```
- **Rationale**:
  - **Version typed once:** the level (`patch`) computes the version, and the tag is derived
    from `Cargo.toml` (FR-029, FR-030).
  - **Refusals:**
    - `exactly = 1` on a pattern that needs a `###` subsection right after `[Unreleased]` makes
      the replace step fail when "Unreleased" is empty (FR-031);
    - `allow-branch` refuses other branches;
    - cargo-release refuses a dirty tree;
    - without `--execute`, every step only reports what it would do.
  - **No direct pushes to `main`:** on an up-to-date `main`, `push` pushes only the tag, because
    the local branch equals the remote. So nothing is pushed to the protected `main` (FR-009).
  - **Dates:** `{{date}}` is UTC (`%Y-%m-%d`), as HR-1 requires.
  - **No registry:** `publish = false` keeps it off crates.io.
- **To verify while implementing**: that `cargo release version` also updates `Cargo.lock`. If it
  doesn't, `cargo check` before the commit step updates it (quickstart §5).
- **Alternatives**:
  - **One-step `cargo release patch --execute`**: needs a direct push to `main`, rejected by the
    clarification.
  - **A wrapper script**: Principle V requires a portable path, and the documented commands
    already are one.

## R8. Image: test per architecture, push, promote, clean up

- **Decision**: the `docker-image` job is split into three.

  | Job | Executor | What it does |
  |-----|----------|--------------|
  | `image-test` (matrix `amd64`, `arm64`) | machine `ubuntu-2604:current` / machine `arm.medium` `ubuntu-2204:current` | `docker buildx build --platform linux/<arch> --load`, then `scripts/test-image.sh` |
  | `image-push` (release only) | machine `ubuntu-2604:current` | multi-platform build and push as `X.Y.Z-rc`, after both `image-test` jobs |
  | `docker-promote` (release only) | Docker `cimg/base:current` | crane 0.22.1 (checksum-verified): `crane tag …:X.Y.Z-rc X.Y.Z` and `latest`, then delete the `-rc` tag |

  **Deleting the `-rc` tag:**
  1. `POST https://hub.docker.com/v2/auth/token` with the username and personal access token
     returns a token;
  2. then `DELETE https://hub.docker.com/v2/namespaces/sardylan/repositories/live-qso-map/tags/X.Y.Z-rc`.

  The job fails if the tag still exists afterwards. A re-run is idempotent (FR-018, SC-011).
- **Rationale**:
  - **Native builds:** each architecture is tested on native hardware. `Dockerfile.release` runs
    nothing on the target platform, so the arm64 build needs no emulation (FR-024, SC-010).
  - **Rehearsals:** they run `image-test` only (FR-012).
  - **No VM for promotion:** crane re-tags the tested multi-arch index without a daemon, keeping
    its digest (FR-022).
- **Risk**: Docker Hub's OpenAPI doesn't list the tag `DELETE` endpoint, although the
  access-token documentation says tags support delete. If the first release shows the endpoint
  doesn't work, `docs/ci.md` gives the manual removal (Docker Hub UI), and the step is revisited.
  The release itself is already complete at that point.
- **Arm machine image**: `ubuntu-2204:current` is the confirmed Arm Gen1 image. `ubuntu-2604` is
  listed for Arm only on Gen2 (beta), so it isn't used.
- **Alternatives**:
  - **QEMU on the x86 machine**: slow, and not real ARM64 hardware.
  - **`docker buildx imagetools` on a Docker executor**: needs the Docker CLI with the buildx
    plugin and a daemon-less login. crane is a single verified binary.

## R9. Protecting `main`

- **Decision**: a GitHub **ruleset** on the default branch, kept in
  `deploy/github/main-ruleset.json`. The maintainer applies it with
  `gh api -X POST repos/sardylan/live-qso-map/rulesets --input deploy/github/main-ruleset.json`,
  or with `PUT …/rulesets/<id>` for an update. Its rules:
  - **deletion** and **non_fast_forward** blocked;
  - **pull_request**, with 0 required approvals, because the project has one maintainer;
  - **required_status_checks**, with `strict_required_status_checks_policy: false`, on these
    contexts, each from integration 302869:
    - `ci/circleci: lint`
    - `ci/circleci: test-rust`
    - `ci/circleci: test-web`
    - `ci/circleci: test-tools`
    - `ci/circleci: audit`
  - **bypass_actors**: `[{actor_type: "RepositoryRole", actor_id: 5, bypass_mode: "always"}]`
    (repository admin, FR-009).
- **Rationale**:
  - **Reviewable:** the rule is a versioned file, so changing a job name and the required checks
    happens in the same pull request (edge case "gate names change").
  - **Not strict:** a strict "up to date with `main`" policy would force a rebase for every
    merge, which isn't worth it with a single maintainer.
- **To verify**: that `actor_id: 5` is the admin role. GitHub's documentation doesn't state it, so
  check it after applying with `gh api repos/sardylan/live-qso-map/rulesets/<id>` (quickstart §3).
- **Alternatives**:
  - **Classic branch protection**: older API, and its bypass is less explicit.
  - **Rules set by hand in the UI**: not reviewable.

## R10. Pull requests from forks

- **Decision**: none run (a platform limit). `docs/ci.md` describes the re-push procedure:
  1. review the fork pull request;
  2. `gh pr checkout <n>`;
  3. `git push origin HEAD:refs/heads/contrib/<n>`;
  4. `gh pr create --head contrib/<n> --base main`;
  5. close the fork pull request with a link to the new one.

  The commits keep the contributor as author.
- **Rationale**:
  - **No fork builds:** CircleCI's GitHub App integration never builds fork pull requests and has
    no approval setting. The project setting `build-forked-pull-requests` affects only the OAuth
    pipeline, which has no trigger.
  - **No secrets:** fork code never runs on CI before review, and never sees secrets (FR-010,
    FR-014).
- **Alternatives**: rejected in the spec's Clarifications (an admin-bypass merge, or keeping
  OAuth for forks).

## R11. Verified downloads and the cross-build tool

- **Decision**: `.circleci/config.yml` keeps each download's version and SHA-256 next to each
  other, and every download goes through one reusable command, `fetch-verified`
  (`curl -fsSL`, then `sha256sum -c`). The pinned downloads are:

  | Download | Version | Checksum source |
  |----------|---------|-----------------|
  | cross (x86_64 musl tarball) | 0.2.5 | none published: computed once and recorded |
  | cargo-deny (x86_64 musl tarball) | 0.20.2 | `.sha256` file per asset |
  | gh (linux amd64 tarball) | 2.102.0 | `gh_2.102.0_checksums.txt` |
  | crane (Linux x86_64 tarball) | 0.22.1 | `checksums.txt` |
  | rustup-init (x86_64-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc) | pinned rustup release | `rustup-init.sha256` on static.rust-lang.org |

  cargo-deb comes from crates.io through `cargo install --locked`, which is a package manager,
  so it isn't affected by FR-016.
- **cross**:
  - **Version:** 0.2.5 stays, because it is still the latest release; the maintainer declined a
    0.2.6.
  - **Images:** `Cross.toml` pins each target's image by digest
    (`ghcr.io/cross-rs/<target>:0.2.5@sha256:…`), so the images are verified too.
  - **Proof:** the rehearsal proves the three musl targets build with Rust 1.99 (FR-017).
  - **Fallback:** if a target fails, it moves to a pinned `:main` image digest, recorded in
    `Cross.toml` with the reason.
- **Rationale**: one verified path for every download, and the expected checksums sit in the
  reviewable config (FR-016).
- **Alternatives**:
  - **cargo-zigbuild**: a different linker for the musl targets, which is a larger change than
    this feature needs.
  - **`cross` from git**: an unreleased build, harder to pin.

## R12. Weekly newest-toolchain run

- **Decision**: a **schedule** trigger on the GitHub App pipeline: Monday 04:00 UTC, on `main`,
  with parameters `{"newest-toolchain": true}`. It is created with
  `circleci api -X POST api/v2/projects/<project-id>/pipeline-definitions/<def-id>/triggers`
  (body in `contracts/ci-service-setup.md`), or in the UI (Project Setup → Schedule) if the API
  refuses. The `newest_toolchain` workflow:
  1. runs `rustup toolchain install stable --profile minimal -c rustfmt,clippy`;
  2. sets `RUSTUP_TOOLCHAIN=stable`;
  3. runs fmt, clippy and the tests as the jobs `lint-stable` and `test-rust-stable`.
- **Rationale**:
  - **Not blocking:** the job names aren't required checks, so a failure never blocks a merge or
    a release (FR-026).
  - **Visible:** it shows as a red workflow in CircleCI, and CircleCI's default email
    notifications report failed runs to the maintainer.
  - **Weekly:** Rust stable changes every 6 weeks, so a weekly check is enough.
- **Risk**: CircleCI's docs say schedule triggers for GitHub App pipelines can't be created
  through the API yet, although the v2 OpenAPI accepts them. The UI is the documented fallback.

## R13. Caches

- **Decision**: one key scheme everywhere.
  - **Save:**
    `cargo-v2-<job>-<< pipeline.parameters.rust-version >>-{{ arch }}-{{ checksum "Cargo.lock" }}-{{ epoch }}`
  - **Restore:** by the prefix up to the lock checksum, then up to the toolchain.
  - **Contents:** the Cargo registry and git directories, plus `target/` without
    `target/*/incremental`.
  - **macOS and Windows:** `build-macos` and `build-windows` get the same caches, with their
    `$HOME/.cargo` paths.
  - **cargo-deb:** a new job `cargo-deb` installs it once per release, cached under
    `cargo-deb-3.8.0-<rust-version>`, and passes the binary through the workspace to the three
    `package-deb` jobs (FR-020).
- **Rationale**:
  - **Fresh:** `{{ epoch }}` makes every save a new key, so the newest output is restored
    (FR-019).
  - **Per toolchain:** the Rust version in the key stops reuse across toolchains (US4
    scenario 2).
  - **Smaller:** pruning `incremental` keeps the cache small.
- **Alternatives**: a `{{ .Revision }}` key would never be restored by later commits.

## R14. Dependency, licence and documentation checks

- **Decision**: a new gate job **`audit`** with these steps:
  1. `cargo deny check advisories licenses`, with cargo-deny 0.20.2 (verified download, R11);
  2. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked`.

  Minimal `deny.toml`:
  - **Advisories:** defaults (vulnerabilities denied, unmaintained crates warned).
  - **Licences:** `confidence-threshold = 0.8`, and this allow list:
    - MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, ISC;
    - BSD-2-Clause, BSD-3-Clause;
    - Unicode-3.0, Unlicense, CDLA-Permissive-2.0;
    - GPL-3.0-or-later (the crate itself).
- **Rationale**:
  - **Compatible licences:** every allowed licence is compatible with GPL-3.0-or-later (FR-025).
    These are the licences the dependency tree uses on 2026-10-12.
  - **New licences need review:** a new one fails the gate until it's reviewed and added.
  - **Required:** `audit` is a required check (R9).
- **Accepted risk**: a newly published advisory can turn the gate red without any change in the
  repository. That is the purpose of the check. The fix is a dependency update, or a documented
  `ignore` with a reason.

## R15. Executors

- **Decision**: unchanged where they work. The rehearsal (FR-011) proves the unconfirmed ones on
  their first use:
  - `ubuntu-2604:current` (x86 machine);
  - `xcode: "27.0"` with `m4pro.medium`;
  - Arm Docker `arm.medium`, used for `test-deb` arm64;
  - Arm machine `arm.medium` with `ubuntu-2204:current`.

  All are on the Free plan. macOS is limited there to `m4pro.medium` with one concurrent job.

## R16. Release 2.0.1 and the unreleased 2.0.0

- **Decision**:
  - **Changelog:** `CHANGELOG.md` starts with `## [Unreleased]`, whose `### Fixed` and
    `### Changed` entries describe this feature. The prepare step turns them into
    `## [2.0.1] - <date>` (FR-028).
  - **About 2.0.0:** a `### Notes` line in that section says that 2.0.0 was never published,
    because of the CI failure, and that 2.0.1 is the 2.0.0 program plus the pipeline fixes.
  - **2.0.0:** a `## [2.0.0] - 2026-10-11` section with a `### Notes` entry: tagged, never
    released. `tests/changelog.rs` needs a section for the crate's current version, which is
    2.0.0 until the release pull request.
  - **Earlier history:** a final line points to the GitHub history for versions up to 1.0.0.
  - **The tag:** `v2.0.0` is left untouched.
- **Rationale**: no tag is rewritten (clarification), and the notes explain the gap.

## R17. Licence label in the image

- **Finding**: `deploy/docker/Dockerfile.release` labels the image
  `org.opencontainers.image.licenses="GPL-3.0-only"`, while `Cargo.toml` and the Debian package
  declare `GPL-3.0-or-later`.
- **Decision**: align the label (and the 008 contract `release-artifacts.md`) to
  `GPL-3.0-or-later`. `Cargo.toml` is the source of the project's licence.
- **Rationale**: published artefacts must not disagree about the licence. This is a one-line fix
  in a file this feature already edits, so it's in scope.
