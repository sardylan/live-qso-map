# CircleCI pipeline review

Review of `.circleci/config.yml` (state at commit `210528a`, 2026-10-12).

`circleci config validate` (CLI 1.5.0) reports the config as **valid**, and
`circleci config process` confirms that the orb executors, the matrix expansion and the
workflow filters resolve as intended.

The pipeline nevertheless **failed on every run** because of a project-setup problem outside the
repository (item 0, resolved on 2026-10-12). Once checkout worked, a real test failure appeared
(item 16), and the run policy needs changing (item 17). Items 1–15 are about design, cost and
robustness, not syntax. They are ordered by importance.

| # | Problem | Area | Severity |
|---|---------|------|----------|
| 0 | Every run fails at checkout: no deploy key on GitHub | Project setup | **Resolved** |
| 16 | `test-web` fails: Node 24 doesn't accept a directory in `node --test` | Testing | **Blocker** |
| 17 | Runs start on every branch push; tag runs unconfirmed | Triggers | High |
| 1 | Secrets not scoped to a context | Security | High |
| 2 | `ghr -delete` replaces an existing release | Release | High |
| 3 | `cross` downloaded without verification, old version | Supply chain | Medium |
| 4 | Cargo cache keys never refresh, miss the toolchain | Caching | Medium |
| 5 | `cargo-deb` compiled from source three times per release | Cost / time | Medium |
| 6 | No caching on macOS and Windows builds | Cost / time | Medium |
| 7 | `docker-promote` uses a full machine VM | Cost | Low |
| 8 | Uneven test coverage across architectures | Testing | Medium |
| 9 | armv7 missing from the container image | Coverage | Low |
| 10 | `-rc` image tags never cleaned up | Hygiene | Low |
| 11 | macOS binary not signed or notarized | Distribution | Low |
| 12 | No security, licence or doc checks | Quality | Medium |
| 13 | No early warning against latest stable Rust | Maintenance | Low |
| 14 | Tag regex rejects pre-release tags | Release | Low |
| 15 | Executor images and resource classes unconfirmed | Availability | Medium |

---

## 0. Every run fails at checkout: no deploy key on GitHub

**Evidence** (`circleci run list`, `circleci job output get <job> --step-num 101`, 2026-10-12):

| Run | Ref | Commit | Result |
|-----|-----|--------|--------|
| `920c7e92-2a0a-40b6-9e03-4df52af083ff` | `feature/008-platform-packaging` | `4d12e4f` | failed after 18 s |
| `b77d42f1-9dc2-4de5-afa6-b89d0710f643` | `main` | `210528a` | failed after 20 s |
| `704af798-d3a8-4cae-b7f5-6bd8998f1dbe` | tag `v2.0.0` | `210528a` | failed after 20 s; 17 release jobs not run |

These three runs are the project's entire history: the pipeline **has never passed a single
job**. In every run, every started job (`lint`, `test-rust`, `test-web`, `test-tools` and, on the
tag, `check-version`) fails at the `Checkout code` step after about 170 ms, with:

```
Cloning git repository - git@github.com:sardylan/live-qso-map.git
git@github.com: Permission denied (publickey).
fatal: Could not read from remote repository.
error running git clone "git@github.com:sardylan/live-qso-map.git": exit status 128
```

**Cause:** CircleCI clones over SSH with the project's checkout key, but GitHub doesn't
accept that key. `gh api repos/sardylan/live-qso-map/keys` returns **0 deploy keys**, while
the CircleCI webhook (`https://circleci.com/hooks/github`) is present. So pushes trigger
pipelines, but no job can fetch the code. The project was followed (webhook created) without a
deploy key being added, or the key was removed from GitHub afterwards. The repository is public,
but CircleCI's `checkout` always uses SSH and needs an authorised key anyway.

**Consequences:**
- None of `main`'s quality gates (constitution Principle V) has ever run in CI.
- The `v2.0.0` release produced nothing: no GitHub release, no packages, no image.
- Every job after checkout (builds, packaging, image, publishing) is **unproven**. Items 3, 7, 8 and
  15 may hide further failures that will appear only once checkout works.
- The CLI token used for this analysis gets `403 Permission denied` on
  `circleci project envvar list`. So it is also unverified whether `DOCKERHUB_USERNAME`,
  `DOCKERHUB_TOKEN` and `GITHUB_TOKEN` exist at all.

