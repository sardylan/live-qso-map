# Specification Quality Checklist: Configurable Last-QSO Box

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-09
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- Items marked incomplete require spec updates before `/speckit-clarify` or `/speckit-plan`
- The owner settled four decisions on 2026-10-09, before writing (see Clarifications):
  - the box is independent of the map;
  - box sizes 3, 5, 10, 15 and 20;
  - choices are remembered in the browser;
  - an invalid column list stops startup.

  No clarification markers were needed.
- This changes a feature-005 default: newest at the bottom replaces newest first. "Newest at the
  top" keeps the old order available.
- How the page receives max(map, box) QSOs is left to the plan (Assumptions).
- Owner addition (2026-10-09): the `cqzone` column, in the defaults between `time` and
  `country`. It comes from cty.dat, with per-callsign overrides, like the entity (FR-013).
- Domain rules checked: HR-1 (UTC date and time), HR-6 (`country` is the DXCC/WAE entity).
- Depends on features 001 (configuration layers), 004 (box) and 005 (entity, date), all merged
  into `main`.
