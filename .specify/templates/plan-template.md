# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]
**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

## Summary

[Primary requirement + technical approach from research]

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2)
**Primary Dependencies**: [crates, each justified below, or NEEDS CLARIFICATION]
**Storage**: [if applicable or N/A]
**Testing**: `cargo test`
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary
**Performance Goals**: [domain-specific or NEEDS CLARIFICATION]
**Constraints**: [domain-specific or NEEDS CLARIFICATION]
**Scale/Scope**: [e.g., QSOs per contest, concurrent viewers or NEEDS CLARIFICATION]

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [ ] **I. Test-First**: tests for each behavior are planned before implementation tasks;
      parsing/conversion logic has edge-case and malformed-input tests planned.
- [ ] **II. Rust Safety & Quality**: no `unsafe` (or justified below); no
      `unwrap`/`expect`/`panic!` for recoverable errors in runtime paths; plan for
      `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` to pass;
      design follows `docs/rust-guidelines.md` (crate `enum Error` + `Result<T>` alias,
      no error-handling crates).
- [ ] **III. Simplicity & Minimal Dependencies**: each new crate justified in the table
      below; still a single self-contained binary; no speculative abstractions.
- [ ] **Domain rules** (`docs/ham-radio.md` §1): UTC times, bands in frequency order
      (never alphabetical), ADIF band/mode names, DXCC/WAE entities as countries, positions
      keep precision and source, dupes are kept.
- [ ] **IV. Persistent Viewer Settings**: every viewer setting on the page is saved in local
      storage and survives a browser restart; reads fall back to defaults; tests planned.
- [ ] **V. Portable Development, Pipeline-Built Releases**: builds and tests run natively on
      Linux, macOS and Windows; no local cross-compiling; platform-specific code is `cfg`-gated
      with a path for every configured target; new crates build for every configured target.
- [ ] **Interface rules** (`docs/http-api.md`): HTTP bodies and WebSocket messages are JSON
      (text frames with `type`); every error response has a JSON `{"error","message"}` body;
      snake_case fields, UTC times, arrays where order matters; contract tests planned.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── main.rs
└── [modules]
tests/
└── [integration tests]
```

**Structure Decision**: [Document the selected structure]

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
