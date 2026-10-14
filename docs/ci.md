# Continuous integration

How the CircleCI pipeline is set up, what starts it, and what to check when it misbehaves. The
pipeline itself is `.circleci/config.yml`. This file records the settings that live **outside**
the repository, on CircleCI and on GitHub (feature 009, FR-008).

| Item | Value |
|------|-------|
| CircleCI project | `gh/sardylan/live-qso-map`, id `e4ace6e1-9360-4882-9faf-21f77e871b0e` |
| CircleCI organisation | `sardylan`, id `332144a0-85cc-43d6-b55d-496fed496e29` |
| GitHub repository | `sardylan/live-qso-map`, id `883686784` |

## 1. Run kinds

| Kind | Started by | Workflow | What runs |
|------|------------|----------|-----------|
| gate | a push to `main`, or a pull request opened, re-opened or pushed to | `test_and_build` | the quality gates |
| release | a push of a tag `vX.Y.Z` | `test_and_build` | the gates, then the release chain |
| rehearsal | the maintainer, by hand (section 7) | `rehearsal` | every build, package and image test of a release; nothing is published |
| scheduled | the weekly schedule (section 9) | `newest_toolchain` | fmt, clippy and tests on the newest stable Rust |

A push to a branch with no pull request starts nothing, and neither does a pull request from a
fork (section 5). The quality gates are the jobs `lint`, `test-rust`, `test-web`, `test-tools`
and `audit`.

## 2. Repository connection

The project uses the **CircleCI GitHub App**, which clones over HTTPS with an App token. No SSH
key or deploy key is needed, even though the repository is public.

| Item | Value |
|------|-------|
| GitHub App | installed on `sardylan` (CircleCI → Organization → VCS Connections), with access to `live-qso-map` |
| Pipeline | `f5014549-bbc3-4712-901f-e2bd315c9e6e`, "live-qso-map (GitHub App)", config and checkout through the App |
| Legacy OAuth pipeline | `baf57313-9196-5daa-8992-aed6f4629331`. CircleCI doesn't allow deleting it. It MUST keep **no trigger**: an OAuth trigger would start a second run for every event, which fails at checkout over SSH |

Until 2026-10-12, the project used only the OAuth connection. That connection clones over SSH, and
GitHub had no key for it, so every run failed at checkout
(`specs/009-fix-ci-pipeline/reference/circleci-review.md`, item 0).

## 3. Triggers

One GitHub App trigger with the preset **`only-build-prs`** starts a run for:
- pushes to `main`;
- pull requests being opened or re-opened, and pushes to their branches;
- tag pushes.

Exactly one run starts for each event.

```bash
circleci project trigger list --pipeline-definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e
circleci project trigger list --pipeline-definition-id baf57313-9196-5daa-8992-aed6f4629331  # must be empty
```

To recreate the event trigger:

```bash
circleci project trigger create --pipeline-definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e \
  --provider github_app --repo-id 883686784 --event-preset only-build-prs
```

Don't add a second trigger for `main`, tags or pull requests: every trigger that matches an event
starts its own run.

## 4. Protection of `main`

`main` is protected by a GitHub ruleset (id `24935176`, applied 2026-10-12), kept in
[`deploy/github/main-ruleset.json`](../deploy/github/main-ruleset.json):
- changes arrive only through pull requests;
- merging needs every quality gate green (`ci/circleci: <job>`, from the CircleCI App);
- force pushes and deletion are blocked;
- the repository admin can bypass the rules in an emergency.

```bash
gh api -X POST repos/sardylan/live-qso-map/rulesets --input deploy/github/main-ruleset.json  # first time
gh api -X PUT repos/sardylan/live-qso-map/rulesets/<id> --input deploy/github/main-ruleset.json  # changes
gh api repos/sardylan/live-qso-map/rulesets --jq '.[] | {id, name, enforcement}'
```

**Renaming or adding a gate job:** change the JSON in the same pull request, and apply it after
the new job has reported once. Otherwise merges wait for a check that never comes.

## 5. Pull requests from forks

CircleCI's GitHub App never builds pull requests from forks, so fork code never runs on CI before
review and never sees any secret. Their required checks never report, so they can't be merged as
they are. After reviewing the code:

```bash
gh pr checkout <n>
git push origin HEAD:refs/heads/contrib/<n>
gh pr create --head contrib/<n> --base main --title "<title> (from #<n>)"
```

The gates run on the new pull request. Merge that one, and close `#<n>` with a link to it. The
commits keep the contributor as author.

## 6. Diagnosis