**Why SSH:** the `checkout` step has no protocol option. The protocol depends on how the project
is connected. A project connected through the legacy GitHub **OAuth** integration (slug
`gh/...`) clones over SSH with a checkout key. A pipeline connected through the CircleCI **GitHub
App** clones over HTTPS with an App token (CircleCI changelog, 2024-03-25). GitHub has no
anonymous SSH access, even to public repositories, so the missing key meant `Permission denied`.

**Resolution (2026-10-12, project settings, not the config file):**
- The CircleCI GitHub App was installed and connected to the `sardylan` organisation (Org → VCS
  Connections), and the user authorised it.
- A GitHub App pipeline was created with
  `circleci pipeline create --config-provider github_app --checkout-provider github_app`:
  `f5014549-bbc3-4712-901f-e2bd315c9e6e`, "live-qso-map (GitHub App)", repository ID
  `883686784`, `.circleci/config.yml`.
- A GitHub App trigger was added, with event preset `all-pushes`:
  `2f62efc0-51a0-461d-96c2-f5750aea445d`. See item 17.
- The OAuth trigger `72e306b5-677e-585b-afaa-f3a134f93c1b` was deleted
  (`DELETE /api/v2/projects/<id>/triggers/<id>`), so that no event starts a second, failing SSH
  run. The OAuth pipeline `baf57313-…` can't be deleted, and stays without triggers.
- The test run `5de2ea62-f2e5-4bfe-84d6-dab7bdb37300` on `main` cloned from
  `https://github.com/sardylan/live-qso-map.git`. `lint`, `test-rust` and `test-tools` passed;
  `test-web` failed (item 16).
- Re-running the `v2.0.0` workflow would use the configuration as tagged, and the OAuth pipeline
  it belongs to. Whether to re-run it or cut a new tag is a release decision (see the spec).

## 1. Secrets not scoped to a context

**Where:** `docker-image` (line 296), `docker-promote` (line 328),
`publish-github-release` (line 343); workflow entries from line 360.

**Problem:** no workflow job declares a `context:`. `DOCKERHUB_USERNAME`,
`DOCKERHUB_TOKEN` and `GITHUB_TOKEN` can therefore only be project-level environment
variables, and those are injected into **every** job, including the branch jobs (`lint`,
`test-*`) that run code from any pushed branch.

**Impact:** any commit that reaches CircleCI, such as a compromised dependency in a
`build.rs`, a test, or a malicious branch, can read the Docker Hub and GitHub tokens. The
GitHub token is able to create and delete releases.

**Fix:** move the secrets to a CircleCI context (for example `release`), restricted to a
security group or to protected tags. Attach it only to `docker-image`, `docker-promote`
and `publish-github-release`. Then delete the project-level variables. Also check that
"Pass secrets to builds from forked pull requests" is disabled.

**Resolution (feature 009):** context `live-qso-map-release` restricted to this project and to `pipeline.git.tag starts-with "v"`, used by four jobs only; `check-secrets.sh` in `check-version` (T024–T029).

## 2. `ghr -delete` replaces an existing release

**Where:** line 355.

**Problem:** `-delete` removes any existing GitHub release with the same tag before
uploading. Re-running the workflow, or a rerun of the failed job, deletes and recreates
the release.

**Impact:** release notes or descriptions edited by hand on GitHub are lost silently.
Download counts reset, and the release briefly disappears, which can break links and
mirrors.

**Fix:** drop `-delete` and add `-replace` (replace assets only) if re-uploads are needed.
Alternatively, have the job fail when the release already exists, so that recreating it
is always a deliberate action.

**Resolution (feature 009):** `publish-github-release` uses gh 2.102.0: `gh release create --verify-tag --notes-file` for a new release, `gh release upload --clobber` on a re-run, so notes are kept; `tools/test_ci_config.py` guards it (T030–T031).

## 3. `cross` downloaded without verification, old version

**Where:** `build-linux`, line 141.

**Problem:** the `cross` binary is fetched with `curl | tar` straight into
`$HOME/.cargo/bin`, without any checksum or signature check. The pinned version, v0.2.5,
dates from early 2023, and its default target images (toolchains, linkers, musl
versions) are old compared with Rust 1.99.

**Impact:**
- If the download is tampered with, a compromised linker builds every Linux release
  binary.
- A future Rust bump may break the old images, and that only shows up on a release tag.

**Fix:**
- Verify a pinned SHA-256 after the download.
- Consider a newer `cross` release, or pin the images explicitly in `Cross.toml`
  (`[target.<triple>] image = "...@sha256:..."`).
- Alternatively, drop `cross` for the musl targets and use `rustup target add` plus a
  musl linker or `cargo-zigbuild`.

