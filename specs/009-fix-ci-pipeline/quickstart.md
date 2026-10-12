# Quickstart: verify feature 009

Section 1 runs on a development host; its second block needs a POSIX shell. The other sections use the CircleCI and GitHub settings
from `contracts/ci-service-setup.md`, the `circleci` CLI (logged in) and `gh`.

## 1. Local checks

Any host (Linux, macOS, Windows):

```bash
cargo test --locked --test changelog              # CHANGELOG has [Unreleased] and a section for the Cargo version
node --test 'web/*.test.js'                       # 10 test files, 131 tests, same on Node 24 and 26
python3 -m unittest discover -s tools -p 'test_*.py'   # incl. tools/test_ci_config.py (R16, R2 regressions)
circleci config validate .circleci/config.yml     # "valid"
circleci config process .circleci/config.yml | grep -c 'context: live-qso-map-release'   # 4
```

CI-only helpers (POSIX `sh`). They're optional locally and need a Unix-like shell, such as Git
Bash on Windows. The `test-tools` gate runs them on every pull request:

```bash
sh scripts/test-check-version.sh
sh scripts/test-changelog-section.sh
sh scripts/test-check-secrets.sh
sh scripts/changelog-section.sh 2.0.1             # after the prepare step: prints the 2.0.1 notes
```

## 2. Which events start a run (FR-002 to FR-006, SC-001, SC-002)

```bash
circleci project trigger list --pipeline-definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e
# one github_app trigger, preset only-build-prs (plus the weekly schedule)
circleci project trigger list --pipeline-definition-id baf57313-9196-5daa-8992-aed6f4629331
# no rows
```

Then trigger each event and count the runs with `circleci run list --limit 5`:

| Event | Expected |
|-------|----------|
| `git push origin HEAD:refs/heads/scratch/no-pr` (branch with no pull request) | no new run |
| open a pull request from that branch | one run, `test_and_build`, five gates only |
| open a **draft** pull request from another scratch branch | one run, `test_and_build`, gates only. If no run starts, drafts are skipped: see task T016 |
| push one more commit to it | one run |
| merge it | one run on `main` |
| `git tag v0.0.0 && git push origin v0.0.0` | one run: the five gates and `check-version`, which fails ("tag v0.0.0 is version 0.0.0, but Cargo.toml has …"), so no build starts |

Clean up: `git push origin :refs/tags/v0.0.0 && git tag -d v0.0.0`, then delete the branch.

Checkout log of any job (step 101): `Cloning git repository - https://github.com/sardylan/live-qso-map.git`.

## 3. Protection of `main` (FR-009, SC-003)

```bash
gh api repos/sardylan/live-qso-map/rulesets --jq '.[] | {id, name, enforcement}'
gh api repos/sardylan/live-qso-map/rulesets/<id> --jq '.bypass_actors, .rules[].type'
git push origin HEAD:main                # as a non-bypass actor: rejected
```

A pull request with a red gate shows "Merging is blocked". The admin bypass is listed as
`RepositoryRole` 5. If GitHub reports a different role, fix the JSON.

## 4. Rehearsal (FR-011 to FR-013, SC-005, SC-010)

Job list (before running it). Each missing job prints a line:

```bash
p=$(circleci config process .circleci/config.yml)
for j in build-linux-x86_64-unknown-linux-musl build-linux-aarch64-unknown-linux-musl \
         build-linux-armv7-unknown-linux-musleabihf build-macos build-windows \
         package-deb-x86_64-unknown-linux-musl package-deb-aarch64-unknown-linux-musl \
         package-deb-armv7-unknown-linux-musleabihf \
         test-deb-debian-12-amd64 test-deb-debian-13-amd64 test-deb-ubuntu-22-04-amd64 \
         test-deb-ubuntu-24-04-amd64 package-archives image-test-amd64 rehearsal-summary; do
  printf '%s\n' "$p" | grep -Eq "(name: |- )$j:?\$" || echo "missing: $j"
done
# after US5 also: test-deb-<distro>-arm64 (×4), image-test-arm64
```

