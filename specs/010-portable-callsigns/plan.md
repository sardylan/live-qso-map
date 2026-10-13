# Implementation Plan: Positions for Prefixed and Portable Callsigns

**Branch**: `feature/010-portable-callsigns` | **Date**: 2026-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/010-portable-callsigns/spec.md`

## Summary

Remote positions are judged against the callsign's entity before they are used. All changes stay
inside the enrichment step.

- **Base-callsign retry (R2, R3):** for `IS0/DL0AAA`, `IS0AAA/P`, `K0AAA/6` and similar, a new
  `callsign` module finds the base callsign. When no online service knows the callsign as
  logged, the lookups run once more with the base callsign. The country data's answer for the
  base callsign is dropped, because the entity stays the logged callsign's. `/MM` and `/AM` are
  never retried.
- **Acceptance (R4):** a new `acceptance` module decides every candidate position:
  - **0°/0°:** always a placeholder.
  - **Square `AA00`:** a placeholder except for Antarctica (`CE9`).
  - **Online positions:** must lie inside the entity's rectangle and outside any rectangle
    nested in it (R5). Cagliari is not inside Italy.

  Logger positions are subject to the placeholder rule only.
- **Rejected candidates (R6, R7, R8):** each loses its position and its location fields, and
  keeps its name and other personal fields. It is listed in the new ADIF field
  `APP_LIVEQSOMAP_POS_REJECTED` (`source[@base]:reason`) and logged as a warning. The cty
  reference point remains the country-level fallback.
- **Documentation (R9):** `docs/ham-radio.md` gets the rule (HR-7 and §2.2).
- **Known limitation (R5):** Sicily's rectangle isn't fully inside Italy's. Entity outlines are
  a follow-up, together with sub-area data.

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md`
§2)

**Primary Dependencies**: none new. Existing: tokio, serde_json (to read the bundled rectangles),
tracing.

**Storage**: none new. One optional ADIF field in the in-memory record; the existing bundled
`src/web/entity_extents.json` is reused.

**Testing**:
- unit tests in `src/enrich/callsign.rs`, `src/enrich/acceptance.rs` and `src/entity_extents.rs`
  (nesting, 180° wrap);
- a new integration test, `tests/portable_callsigns.rs`, covering the behaviour table, with
  injected fake providers and a cty fixture.

**Target Platform**: development on any host; release targets as configured in
`.circleci/config.yml` (Principle V).

**Project Type**: single binary

**Performance Goals**:
- no extra lookups for callsigns that an online service knows;
- at most one extra round, bounded by the existing timeout, for unknown compound callsigns;
- the acceptance check is O(nested), at most a few rectangles per entity.

**Constraints**:
- callsigns are never altered in records (HR-5);
- the entity still comes from cty.dat (feature 005);
- the logger's position is trusted except for placeholders (FR-012);
- no page or API change (`docs/http-api.md`): the new field travels in the record's ADIF map.

**Scale/Scope**: 346 cty.dat entities and 333 rectangles, of which 35 contain nested ones (83
pairs); up to three providers per QSO.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests come before the code.
  - **`callsign`:** every form in research R2 (prefix before and after, `/P`, `/6`, `/MM`,
    several slashes, empty parts, lowercase, plain calls).
  - **`acceptance`:**
    - exact 0°/0° and near-zero values;
    - `AA00` and `AA00AA` given as a locator and as coordinates, with Antarctica as the exception;
    - nested and overlapping rectangles (Sardinia in Italy; Germany and Poland);
    - a rectangle crossing 180° (`3D2`);
    - an unknown entity.
  - **Integration:** one test per row of `contracts/enrichment.md`, written first, failing.
  - **Regression:** existing lookup tests stay green (SC-006).
- [x] **II. Rust Safety & Quality**:
  - no `unsafe`, no `unwrap`/`expect` in runtime code;
  - new code returns nothing fallible: pure functions over `Option`;
  - new integration tests return `Result` and use `?`, with no inline lint suppressions (LL-001);
  - fmt and clippy are gates.
- [x] **III. Simplicity & Minimal Dependencies**:
  - no new crate;
  - two small modules;
  - the existing source type, ranking and rectangle data are reused;
  - the suffix lists move to one shared place instead of being duplicated.
- [x] **Domain rules** (`docs/ham-radio.md` §1):
  - **HR-5:** the callsign stays as logged.
  - **HR-6:** the entity comes from cty.dat, and nested entities (Sardinia, Sicily) stay separate.
  - **HR-7:** precision and source are written for the final position, rejections are noted, and
    a country-level position is never turned into a locator.
  - **HR-8:** sign convention unchanged.
  - **HR-11:** the new field uses the `APP_LIVEQSOMAP_` prefix.
  - The domain document is updated (R9).
- [x] **IV. Persistent Viewer Settings**: not affected.
- [x] **V. Portable Development, Pipeline-Built Releases**: pure Rust, no platform-specific code,
  tested on every host. No release change.
- [x] **Interface rules** (`docs/http-api.md`): no new endpoint or message. The record's ADIF map
  already carries every field as JSON.

**Post-design re-check**: unchanged after Phase 1. No violations.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| none | — | — |

## Project Structure

### Documentation (this feature)

```text
specs/010-portable-callsigns/
├── spec.md
├── plan.md
├── research.md          # R1–R9
├── data-model.md        # compound callsign, position check, entity areas, new field, flow
├── quickstart.md
├── contracts/
│   └── enrichment.md    # behaviour table (18 rows), field grammar, logging
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/enrich/callsign.rs        # new: base_call, retry_allowed, shared suffix lists (R2)
src/enrich/acceptance.rs      # new: Verdict, RejectReason, check(position, entity_prefix, online, areas) → Verdict, rejection_note (R4, R7)
src/enrich/cty.rs             # uses the shared suffix lists from callsign.rs
src/enrich/resolve.rs         # resolve_remote(…, accept) → Lookup { candidates, rejections, known_online }; select(candidates, order) holds today's ranking
src/enrich/mod.rs             # enrich(): logger placeholder, base-call round, notes, location-field stripping
src/entity_extents.rs         # contains() promoted from the tests; nested lists precomputed (R5)
src/pipeline.rs               # passes the extent table into Lookups
tests/portable_callsigns.rs   # new: behaviour table rows 1–18
tests/fixtures/cty-portable.dat   # new: Germany, France, Italy, Sardinia, Sicily, USA, Antarctica (verbatim from cty.dat)
docs/ham-radio.md             # HR-7 sentence, §2.2 column, changelog line (R9)
CHANGELOG.md                  # "Unreleased" entries (feature 009 process)
```

**Structure Decision**: existing layout. Two new modules sit next to the code they serve, in
`src/enrich/`. No new top-level directories.

## Complexity Tracking

No violations.