**Resolution (feature 009):** cross 0.2.5 goes through `fetch-verified` with a recorded SHA-256, and `Cross.toml` pins each target image by digest (T033). Whether 0.2.5 still builds with Rust 1.99 is proven by the rehearsal (T023).

## 4. Cargo cache keys never refresh, miss the toolchain

**Where:** `with-cargo-cache` command, lines 61–67.

**Problems:**
- **Frozen cache.** CircleCI never overwrites an existing cache key. While `Cargo.lock`
  is unchanged, the first `target/` saved under a key stays frozen forever. Later builds
  restore it, rebuild the changed crates, and their results are thrown away.
- **Growing cache.** The fallback key `cargo-v1-<key>-` restores an older `target/`
  after a lockfile change, and it is saved again with all its stale artifacts on top. The
  cache grows without bound.
- **Rust version missing from the key.** After `rust-version` is bumped (line 12),
  artifacts from the old compiler are restored, which is useless (everything is rebuilt)
  but still costs download time.
- **Git dependencies not cached.** `/usr/local/cargo/git` is not cached. It doesn't
  matter today, but it would once a git dependency is added.

**Fix:** include the toolchain in the key, and save with a key that changes
(`{{ epoch }}`) while restoring by prefix:

```yaml
- restore_cache:
    keys:
      - cargo-v2-<< parameters.key >>-<< pipeline.parameters.rust-version >>-{{ checksum "Cargo.lock" }}
      - cargo-v2-<< parameters.key >>-<< pipeline.parameters.rust-version >>-
- save_cache:
    key: cargo-v2-<< parameters.key >>-<< pipeline.parameters.rust-version >>-{{ checksum "Cargo.lock" }}-{{ epoch }}
```

Also consider trimming `target/` before saving (for example dropping `target/*/incremental`).

**Resolution (feature 009):** `with-cargo-cache` keys carry the job, the pinned Rust version and `{{ arch }}`, save under `{{ epoch }}` every run, cache `registry`, `git` and `target`, and prune incremental data (T044). Speed-up measured in T047.

## 5. `cargo-deb` compiled from source three times per release

**Where:** `package-deb`, line 215 (matrix of 3 targets).

**Problem:** `cargo install cargo-deb --version 3.8.0 --locked` builds the tool from
source in every matrix job, with no cache. That is several minutes of compute per job,
three times per release.

**Fix (any of these):**
- install it once in a small job and pass the binary through the workspace;
- cache `/usr/local/cargo/bin/cargo-deb` keyed on the version;
- use `cargo binstall cargo-deb@3.8.0` or a prebuilt release archive with a checksum.

**Resolution (feature 009):** a `cargo-deb` job installs it once (cached across releases) and hands it to the package jobs through the workspace (T046).

## 6. No caching on macOS and Windows builds

**Where:** `build-macos` (line 162), `build-windows` (line 178).

**Problem:** both jobs install Rust from scratch and do a cold `cargo build --release`.
They run on the most expensive executors (`m4pro.medium`, `windows.medium`).

**Fix:** add `restore_cache`/`save_cache` for the Cargo registry and `target/`, with the
same key scheme as in item 4. The paths are `~/.cargo/registry` and `target` on macOS,
and `%USERPROFILE%\.cargo\registry` and `target` on Windows.

**Resolution (feature 009):** `build-macos` and `build-windows` use `with-cargo-cache` with `~/.cargo` (T045).

## 7. `docker-promote` uses a full machine VM

**Where:** line 329.

**Problem:** the job only runs `docker login` and `docker buildx imagetools create`,
which are registry API calls. It does not need a Docker daemon, yet it starts an
`ubuntu-2604` machine VM, which is slower to start and costs more.

**Fix:** use a Docker executor with the Docker CLI and buildx plugin (for example
`cimg/base` plus `setup_remote_docker`), or a small image that has `crane`/`regctl` to
copy the tags.

**Resolution (feature 009):** `docker-promote` runs on `cimg/base` with crane 0.22.1; no machine executor (T032).

## 8. Uneven test coverage across architectures

**Where:** `test-deb-*` (lines 406–421), `docker-image` smoke test (lines 315–318).

**Problems:**
- **Debian packages.** Only the amd64 `.deb` goes through the install / upgrade / save /
  remove / purge life-cycle test (`scripts/test-deb.sh`). The arm64 and armhf packages
  only get static checks: architecture, version and contents.
- **Container image.** The smoke test (`scripts/test-image.sh`) runs only on the amd64
  image. The arm64 image is built and pushed (line 322) without ever being run.