| Symptom | Check |
|---------|-------|
| No run starts | Does the GitHub App still have access to the repository (GitHub → Settings → Applications → CircleCI → Repository access)? Does the trigger still exist (section 3)? Was it a push to a branch with no pull request (no run is expected)? |
| Two runs per event | An OAuth trigger was recreated, or a second App trigger was added: keep only the one from section 3 |
| Checkout clones `git@github.com:…` and fails with `Permission denied (publickey)` | The run came from the OAuth pipeline: delete its trigger |
| A gate is red | Open the job (`circleci run list`, `circleci job get <id>`, `circleci job output get <id> --step-num <n>`); the step output names the failing check |

## 7. Rehearsal

A rehearsal runs everything a release builds and tests, but publishes nothing:
- the five builds;
- the three Debian packages, and their install/upgrade/remove tests;
- the Windows and macOS archives;
- the container image smoke test;
- the browser tests (section 11).

It receives no secrets, and it never starts by itself. Run one before tagging whenever the release
path changed: executors, cross images, packaging or the image.

**Auto-cancel:** the project has CircleCI's "auto-cancel redundant builds" setting on. A
rehearsal started on a branch cancels that branch's gate run if it is still running, and a later
push to the branch can cancel the rehearsal. So either start rehearsals on a branch of their own
(for example `rehearsal/<topic>`, which has no pull request), or wait until the gates have
finished. If a gate run was cancelled, re-run it (`circleci workflow rerun <workflow-id>`, or
"Rerun workflow" in the UI); otherwise the pull request's required checks never complete.

```bash
circleci pipeline run --definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e \
  --branch <branch> --param rehearsal=true
circleci run watch <run-id>
```

If the run starts `test_and_build` instead (the parameter arrived as a string), use the API with a
JSON boolean:

```bash
circleci api -X POST api/v2/project/gh/sardylan/live-qso-map/pipeline/run -d '{
  "definition_id": "f5014549-bbc3-4712-901f-e2bd315c9e6e",
  "config": {"branch": "<branch>"}, "checkout": {"branch": "<branch>"},
  "parameters": {"rehearsal": true}}'
```

**Outputs:**
- **File names:** they use the `Cargo.toml` version with a `-rehearsal.<sha>` suffix.
- **Where:** they are stored as artefacts of the `rehearsal-summary` job.
- **Secrets:** the summary prints `publishing secrets: not checked (rehearsal)`. To check the
  secrets' names, see section 8.

## 8. Secrets

The publishing secrets live in the CircleCI context **`live-qso-map-release`**, never in
project-level variables:

| Variable | What it is |
|----------|------------|
| `GITHUB_TOKEN` | fine-grained GitHub token, repository `sardylan/live-qso-map` only, permission `contents: write` |
| `DOCKERHUB_USERNAME` | Docker Hub account that owns `sardylan/live-qso-map` |
| `DOCKERHUB_TOKEN` | Docker Hub personal access token, Read/Write/Delete on `sardylan/live-qso-map` (Delete removes the `-rc` tag) |

The context has two restrictions, which CircleCI enforces whatever the config says:
- the project `e4ace6e1-9360-4882-9faf-21f77e871b0e`;
- the expression `pipeline.git.tag starts-with "v"`.

Runs for `main`, pull requests, rehearsals and the weekly schedule have no tag, so they can't
receive it. Only four jobs declare it: `check-version`, `image-push`, `publish-github-release` and
`docker-promote`.

`check-version` runs `scripts/check-secrets.sh`, which stops the release before any build when a
secret is missing, and names it without printing any value.

```bash
circleci context create live-qso-map-release --org gh/sardylan
circleci context restriction create live-qso-map-release --type project \
  --value e4ace6e1-9360-4882-9faf-21f77e871b0e --org gh/sardylan
circleci context restriction create live-qso-map-release --type expression \
  --value 'pipeline.git.tag starts-with "v"' --org gh/sardylan
circleci context secret list live-qso-map-release --org gh/sardylan    # names only, before tagging
circleci project envvar list                                        # must not list them
```

**Entering or rotating a value:** use the CircleCI UI (Organization Settings → Contexts) or the
CLI prompt, never a file or a command-line argument. Rotate a token by creating the new one,
replacing the value in the context, then revoking the old token.

## 9. Weekly newest-toolchain run

CI pins Rust (`rust-version` in `.circleci/config.yml`, together with the `Dockerfile`;
`docs/rust-guidelines.md` §2.2). A weekly schedule runs fmt, clippy and the tests on the newest
**stable** Rust, so that new lints or breakages show up before the pin is raised.