Run it:

```bash
circleci pipeline run --definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e \
  --branch feature/009-fix-ci-pipeline --param rehearsal=true
circleci run watch <run-id>
```

**Expected:**
- **Jobs:** only the `rehearsal` workflow runs, and it is green. It includes the three
  `build-linux` jobs, `build-macos`, `build-windows`, the three `package-deb` jobs, eight
  `test-deb` jobs (four amd64, four arm64), `package-archives`, both `image-test` jobs and
  `rehearsal-summary`.
- **Artefacts:** `rehearsal-summary` shows the artefacts, and prints
  `publishing secrets: not checked (rehearsal)`.
- **Nothing published:**
  - `gh release list` is unchanged;
  - `crane ls sardylan/live-qso-map` shows no new tag.

If `--param rehearsal=true` arrives as a string, the run uses `test_and_build` instead (gates
only). In that case, use the API with a JSON boolean:
`circleci api -X POST api/v2/project/gh/sardylan/live-qso-map/pipeline/run -d '{"definition_id":"f5014549-…","config":{"branch":"…"},"checkout":{"branch":"…"},"parameters":{"rehearsal":true}}'`.

## 5. Release steps, dry run (US6, FR-029 to FR-032)

On an up-to-date `main` with entries under "Unreleased":

```bash
git switch -c release/next
cargo release version patch                  # dry run: "Upgrading live-qso-map from X to Y"
cargo release replace                        # dry run: shows the CHANGELOG replacement
git switch main && git branch -D release/next
```

**Refusals to check:**
- with "Unreleased" empty, `cargo release replace` fails on `exactly = 1`;
- with a dirty tree, cargo-release refuses;
- on another branch name, `allow-branch` refuses.

After a real `version --execute`: `git diff Cargo.lock` shows the root version changed. If it
doesn't, add `cargo check` before `commit` in `docs/releasing.md` (research R7).

## 6. Secrets (FR-013, FR-014, SC-007)

```bash
circleci context env-var list live-qso-map-release --org gh/sardylan   # three names, no values
circleci project envvar list                                        # none of the three names
```

`circleci config process` shows `context:` only on `check-version`, `image-push`,
`publish-github-release` and `docker-promote`. A rehearsal or pull request run never shows the
context in its "Preparing environment variables" step.

## 7. Weekly newest toolchain (FR-026)

```bash
circleci pipeline run --definition-id f5014549-… --branch main --param newest-toolchain=true
```

**Expected:**
- only `lint-stable` and `test-rust-stable` run;
- the commit's required checks (`ci/circleci: lint`, …) are not changed.

## 8. First release, 2.0.1 (FR-028, FR-018, SC-006, SC-008, SC-011)

Follow `docs/releasing.md`: prepare (patch), merge, tag.

**Expected:**
- **The run:** `check-version` passes (version, changelog section, three secrets present), and
  the whole release chain is green on the first attempt.
- **GitHub release:** `gh release view v2.0.1` shows all 008 artefacts and `SHA256SUMS`, and the
  notes equal `sh scripts/changelog-section.sh 2.0.1`.
- **Docker Hub:** `crane ls sardylan/live-qso-map` lists `2.0.1` and `latest`, with no
  `2.0.1-rc`. `crane digest` is the same for `2.0.1` and `latest`.
- **Notes survive a re-run:**
  1. edit the release notes by hand;
  2. re-run `publish-github-release` from the CircleCI UI;
  3. check that the notes are unchanged.
- **`v2.0.0`:** `git ls-remote --tags origin v2.0.0` still points at `210528a`.

## 9. Caches (FR-019 to FR-021, SC-009)

1. Run the gates twice on the same commit (a re-run of the workflow). Compare the `test-rust`
   durations: the second must be at least 30% shorter.
2. Change `rust-version` in a branch. Check that the "Restoring cache" step finds no key for
   the old version.
