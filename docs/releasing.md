# Releasing

A release is a `vX.Y.Z` tag on `main`. The CircleCI pipeline builds, tests and publishes
everything from it (`docs/ci.md`). The version lives only in `Cargo.toml`, and the tag is derived
from it, so the two can't disagree. `main` is protected, so the release commit arrives through a
pull request like any other change.

## Requirements

- **cargo-release 1.1.6:** `cargo install cargo-release --version 1.1.6 --locked`. Its settings
  are in [`release.toml`](../release.toml).
- **The GitHub CLI** (`gh`), logged in.
- **Not needed to build, test or lint:** neither tool.

## The changelog

Every pull request adds its user-visible changes under `## [Unreleased]` in
[`CHANGELOG.md`](../CHANGELOG.md), in a `### Added`, `Changed`, `Fixed`, `Removed`, `Security`
or `Notes` subsection. At release time those entries become the version's section, and the
section becomes the GitHub release notes. A release with an empty "Unreleased" is refused.

## Step 1: prepare

On an up-to-date, clean `main`:

```bash
git switch main
git pull --ff-only
git switch -c release/next
cargo release patch --no-tag --no-push --no-publish --execute     # or minor / major
git push -u origin release/next
gh pr create --base main --fill
```

The `cargo release` command:
- sets the new version in `Cargo.toml` and `Cargo.lock`;
- turns "Unreleased" into `## [X.Y.Z] - <UTC date>`;
- commits `chore(release): X.Y.Z`.

It creates no tag and pushes nothing. Without `--execute`, it only shows what it would do.

**It refuses:**

| Situation | Message |
|-----------|---------|
| uncommitted changes | `uncommitted changes detected` |
| a branch other than `main` or `release/next` | `cannot release from branch …` |
| an empty "Unreleased" | `at least 1 replacements expected, found 0` |

A refusal for an empty "Unreleased" happens after the version was already raised in
`Cargo.toml` and `Cargo.lock`. Undo it with `git restore Cargo.toml Cargo.lock`.

## Merge

The release pull request needs the quality gates green, like any other.
`tests/changelog.rs` checks the new section. Merge it, then delete `release/next`.

## Step 2: tag

```bash
git switch main
git pull --ff-only
cargo release tag --execute
cargo release push --execute
```

**What it does:**
- **Tag:** `tag` creates `vX.Y.Z` on the merged release commit.
- **Push:** `push` pushes only the tag, because the local `main` is the same as the remote one.
- **Release:** the tag starts the release chain on CircleCI.

**It refuses:**
- **The version is already tagged:** "disabled due to existing tag".
- **Uncommitted changes.**
- **`main` behind its remote:** caught only when the stale commit's version is already tagged.
  Otherwise the tag would land on an older commit. Always run `git pull --ff-only` first.

## Before tagging

- **Secrets:** check that the three publishing secrets exist (names only):
  `circleci context secret list live-qso-map-release --org gh/sardylan` (`docs/ci.md`,
  section 8).
- **Rehearsal:** if the release path changed since the last release (executors, packaging, image,
  cross), run a rehearsal first (`docs/ci.md`, section 7).

## After the tag

**The release chain:**
1. `check-version` (version, changelog section, secrets), then the five builds, the packages and
   their tests, the archives and the image tests;
2. the `-rc` image push, and the browser tests (`browser-test`) on the x86_64 binary;
3. the GitHub release, with the changelog section as notes and `SHA256SUMS`. It waits for
   `browser-test`, so a release whose map page fails in the browser is never published;
4. the Docker Hub tags `X.Y.Z` and `latest`.

**If something fails:**
- **Before publishing:** fix the cause and re-run the workflow from the failed job. Re-running
  `publish-github-release` replaces the files and keeps the notes, including notes edited by
  hand.
- **`browser-test` failed:** open its artifacts (`browser-test/`, `docs/ci.md` section 11). The
  failed step, its message and a screenshot say what broke. A network hiccup (for example the
  CDN) is fixed by re-running from failed; a real page bug needs a code change.
- **If the fix needs a code change:** release a new patch version. Never move or reuse a tag.