| Item | Value |
|------|-------|
| Trigger | schedule, Monday 04:00 UTC, branch `main`, parameter `newest-toolchain: true` |
| Workflow | `newest_toolchain`: jobs `lint-stable`, `test-rust-stable` |
| Blocking | never. The job names aren't required checks, and releases don't depend on them |
| Failures | a red `newest_toolchain` workflow in CircleCI, plus CircleCI's e-mail for failed runs |

When it fails, fix the code on a pull request, or raise the pin deliberately: change
`rust-version` and the `Dockerfile`'s `RUST_VERSION` together, after fmt, clippy and the tests
pass on the new version.

The schedule is trigger `04b1cb13-f913-41b9-9424-e36df2ee3628`, created on 2026-10-13 through the
CircleCI API: although CircleCI's docs say schedules for GitHub App pipelines can't be created that
way, the call worked. A manual run with the same parameter (`ac823b8e`) ran only `lint-stable` and
`test-rust-stable`, both green, and left the required checks untouched.

To recreate it (or in the UI under Project Setup → Schedule):

```bash
circleci api -X POST api/v2/projects/e4ace6e1-9360-4882-9faf-21f77e871b0e/pipeline-definitions/f5014549-bbc3-4712-901f-e2bd315c9e6e/triggers -d '{
  "event_name": "weekly newest toolchain",
  "event_source": {"provider": "schedule", "schedule": {"cron_expression": "0 4 * * 1", "attribution_actor": "system"}},
  "config_ref": "main", "checkout_ref": "main",
  "parameters": {"newest-toolchain": true}}'
```

## 10. Audit gate

`audit` runs two checks:
- `cargo deny check advisories licenses` (cargo-deny 0.20.2, settings in
  [`deny.toml`](../deny.toml));
- `cargo doc` with warnings denied. rustdoc's `bare_urls` lint is allowed, because the options'
  doc comments are also the `--help` text.

**Fixing a red `audit`:**
- **A new advisory:** update the dependency (`cargo update -p <crate>`). If no fix exists and
  the advisory doesn't apply, add its id to `ignore` in `deny.toml`, with the reason and a
  follow-up.
- **A new licence:** check that it is compatible with GPL-3.0-or-later, then add it to `allow`
  in `deny.toml`. If it isn't, replace the dependency.
- **A documentation warning:** fix the doc comment.

## 11. Browser tests

`browser-test` opens the release's map page in a real browser before anything is published
(feature 012). It runs `node tests/browser/run.mjs` against the x86_64 release binary:
- **Session:** QSOs over UDP, the settings panel, the three themes, a reload, a restart of the
  program and a band-group choice (`tests/browser/README.md`).
- **Failures:** a failed request to the program, a script error, a missing control, or a missed
  time limit. Base-map tiles and map-library warnings never fail it.

**When:** on release tags and in the rehearsal (section 7) only. Pull requests, `main` pushes and
the weekly run (section 9) never run it.

**What it blocks:** `publish-github-release` requires it, and `docker-promote` requires
`publish-github-release`. A failure therefore stops both the GitHub release and the Docker Hub
tags. The `-rc` image may already be pushed; it is never promoted.

**Where to look:** the job's Artifacts tab, folder `browser-test/`:
- `report.json`: steps, results and times;
- `console.log`: every console message, exception and failed request, with `FATAL` on the ones
  that failed the run;
- `program-a.log` and `program-b.log`: the program's own output;
- one screenshot per tested state, plus `NN-<step>-failed.png` for the failed step.

The job stores the artifacts first and fails afterwards, so they are there for a failed run too.
"Rerun workflow from failed" runs `browser-test` again: it keeps no cache.

**Its pins:** the image `cimg/node:24.21.0-browsers`, which holds Chrome's libraries but not
Chrome, and Chrome for Testing 154.0.8037.92 with its SHA-256. To bump Chrome:
1. pick a version at least two weeks old from
   `https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json`;
2. download its `linux64` `chrome-linux64.zip` and compute its SHA-256;
3. update the URL and the hash in the `browser-test` job and in `tools/test_ci_config.py`;
4. run a rehearsal.

**Locally:** `cargo build --release`, then `node tests/browser/run.mjs`. You need Node 22+ and
Chrome or Chromium; `--headed` shows the window. It isn't one of the merge gates.

## 12. Local reproduction

Everything the gates run can be repeated on any host with `cargo`, `node` and `python3`:

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
node --test 'web/*.test.js'
python3 -m unittest discover -s tools -p 'test_*.py'
```

These parts are CI-only:
- the `scripts/*.sh` helpers are POSIX `sh`. Locally they need a Unix-like shell, such as Git Bash
  on Windows;
- the `audit` gate needs cargo-deny 0.20.2 (`cargo install cargo-deny --version 0.20.2 --locked`),
  which is optional locally.
