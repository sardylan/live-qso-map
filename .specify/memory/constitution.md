<!--
Sync Impact Report
==================
Version change: 1.5.0 → 1.6.0
Bump rationale: MINOR, because the Development Workflow gains two mandatory rules (feature branch
naming, branch deletion after completion). No principle is removed or redefined.

Modified principles: none (I–V unchanged)
Added principles: none

Modified sections:
  - Development Workflow: feature branches are named feature/NNN-title and are deleted, locally
    and on the remote, once the feature is merged into main.

Added sections: none
Removed sections: none

Templates and runtime files:
  - .specify/templates/spec-template.md          ✅ updated (Feature Branch: feature/[###-feature-name])
  - .specify/templates/plan-template.md          ✅ updated (Branch: feature/[###-feature-name])
  - .specify/templates/tasks-template.md         ✅ no change needed (refers to specs/, not branches)
  - .specify/templates/commands/*.md             ⚠ not present
  - .specify/scripts/bash/create-new-feature.sh  ✅ updated (creates feature/NNN-short-name)
  - .specify/scripts/bash/common.sh              ✅ updated (validates feature/NNN-; maps branch to specs/NNN-*)
  - .specify/scripts/bash/update-agent-context.sh ✅ updated (labels CLAUDE.md by feature id NNN-title)
  - CLAUDE.md, README.md                         ✅ no change needed

Follow-up TODOs:
  - The spec-kit plugin's own /speckit-specify instructions still describe NNN-short-name
    branches; the repository script is authoritative and now creates feature/NNN-short-name.
-->

# live-qso-map Constitution

live-qso-map keeps a live map of every QSO an amateur radio station makes during a
contest.

## Core Principles

### I. Test-First Development (NON-NEGOTIABLE)

- Tests MUST be written before the implementation they cover. They MUST be seen
  failing first, and only then may the implementation be written (Red → Green → Refactor).
- Every behavior change MUST come with a test that fails without the change.
  Every bug fix MUST include a regression test that reproduces the bug.
- `cargo test` MUST pass on every commit merged to the main branch. Ignored tests
  (`#[ignore]`) MUST carry a comment that gives the reason and a follow-up reference.
- Parsing and conversion logic (QSO records, locators, coordinates, timestamps) MUST
  have unit tests that cover malformed and edge-case input, not just the happy path.

Rationale: during a contest there is no time to debug a broken map. The tests are the
only reliable evidence that the tool behaves correctly before it goes on air.

### II. Rust Safety & Quality Gates

- `unsafe` code is forbidden unless a written justification sits next to it and the
  plan's Complexity Tracking records it. The crate SHOULD declare
  `#![forbid(unsafe_code)]` while no justified exception exists.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` MUST pass with
  no warnings.
- Runtime code paths MUST NOT use `unwrap()`, `expect()` or `panic!` to handle
  recoverable errors. Errors MUST be propagated with `Result` and reported with
  context. `unwrap`/`expect` are allowed only in tests and for invariants that are
  documented as impossible to violate.
- Invalid or unexpected input (malformed QSO data, network glitches) MUST be logged
  and skipped. It MUST NOT crash the process.
- Rust code MUST follow `docs/rust-guidelines.md`. In particular, every fallible
  function MUST return the crate-wide `Result<T>` alias over a single, hand-written
  `enum Error` defined in `src/errors.rs`. No error-handling crates are allowed.

Rationale: a crash mid-contest loses live tracking. The compiler and linters are cheap
guards that catch whole classes of defects before they reach the operator.

### III. Simplicity & Minimal Dependencies

- Build the simplest thing that satisfies the current specification (YAGNI). Do not
  add speculative abstractions, configuration knobs or extension points.
- Every new crate dependency MUST be justified in the plan: why it is needed, what
  the alternatives were, and why the standard library or an existing dependency is
  not enough. Prefer well-maintained crates with small transitive dependency trees.
- The project MUST ship as a single self-contained binary. It MUST NOT require
  external runtime services unless a specification explicitly demands one.
- Any added complexity that a principle would otherwise reject MUST be recorded in the
  plan's Complexity Tracking table, together with the simpler alternative that was
  rejected.

Rationale: contest stations run on varied, often modest hardware, sometimes with poor
connectivity. Fewer moving parts mean an easier setup and fewer failures on the day.

### IV. Persistent Viewer Settings

- Every setting a viewer can change on the map page MUST be saved in the browser's local
  storage, so that it survives closing and reopening the browser. Examples: the map's
  "Last N", the projection, auto-zoom, the last-QSO box choices.
- Reading a saved setting MUST NOT fail the page: missing, unknown or unreadable values, and a
  browser that refuses storage, MUST fall back to the setting's default.
- Settings chosen by the operator in the service configuration are not viewer settings. They
  are not stored in the browser.

Rationale: the map often runs unattended on a wall screen or kiosk that gets restarted during a
contest. A viewer's choices must come back by themselves, without anyone setting them again.

### V. Portable Development, Pipeline-Built Releases

- Development MUST work natively on Linux, macOS and Windows. Building, testing and linting use
  only `cargo`, `node` and `python3` with its standard library; no step may require a specific
  host OS, shell, container runtime or emulator. A helper that runs only on some hosts (for
  example a Bash script) MUST be optional or have a portable equivalent.
- Cross-compilation MUST happen only in the CI pipeline. Developers build and test for their own
  host and MUST NOT need `cross`, extra Rust targets or emulation. Release binaries MUST come
  only from the pipeline.
- The code MUST stay buildable for every configured target at all times. The configured targets
  are the release targets in `.circleci/config.yml`: x86_64, aarch64 and armv7 Linux (static
  musl), aarch64 macOS and x86_64 Windows (MSVC).
- Platform-specific code MUST be gated with `cfg` and MUST have a working path on every
  configured target. A new dependency MUST build for every configured target.
- The pipeline MUST run the quality gates (`cargo fmt --check`, `cargo clippy`, `cargo test`, the
  frontend and tool tests) on every commit to the main branch. It MUST build release binaries for
  the configured targets only for release tags (`vX.Y.Z`).

Rationale: contributors work on whatever machine they have, while stations run the tool on PCs,
Raspberry Pis and servers. Keeping cross-compilation in one reproducible place avoids
per-developer toolchains. Gating every commit on main keeps it releasable, while the slower
multi-platform builds run only when a release is cut.

## Technical Constraints

- Language: Rust, edition 2024. The project builds with stable `cargo` and no
  nightly features.
- Distribution: one self-contained binary per configured target (Principle V), built by the
  pipeline. `cargo build --release` builds it for the developer's own host.
- `Cargo.lock` MUST be committed. Dependency updates are deliberate changes that
  must pass every quality gate.
- Amateur radio domain rules MUST be followed: `docs/ham-radio.md` §1 (HR-1 to HR-11).
  Examples: UTC everywhere, bands always in frequency order, "country" means a DXCC/WAE
  entity. §2 of that document is the shared glossary that specifications and code use.
- Interfaces exposed over HTTP MUST follow `docs/http-api.md` (API-1 to API-5): every REST
  request and response body and every WebSocket message is JSON, and every error response has a
  JSON body. The map page's static files are not payloads and are exempt.
- Feature-specific constraints (ingestion sources, rendering, contest formats) are defined
  in each feature's specification.

## Development Workflow

- Features follow the spec-driven flow: specification → plan (with Constitution
  Check) → tasks → implementation.
- Every feature MUST be developed on a git branch named `feature/NNN-title`, where `NNN` is the
  three-digit feature number and `title` is the feature's short kebab-case title, the same as its
  `specs/NNN-title/` directory (for example `feature/008-platform-packaging`).
- When a feature's work is complete, that is merged into the main branch, its feature branch
  MUST be deleted, both locally and on the remote.
- Each plan MUST pass the Constitution Check before research and again after design.
  Any violation MUST be justified in Complexity Tracking or the design MUST change.
- Before a merge, the following MUST all succeed: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- A review MUST confirm that the tests were written before the implementation and
  that every new dependency is justified.
- If a release tag's target build fails, nothing is published: the fix lands on main and a new
  release tag is cut.

## Governance

- This constitution takes precedence over other project practices and guidance. When a
  conflict arises, the constitution wins until it is amended.
- Amendments: propose the change in writing (a PR that edits this file) with its
  rationale, update the Sync Impact Report, and propagate the change to the dependent
  templates in `.specify/templates/` in the same change.
- Versioning follows semantic versioning:
  - MAJOR: a principle or governance rule is removed or redefined in a way that is
    not backward compatible.
  - MINOR: a new principle or section is added, or existing guidance is materially
    expanded.
  - PATCH: clarifications, wording and typo fixes that do not change meaning.
- Compliance review: every plan's Constitution Check and every code review MUST verify
  adherence to Principles I–V, to the domain rules in `docs/ham-radio.md` §1 and to the
  interface rules in `docs/http-api.md`.
  Unjustified violations block the merge.

**Version**: 1.6.0 | **Ratified**: 2026-10-08 | **Last Amended**: 2026-10-11
