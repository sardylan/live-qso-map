# Research: Positions for Prefixed and Portable Callsigns

Feature: `010-portable-callsigns` · Spec: [spec.md](spec.md) · Date: 2026-10-13

**Facts checked on 2026-10-13:**
- **cty.dat** (`./cty.dat`, AD1C, 346 entities):
  - no per-prefix position overrides (`<lat/lon>`) at all;
  - no entity has its reference point at 0°/0°;
  - Antarctica's primary prefix is `CE9`, with its reference point at the South Pole (−90°, 0°).
- **Bundled rectangles** (`src/web/entity_extents.json`, feature 004):
  - 333 entities have one;
  - four cross the 180th meridian (`west > east`): `3D2`, `CE9`, `KL`, `UA9`.
- **Online providers** (QRZ, HamQTH, QRZCQ) answer an unknown callsign with `Ok(None)`. Their
  results carry these fields: `NAME`, `QTH`, `GRIDSQUARE`, `LAT`, `LON`, `STATE`, `CNTY`,
  `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `IOTA`.
- **The country data is also a lookup source** (`cty_source`): it always answers with the
  entity's reference point, at precision "country".

## R1. Where the change goes

- **Decision**: all changes stay in the enrichment step (`src/enrich/`). Two new pure modules hold
  the rules, and `enrich::enrich` applies them:
  - **`src/enrich/callsign.rs`:** reads the callsign as logged: base callsign, whether a retry
    with it is allowed.
  - **`src/enrich/acceptance.rs`:** decides whether a position is acceptable for an entity:
    inside its area with nesting, or a placeholder. Returns the reason.
- **Rationale**:
  - Lookups already run per QSO in `enrich`, and the entity already comes from cty.dat there
    (feature 005).
  - Pure functions are unit-tested on every host (Principle I), with no network.
  - Providers and `resolve_remote`'s ranking stay as they are.
- **Alternatives**: filtering inside each provider. Rejected: the same rule would be repeated
  three times, and providers don't know the entity.

## R2. Reading a compound callsign

- **Decision**: `base_call(call) -> Option<String>` and `retry_allowed(call) -> bool`, built on the
  splitting rules `cty.rs` already uses (`docs/ham-radio.md` §2.2):
  1. split on `/` and drop empty parts;
  2. drop the ignored suffixes (`P`, `M`, `QRP`, `QRPP`, `A`, `B`, `LH`) and single call-area
     digits;
  3. of the remaining parts, the **base** is the longest. With two parts, the shorter one is the
     location prefix, as cty.rs already decides;
  4. `base_call` returns `None` when the base equals the trimmed, uppercased call: there's nothing
     to retry.

  `retry_allowed` is false for `/MM` and `/AM` (no entity, station not at home).

  | Call | Base | Retry allowed |
  |------|------|---------------|
  | `IS0/DL0AAA` | `DL0AAA` | yes |
  | `DL0AAA/IS0` | `DL0AAA` | yes |
  | `IS0AAA/P` | `IS0AAA` | yes |
  | `K0AAA/6` | `K0AAA` | yes |
  | `IS0/DL0AAA/P` | `DL0AAA` | yes |
  | `DL0AAA` | none | no |
  | `DL0AAA/MM` | none | no |

- **Rationale**: one shared set of suffix constants. The ignored-suffix and maritime lists move
  from `cty.rs` into `callsign.rs`, and cty.rs imports them, so entity rules and retry rules
  can't drift apart.
- **Alternatives**: a separate, richer callsign grammar. Rejected as YAGNI; the existing rules
  already cover every form in the spec.

## R3. When to retry with the base callsign

- **Decision**:
  1. **First round:** every source (providers and cty) gets the call as logged, as today.
  2. **Second round, providers only:** it runs when the first round had **no online candidate**
     (every provider answered `Ok(None)` or failed), `retry_allowed` is true, and `base_call`
     returns a base. All sources run again with the base call, and **candidates whose source is
     `Cty` are dropped**: the cty answer for the base call (`DL0AAA` → Germany) would contradict
     the logged call's entity.
  3. The candidates of both rounds are pooled for acceptance (R4) and selection (the existing
     precision-then-order rule).
- **Rationale**:
  - Re-calling every source and dropping the `Cty` answers avoids changing the `LookupSource`
    type, so test injection and `pipeline.rs` stay as they are.
  - The retry costs at most one extra round, bounded by the same timeout, and only for callsigns
    no service knows (spec assumption "one extra lookup at most").
  - "Unknown online" means no candidate from a non-`Cty` source. A provider error or timeout
    counts as unknown, which is the safe direction: the retry may find data.
- **Alternatives**: tagging sources as online or offline in `Lookups`. Cleaner in theory, but it
  changes the `LookupSource` plumbing for no behavioural gain.

## R4. Accepting a position

- **Decision**: `check(position, entity_prefix, online, areas) -> Verdict`, where `Verdict` is
  `Accept` or `Reject(RejectReason)`, a plain enum, not a `Result` (`docs/rust-guidelines.md`).
  `online` is true for online providers, and false for the logger and cty. It is applied to:
  - every online candidate's position;
  - the logger position, for the placeholder rule only (FR-012);
  - the cty candidate's position, for the placeholder rule only.

  The reasons are `outside-entity` and `placeholder`. The checks, in order:
  1. **Placeholder 0°/0°**: `lat == 0.0 && lon == 0.0` exactly. Locators can't produce it: square
     centres are never 0. → `placeholder`, for every callsign.
  2. **Placeholder `AA00`**: the position lies in locator square `AA00` (lat in [−90, −89), lon in
     [−180, −178)). That covers `AA00` and `AA00AA` however they were sent (locator or
     coordinates). → `placeholder`, unless the entity's prefix is `CE9` (Antarctica).
  3. **Outside the entity** (online candidates only): the entity has a rectangle, and the position
     is outside it or inside a rectangle nested in it (R5). → `outside-entity`.
  4. **Otherwise accept**, including when the entity, its rectangle or the country data is unknown
     (spec edge cases).
- **Rationale**: one function, so the logger, online and cty paths share the placeholder rule.
  Checking against the position, not the locator text, catches placeholders that a provider sends
  as coordinates.
- **Alternatives**: matching the text `AA00AA` only. Rejected: a provider may send `LAT`/`LON`
  `-90/-180` instead.

## R5. Entity areas with nesting

- **Decision**: an `EntityAreas` view over the existing `ExtentTable`. At load, it precomputes
  each entity's list of **nested** entities: the rectangles fully inside its own rectangle,
  wrap-aware.
  - **Inside entity E:** the point is in E's rectangle and in none of E's nested rectangles.
  - **Wrap handling:** the `contains` helper in the extents tests (`west > east` crosses 180°)
    becomes production code.
  - **Size:** on the rectangles bundled on 2026-10-13, 35 entities contain nested ones (83
    pairs). For example:
    - Italy contains `IS`, `T7`, `HV`, `3A`, `S5`, `TK`;
    - France contains `LX`, `GJ`, `GU`, `3A`, `C3`;
    - England contains `GW`, `GD`;
    - Brazil contains `CP`, `ZP`.
- **Rationale**: it implements the clarification "Cagliari is not inside Italy". Neighbours whose
  rectangles only overlap (Germany and Poland, Netherlands and Belgium) are unaffected.
- **Known limitation**: Sicily's rectangle reaches south of Italy's (36.65° vs 37.91°N), so it is
  **not** fully nested. A locator on Sicily's northern coast still counts as inside Italy for an
  `I/…` callsign. An overlap threshold doesn't fix this without breaking real neighbours: Belgium's
  rectangle overlaps about 29% of the Netherlands' in Limburg and Zeeland. The real fix is entity
  outlines instead of rectangles, from the same Natural Earth source and the feature 004
  generator. Recorded as a follow-up, next to the sub-area data.
- **Alternatives**:
  - **Smallest containing rectangle:** rejected, it would put eastern Germany in Poland.
  - **Outlines:** rejected for now. They're the correct fix, but a larger bundled file and a
    new generator, out of this feature's scope.

## R6. Location fields of a rejected result (FR-004)

- **Decision**: when a candidate's position is rejected, its position is set to `None`, and these
  fields are removed from its `fields` before merging: `GRIDSQUARE`, `LAT`, `LON`, `QTH`,
  `STATE`, `CNTY`, `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `IOTA`. `NAME` and every other field
  stay.
