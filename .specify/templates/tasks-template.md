---
description: "Task list template for feature implementation"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: MANDATORY (Constitution Principle I). Test tasks come before the
implementation tasks they cover and MUST fail before implementation begins.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, ...)
- Include exact file paths in descriptions

## Phase 1: Setup

- [ ] T001 Add justified dependencies to `Cargo.toml` (see plan Dependency Justification)
- [ ] T002 [P] Ensure `#![forbid(unsafe_code)]` and lint config are in place

---

## Phase 2: Foundational (blocking prerequisites)

- [ ] T003 [P] Write failing tests for shared types/parsers in `src/[module].rs` / `tests/`
- [ ] T004 Implement shared types/parsers to make T003 pass

**Checkpoint**: Foundation ready, so user story work can begin.

---

## Phase 3: User Story 1 - [Title] (Priority: P1) 🎯 MVP

**Goal**: [What this story delivers]
**Independent Test**: [How to verify on its own]

### Tests for User Story 1 (write first, confirm they FAIL)

- [ ] T010 [P] [US1] Unit tests incl. malformed/edge input in `src/[module].rs`
- [ ] T011 [P] [US1] Integration test in `tests/[name].rs`

### Implementation for User Story 1

- [ ] T012 [US1] Implement [component] in `src/[module].rs`
- [ ] T013 [US1] Error handling via `Result` (no `unwrap`/`panic!` in runtime paths)

**Checkpoint**: User Story 1 is functional and testable on its own.

---

[Repeat phases for additional user stories]

---

## Phase N: Polish & Quality Gates

- [ ] TXXX `cargo fmt --check` passes
- [ ] TXXX `cargo clippy --all-targets -- -D warnings` passes
- [ ] TXXX `cargo test` passes
- [ ] TXXX Platform-specific code is `cfg`-gated with a path for every configured target (Principle V)
- [ ] TXXX Review: each new dependency justified; no unjustified complexity

## Dependencies & Execution Order

- Setup → Foundational → User Stories (priority order or parallel) → Polish
- Within each story: tests → models → services → integration
