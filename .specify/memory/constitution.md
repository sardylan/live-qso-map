<!--
Sync Impact Report
==================
Version change: 1.1.0 → 1.2.0
Bump rationale: MINOR, because guidance is materially expanded. Technical Constraints gains a binding
reference to the ham radio domain rules in docs/ham-radio.md (§1, HR-1 to HR-11), and Compliance
review now covers them. No principle is removed or redefined.

Modified principles: none (I–III unchanged)

Modified sections:
  - Technical Constraints: domain rules in docs/ham-radio.md §1 are binding; the old placeholder
    bullet about domain constraints is replaced.
  - Governance / Compliance review: also verifies the domain rules.

Added sections: none (new external document docs/ham-radio.md)
Removed sections: none

Templates:
  - .specify/templates/plan-template.md   ✅ updated (Constitution Check gains a "Domain rules" gate)
  - .specify/templates/spec-template.md   ✅ updated (points specs to the domain glossary and rules)
  - .specify/templates/tasks-template.md  ✅ no change needed
  - .specify/templates/commands/*.md      ⚠ not present
  - CLAUDE.md                             ✅ updated (pointer outside the managed SPECKIT block)

Follow-up TODOs:
  - specs/001-live-qso-map: checked against HR-1 to HR-11. One violation of HR-2 was found and fixed:
    the band legend was alphabetical because hello.bands was a JSON object.
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

## Technical Constraints

- Language: Rust, edition 2024. The project builds with stable `cargo` and no
  nightly features.
- Distribution: a single binary built from this repository (`cargo build --release`).
- `Cargo.lock` MUST be committed. Dependency updates are deliberate changes that
  must pass every quality gate.
- Amateur radio domain rules MUST be followed: `docs/ham-radio.md` §1 (HR-1 to HR-11).
  Examples: UTC everywhere, bands always in frequency order, "country" means a DXCC/WAE
  entity. §2 of that document is the shared glossary that specifications and code use.
- Feature-specific constraints (ingestion sources, rendering, contest formats) are defined
  in each feature's specification.

## Development Workflow

- Features follow the spec-driven flow: specification → plan (with Constitution
  Check) → tasks → implementation.
- Each plan MUST pass the Constitution Check before research and again after design.
  Any violation MUST be justified in Complexity Tracking or the design MUST change.
- Before a merge, the following MUST all succeed: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- A review MUST confirm that the tests were written before the implementation and
  that every new dependency is justified.

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
  adherence to Principles I–III and to the domain rules in `docs/ham-radio.md` §1.
  Unjustified violations block the merge.

**Version**: 1.2.0 | **Ratified**: 2026-10-08 | **Last Amended**: 2026-10-09