**Impact:** a defect specific to arm64 or armhf, such as a wrong binary layout or a
maintainer script that only fails there, ships unnoticed. Raspberry Pi users are an
explicit target (`Cross.toml`).

**Fix:**
- Run `test-deb` for arm64 on an `arm.medium` machine or Docker executor.
- Run `test-image.sh` on an arm64 executor against the pushed `-rc` image before
  promotion.
- armhf can stay on static checks, or use QEMU if it's worth the cost.

**Resolution (feature 009):** `test-deb` runs on arm64 Docker (`arm.medium`) for the four distributions, and `image-test-arm64` builds and smoke-tests the image on an Arm machine before `image-push` (T050–T051).

## 9. armv7 missing from the container image

**Where:** `docker-image`, lines 315–325.

**Problem:** the armv7 musl binary is built, archived and packaged as a `.deb`, but the
container image is published only for `linux/amd64` and `linux/arm64`.

**Impact:** users of 32-bit Raspberry Pi OS can't use the Docker image. This may be a
deliberate choice; if so, it should be documented.

**Fix:** either add `linux/arm/v7` (`dist/linux/arm/live-qso-map` with
`TARGETARCH=arm`), or state the supported image platforms in the README.

**Resolution (feature 009):** out of scope: feature 008 decided the image ships for amd64 and arm64 only (clarification Q2).

## 10. `-rc` image tags never cleaned up

**Where:** lines 325 and 340.

**Problem:** every release leaves `sardylan/live-qso-map:<version>-rc` on Docker Hub. If
publishing fails after `docker-image`, the `-rc` tag stays there with no matching
release.

**Impact:** the registry fills with clutter, and users may pull an `-rc` tag thinking it
is a supported version.

**Fix:** delete the `-rc` tag after a successful promotion (Docker Hub API), or push the
candidate to a separate staging repository.

**Resolution (feature 009):** `docker-promote` deletes the `-rc` tag through the Docker Hub API after re-tagging with crane, and fails if it is still there (T032).

## 11. macOS binary not signed or notarized

**Where:** `build-macos` (line 162), `package-archives`.

**Problem:** the arm64 binary is shipped in a zip without code signing or notarization.

**Impact:** Gatekeeper quarantines the downloaded binary. Users must run
`xattr -d com.apple.quarantine` or allow it in System Settings.

**Fix:** sign it with a Developer ID and notarize it (`codesign` + `xcrun notarytool`),
with the credentials in a restricted context. Otherwise, document the workaround in
`deploy/macos/README.txt`, if that isn't already done.

**Resolution (feature 009):** out of scope: feature 008 keeps the macOS archive unsigned and documents the "Open anyway" step.

## 12. No security, licence or doc checks

**Where:** `lint` / `test-rust`.

**Problem:** the pipeline doesn't check dependencies for known vulnerabilities, yanked
crates or licence compatibility (the project is GPL-3.0-only). It also doesn't fail on
broken documentation.

**Fix:** add a job with `cargo deny check` (advisories, bans, licences, sources) or
`cargo audit`, and a `cargo doc --no-deps` step with `RUSTDOCFLAGS="-D warnings"`.

**Resolution (feature 009):** new gate `audit`: cargo-deny 0.20.2 (`deny.toml`) and `cargo doc` with warnings denied, except rustdoc's `bare_urls` (doc comments double as `--help` text) (T048–T049).

## 13. No early warning against latest stable Rust

**Where:** pipeline parameter `rust-version` (line 12).

**Problem:** CI pins Rust 1.99 on purpose (`docs/rust-guidelines.md` §2.2), while
development tracks latest stable (`rust-toolchain.toml`). New lints or compiler
behaviour are found only when someone bumps the pin, or on a developer machine.

**Fix:** add a scheduled pipeline (for example weekly) that runs `lint` and `test-rust`
with `rust-version` overridden to `stable`, so problems appear before the pin is moved.

**Resolution (feature 009):** `newest_toolchain` workflow, started by a weekly schedule with `newest-toolchain: true`; jobs `lint-stable` and `test-rust-stable` (T052, schedule in T054).

## 14. Tag regex rejects pre-release tags

**Where:** filters at lines 364 and 375 (`/^v\d+\.\d+\.\d+$/`).

**Problem:** tags such as `v2.1.0-rc.1` match neither the test filter nor the release
filter, so CircleCI ignores them entirely.

**Impact:** you can't rehearse the full release chain (packaging, image, GitHub
pre-release) without publishing a final version number.

