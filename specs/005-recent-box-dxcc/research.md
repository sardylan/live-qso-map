# Research: DXCC Entity and Date in the Last-QSO Box

**Feature**: `005-recent-box-dxcc` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

There are no NEEDS CLARIFICATION items: the owner settled four decisions before the spec was
written, and confirmed the defaults.

## R1. Where the entity is resolved (FR-001–FR-004)

- **Decision**: resolve it **at enrichment** and store it on the enriched QSO.
  - `enrich()` resolves the remote callsign with `CountryData::entity_of` (feature 004).
  - The result goes in a new field, `EnrichedQso.entity: Option<QsoEntity>`.
  - `Lookups` carries the loaded country data next to the sources, as
    `country: Option<Arc<CountryData>>`.
- **Rationale**:
  - enrichment already derives everything else about a QSO;
  - a correction is enriched again, so FR-004 holds without extra code;
  - snapshots and upserts read the stored value, so nothing is resolved per page.
- **Alternatives**:
  - resolve when building `QsoView` for each page: the country data would need to be threaded
    through every session, and the same callsign resolved again for every page and snapshot;
  - reuse the `COUNTRY` field that the cty.dat lookup writes into the record: it is present only
    when cty.dat supplied the position, so the logger-positioned QSOs would have no entity
    (against FR-002).

## R2. Tests with country data (Principle I)

- **Decision**: add `pipeline::start_with_country(config, sources, country)` for tests.
  - `start(config, sources)` is unchanged: it builds the country data itself in production, and
    uses no country data when the sources are injected.
  - Unit tests build `Lookups` with the `tests/fixtures/cty-excerpt.dat` fixture directly.
- **Rationale**: the existing test call sites stay untouched.

## R3. Wire format and protocol version (FR-005, API-4)

- **Decision**: `QsoView` gains `entity: {"prefix": "JA", "name": "Japan"}`, or `null` when
  unknown. **The protocol stays at 3.**
- **Rationale**:
  - the change only adds a field, which API-4 does not count as breaking;
  - pages ignore fields they don't know;
  - feature 003 set the precedent by keeping protocol 2 when it added `remove`.
- **Alternative**: bump to 4. It would cost a version for no client-visible break.

## R4. Box row layout and truncation (FR-006–FR-008, SC-003)

- **Decision**: in `recent.js`, `rowView(qso)` returns
  `{call, band, date, time, entity, entityTitle, markerKey}`:
  - `date` is `YYYY-MM-DD` and `time` is `HH:MMZ`, both from the UTC `datetime`, empty when it is
    missing;
  - `entity` is `prefix · name`, with the name cut to 18 characters by a pure function,
    `shortName(name, 18)`. The cut keeps the first 17 characters, trims trailing spaces, and adds
    "…";
  - `entityTitle` is the full `prefix · name`, used as the cell's `title`;
  - an unknown entity gives `—` and an empty title;
  - `mode` is no longer part of the row.
- **Rationale**:
  - cutting in a pure function makes the 18-character rule testable;
  - a CSS ellipsis would depend on the font and couldn't be tested with `node --test`.
- **Width check (SC-003)**:
  - the longest cty.dat names have 24 characters, for example "Central African Republic";
  - after the cut, a row is at most about 10 + 5 + 10 + 6 + 22 characters, which is well under
    45 % of a 1024 px window at 13 px;
  - the check is done manually (quickstart).

## R5. Popup entity (FR-010)

- **Decision**: a pure helper, `entityLabel(entity)` in `recent.js`, gives `prefix · name` or `—`,
  the same text as the box but **not cut**, because popups have room.
- **Glue**: `popupContent` in `app.js` adds one cell per QSO with that label.
- **Unchanged**: the existing popup update path, `diffLayers` signature → `setDOMContent`.
  - The signature includes `[id, rev]`, so a correction triggers a popup refresh.
  - Changing the callsign raises `rev`, so the entity refreshes too (US2 scenario 2).
