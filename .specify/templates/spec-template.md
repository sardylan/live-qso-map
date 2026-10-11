# Feature Specification: [FEATURE NAME]

**Feature Branch**: `feature/[###-feature-name]`
**Created**: [DATE]
**Status**: Draft
**Input**: User description: "$ARGUMENTS"

<!--
  Use the amateur radio terms defined in docs/ham-radio.md §2 (QSO, entity, band, locator,
  dupe…). Requirements MUST NOT contradict its rules in §1 (e.g. UTC times, bands listed in
  frequency order).
-->

## User Scenarios & Testing *(mandatory)*

<!--
  User stories MUST be prioritized (P1, P2, P3...) and each MUST be independently
  testable: implementing just one still delivers a usable slice of value.
-->

### User Story 1 - [Brief Title] (Priority: P1)

[Describe this user journey in plain language]

**Why this priority**: [Explain the value]

**Independent Test**: [How this can be tested on its own]

**Acceptance Scenarios**:

1. **Given** [initial state], **When** [action], **Then** [expected outcome]

---

### Edge Cases

- What happens when [malformed or unexpected input, e.g. invalid QSO data]?
- How does the system handle [connection loss / restart mid-contest]?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST [specific capability]
- **FR-002**: System MUST [NEEDS CLARIFICATION: unclear aspect]

### Key Entities *(include if feature involves data)*

- **[Entity]**: [What it represents, key attributes without implementation]

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: [Measurable, technology-agnostic metric]
