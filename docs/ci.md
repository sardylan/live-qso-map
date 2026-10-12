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
fork (section 5). The quality gates are the jobs `lint`, `test-rust`, `test-web` and
`test-tools`.

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

`main` is protected by a GitHub ruleset, kept in
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

## 10. Local reproduction

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