**Fix:** if pre-releases are wanted, extend the regex (for example
`/^v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/`). Then update `scripts/check-version.sh`, and
mark the release as a pre-release (`ghr -prerelease`) without promoting `latest`.

**Resolution (feature 009):** out of scope: replaced by the manual rehearsal (TR-014).

## 15. Executor images and resource classes unconfirmed

**Where:** line 28 (`ubuntu-2604:current`), line 33 (`xcode: "27.0"`), line 34
(`m4pro.medium`).

**Problem:** `circleci config validate` checks only the schema, not whether CircleCI
actually offers these images and resource classes. Verified so far:
`windows-server-2025-gui:current` (from orb `circleci/windows@5.1.1`) exists, and the
Docker Hub tag `rust:1.99` exists.

**Impact:** an unavailable image or resource class makes the release jobs fail only on a
tag, after the version has already been tagged.

**Fix:** check them against CircleCI's machine image and Xcode lists, and the resource
classes available on the plan. Consider a manual or branch-triggered dry run of the
build jobs (without publishing) to prove the executors before a real release.

**Resolution (feature 009):** pending: proven by the first rehearsal (T023, T056).

## 16. `test-web` fails: Node 24 doesn't accept a directory in `node --test`

**Where:** `test-web` job (`node --test web/`, executor `node:24`), and `README.md` line 279,
which documents the same command.

**Evidence:** run `5de2ea62…`, job `9296c78a-1ab8-41c5-aaa2-26d4bde853a5`, step 102:

```
Error: Cannot find module '/root/project/web'
✖ web (46.813793ms)
ℹ tests 1  ℹ pass 0  ℹ fail 1
```

**Problem:** the 10 `web/*.test.js` files (131 tests) are committed, and `node --test web/` passes locally
on Node v26.11.1. Node 24.21.0, the version in the CI image, treats the `web/` argument as a single
module to run, not as a directory to search for tests. So the job runs one "test" that can't load,
instead of the real tests.

**Impact:** the page tests are a quality gate (constitution Principle V). With it red, `main` can
never pass, and a release tag can never reach the build jobs.

**Fix (any of these):**
- use a glob that Node 22+ expands itself: `node --test 'web/*.test.js'` (quoted, so that Node,
  not the shell, expands it, the same way on every host), and update the README;
- or move the executor to the Node version developers use (`node:26`), keeping the command;
- doing both makes the command work on every current Node LTS and the newest release.

**Resolution (feature 009):** `test-web` runs `node --test 'web/*.test.js'`; `tools/test_ci_config.py` guards it (T007–T010).

## 17. Runs start on every branch push; tag runs unconfirmed

**Where:** CircleCI trigger `2f62efc0…` (preset `all-pushes`); workflow filters in
`.circleci/config.yml` (`every-commit` / `release-tags` anchors).

**Problems:**
- **Required policy:** run the quality gates on every push to `main` and on every pull request;
  run the release builds only on `vX.Y.Z` tags.
- **Current behaviour:** the `all-pushes` trigger starts a run on every push to any branch,
  including branches with no pull request. Pull request events start nothing by themselves.
- **Tags unconfirmed:** the CLI describes `all-pushes` as "every push to any branch". It isn't
  documented whether a tag push starts a run through the GitHub App trigger. If it doesn't,
  releases can't start at all. If it does and a separate tag trigger is added, every tag
  starts two runs.

**Fix:** replace the trigger with one per run kind:
- `default-branch-pushes` for `main`;
- a pull request preset (for example `only-build-prs`, "pushes to branches that have an open
  pull request") for pull requests;
- `only-tags` for tags, unless a test shows that `all-pushes` already covered them.

Keep the workflow filters as the second line of defence: the release jobs stay limited to
`/^v\d+\.\d+\.\d+$/` tags. Then push a throw-away test tag (or start a rehearsal) to confirm
that exactly one run starts per event.

## State before implementation (2026-10-12, task T001)

| Item | State |
|------|-------|
| GitHub App pipeline `f5014549-bbc3-4712-901f-e2bd315c9e6e` | one trigger `2f62efc0-51a0-461d-96c2-f5750aea445d`, provider `github_app`, preset `all-pushes` |
| OAuth pipeline `baf57313-9196-5daa-8992-aed6f4629331` | no triggers |
| GitHub rulesets on `sardylan/live-qso-map` | none (`[]`) |
| Project-level environment variables | `GITHUB_TOKEN` only (created 2024-12-01). `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` **don't exist**, so the 008 release chain would also have failed at `docker-image` (login). They are created directly in the context by T029. |

`circleci project envvar list` answered this time (earlier in the analysis it returned 403).
