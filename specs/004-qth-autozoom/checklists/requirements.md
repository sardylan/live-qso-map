# Specification Quality Checklist: QTH Marker, Auto-Zoom and Map Panels

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
- Four decisions were settled with the project owner on 2026-10-09, before writing: QTH source,
  entity framing, auto-zoom behaviour and defaults (see Clarifications). No clarification markers
  were needed.
- The Assumptions still name the browser's local storage and a house emoji or symbol. These
  continue the feature-002 conventions; they are not new technology choices.
- Domain rules checked:
  - HR-1: UTC time in the box;
  - HR-6: "ham country" means the DXCC entity;
  - HR-7: the house popup shows position source and precision;
  - §2.10: great-circle lines.
- Depends on features 001–003 (all merged into `main`).
- Owner confirmed (2026-10-09) the defaults in Assumptions and the edge cases:
  - black lines replace the band colours; markers keep theirs; lines may be slightly transparent;
  - a 500 km fallback radius;
  - automatic return to the globe; automatic switches are never saved;
  - box rows: callsign, band, mode/submode, UTC time; five rows, fixed; the box follows the limit
    selector and the band filter;
  - a metric scale only;
  - a city-scale zoom cap, and at most one refit per second;
  - the house popup;
  - station data and the entity extent are sent in the handshake, with a protocol version bump.
