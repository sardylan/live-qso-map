# Data Model: Positions for Prefixed and Portable Callsigns

No stored data changes shape. The QSO record gains one optional field, and the enrichment step
gains three transient values.

## Compound callsign (transient, per QSO)

| Value | Type | Rule |
|-------|------|------|
| logged | text | `CALL` as received. Never modified (HR-5, FR-010) |
| base | text or none | `base_call(logged)` (research R2). None when equal to the logged call |
| retry_allowed | bool | false for `/MM` and `/AM` (FR-007) |
| entity | cty.dat entity or none | `entity_info(logged)`: the prefix entity for prefixed calls (FR-001, FR-011) |

## Position check (transient, per candidate)

| Input | Output |
|-------|--------|
| position (lat, lon, precision, source), entity prefix or none, round (`logged` or `base`) | `Accept`, or `Reject(reason)` with reason `placeholder` or `outside-entity` |

**Rules, in order** (research R4):
1. **Exactly 0°/0°:** `placeholder`.
2. **Inside square `AA00`:** `placeholder`, unless the entity is `CE9`.
3. **Online sources only:** an entity with a known rectangle, and a point outside it or inside a
   nested rectangle, gives `outside-entity`.
4. **Otherwise:** `Accept`.

## Entity areas (loaded once, from the bundled `entity_extents.json`)

| Field | Type | Notes |
|-------|------|-------|
| prefix | text | cty.dat primary prefix, without `*` |
| bounds | `[west, south, east, north]` | `west > east` crosses 180° |
| nested | list of prefixes | entities whose rectangle lies entirely inside `bounds`, precomputed (research R5) |

**Inside(prefix, point):**
- **Unknown prefix:** "unknown", and the point is accepted.
- **Known prefix:** the point is in `bounds`, and in none of the `nested` rectangles.

## QSO record: new field

| ADIF field | When | Value |
|------------|------|-------|
| `APP_LIVEQSOMAP_POS_REJECTED` | at least one position was rejected | `source[@base]:reason` entries, comma-separated, in source order: `logger`, the providers in configured order, `cty` (research R7) |

**Unchanged fields:**
- **`APP_LIVEQSOMAP_POS_SOURCE` and `APP_LIVEQSOMAP_POS_PRECISION`:** describe the position
  finally used.
- **`LAT`, `LON` and `GRIDSQUARE`:** written from the accepted position, as today. Never from a
  rejected one (FR-004).

**Location fields removed from a rejected candidate before merging** (research R6): `GRIDSQUARE`,
`LAT`, `LON`, `QTH`, `STATE`, `CNTY`, `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `IOTA`.

## Enrichment flow (per QSO)

```
logger position ── placeholder? ── yes → note logger:placeholder, treat as absent
      │ no (accepted)                                         │
      ▼                                                       ▼
   use it (no lookups, FR-012)              round 1: all sources with the logged call
                                                              │
                                  no online candidate, retry allowed, base exists?
                                              │ yes                         │ no
                                              ▼                             │
                         round 2: all sources with the base call,           │
                         Cty candidates dropped                              │
                                              └──────────────┬──────────────┘
                                                             ▼
                          check each candidate (R4): rejected → position None,
                          location fields stripped, note source[@base]:reason
                                                             ▼
                       select: most precise accepted position; ties by configured order
                       (the cty reference point is the country-level fallback)
                                                             ▼
                       write position fields, merged fields, rejection note; entity from cty
```