- **Rationale**: these fields describe *where* the station was found. For `IS0/DL0AAA` they
  describe its German home, so `COUNTRY` and `DXCC` would contradict the entity shown. Personal
  fields stay true (clarification Q2 of /speckit-clarify).
- **Alternatives**: dropping the whole result (clarification option B, not chosen).

## R7. The rejection note (FR-013)

- **Decision**: the ADIF field `APP_LIVEQSOMAP_POS_REJECTED`, set only when something was
  rejected.
  - **Value:** comma-separated `source:reason` entries, in source order (the logger first, then
    the configured providers, then cty). For example, `qrz:outside-entity,hamqth:outside-entity`
    or `logger:placeholder`.
  - **Source names:** the existing `PositionSource::as_str()` (`logger`, `qrz`, `hamqth`,
    `qrzcq`, `cty`).
  - **Second round:** a rejection from the base-call round gets the suffix `@base`, for example
    `qrz@base:outside-entity`, so the note says which call gave it.
- **Rationale**:
  - The `APP_LIVEQSOMAP_` prefix follows HR-11.
  - The page already shows the record's fields with the QSO, and the HTTP interface carries
    them as JSON, so no page or API change is needed (Interface rules).
  - It sits next to the existing `APP_LIVEQSOMAP_POS_SOURCE` and `_POS_PRECISION`.
- **Alternatives**:
  - **One field per source:** more fields for a rare event.
  - **A free-text sentence:** harder to test and to filter.

## R8. Logging (FR-009)

- **Decision**: one `tracing::warn!` per rejection, with `call`, `source` and `reason` as
  structured fields, never secrets. It sits beside the existing "no position found" warning.
- **Rationale**: the same shape as the existing lookup warnings in `resolve.rs`.

## R9. Documentation of the domain rule

- **Decision**: update `docs/ham-radio.md`, which is binding:
  - the §2.2 table gets a "Position looked up as" column (logged call, then base call);
  - HR-7 gets a sentence: a position outside the callsign's entity, or a placeholder (0°/0°;
    `AA00` except Antarctica), is not a position.
  - a line in the document's changelog.
- **Rationale**: the rule changes how positions are judged. The domain document is where future
  features look for it (constitution: domain rules are binding).
