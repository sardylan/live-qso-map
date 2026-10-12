# Contract: Release Process (feature 009)

The maintainer's side of a release. It is documented in `docs/releasing.md`. Needs:
- cargo-release 1.1.6 (`cargo install cargo-release --version 1.1.6 --locked`);
- the GitHub CLI, logged in.

Neither is needed to build, test or lint (FR-029).

## `release.toml`

```toml
publish = false
tag-name = "v{{version}}"
allow-branch = ["main", "release/next"]
pre-release-commit-message = "chore(release): {{version}}"
pre-release-replacements = [
  { file = "CHANGELOG.md", search = "## \\[Unreleased\\]\n\n###", replace = "## [Unreleased]\n\n## [{{version}}] - {{date}}\n\n###", exactly = 1 },
]
```

## Step 1: prepare

Precondition: on `main`, up to date with `origin/main`, clean tree, and "Unreleased" has
entries.

```
git switch main && git pull --ff-only
git switch -c release/next
cargo release <patch|minor|major> --no-tag --no-push --no-publish --execute
git push -u origin release/next
gh pr create --base main --fill
```

The separate steps (`cargo release version`, `replace`, `commit`) were rejected after testing
(T041): `version` and `replace` check neither a clean tree nor `allow-branch`. The full command
checks both before changing anything.

**Result:**
- one commit `chore(release): X.Y.Z` on `release/next`, holding:
  - `Cargo.toml` and `Cargo.lock` at `X.Y.Z`;
  - `CHANGELOG.md` with `## [X.Y.Z] - <UTC date>` under an empty `## [Unreleased]`;
- a pull request into `main`;
- no tag.

**Refusals (non-zero exit):**

| Situation | Refused by |
|-----------|------------|
| "Unreleased" empty | `exactly = 1` replacement (after the version bump: undo with `git restore Cargo.toml Cargo.lock`) |
| dirty tree | cargo-release (exit 101) |
| branch not in `allow-branch` | cargo-release (exit 101) |
| missing `--execute` | dry run: prints the plan only |

## Merge

The pull request needs the five required checks green (ruleset). `tests/changelog.rs` checks the
new section in `test-rust`. Merge as usual. Delete `release/next` after the merge.

## Step 2: tag

Precondition: the release pull request is merged.

```
git switch main
git pull --ff-only
cargo release tag --execute
cargo release push --execute
```

**Result:**
- tag `vX.Y.Z` on the merged release commit;
- only the tag is pushed, because the local `main` equals the remote;
- the release chain starts (`contracts/pipeline.md`).

**Refusals:**

| Situation | Refused by |
|-----------|------------|
| the tag already exists | cargo-release: "disabled due to existing tag" (exit 2) |
| dirty tree | cargo-release |
| `main` behind the remote | caught only when the stale commit's version is already tagged; otherwise the documented `git pull --ff-only` is the guard |

## After the release

- **GitHub release:** its notes are the version's changelog section, and they can be edited by
  hand. Re-running `publish-github-release` keeps them (FR-015).
- **Problems:**
  - a failure before `publish-github-release`: fix the cause and re-run the workflow from
    failed;
  - a fix that needs code: a new patch version. Tags are never moved.

## First use: 2.0.1

`CHANGELOG.md` is created by this feature with:
- `## [Unreleased]`, holding this feature's entries;
- a `### Notes` entry about 2.0.0;
- a line pointing to the GitHub history for 1.0.0 and earlier.

After this feature's pull request is merged:
1. `prepare` (patch) produces 2.0.1;
2. its pull request is merged;
3. `tag` pushes `v2.0.1`.

`v2.0.0` is never touched.
