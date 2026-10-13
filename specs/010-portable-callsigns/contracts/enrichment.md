# Contract: Position Enrichment of Prefixed and Portable Callsigns (feature 010)

It extends the enrichment behaviour of features 001 (FR-011 to FR-019) and 005 (entity from
cty.dat).

## Inputs and outputs

- **In:** one QSO record (`CALL`, optional `LAT`/`LON`/`GRIDSQUARE` from the logger), the lookup
  sources (providers, then cty), the country data, and the bundled entity rectangles.
- **Out:** the enriched record. Its position fields are as before, plus
  `APP_LIVEQSOMAP_POS_REJECTED` when needed. The QSO's `remote` position and `entity` are as
  before.

## Behaviour table

All positions are illustrative. "Home DE" means a locator in Germany; "SA" is inside Sardinia's
rectangle; "IT" is in mainland Italy.

| # | `CALL` | Logger | Online answer (logged call) | Online answer (base call) | Result position | Source / precision | `APP_LIVEQSOMAP_POS_REJECTED` |
|---|--------|--------|-----------------------------|---------------------------|-----------------|--------------------|-------------------------------|
| 1 | `IS0/DL0AAA` | — | locator SA | (not asked) | that locator | provider / its precision | — |
| 2 | `IS0/DL0AAA` | — | home DE | (not asked) | Sardinia reference | cty / country | `qrz:outside-entity` |
| 3 | `IS0/DL0AAA` | — | unknown | home DE | Sardinia reference | cty / country | `qrz@base:outside-entity` |
| 4 | `IS0/DL0AAA` | — | unknown | locator SA | that locator | provider / its precision | — |
| 5 | `DL0AAA/IS0` | — | home DE | — | Sardinia reference | cty / country | `qrz:outside-entity` |
| 6 | `I/HB9AAA` | — | locator in Cagliari | — | Italy reference | cty / country | `qrz:outside-entity` |
| 7 | `IS0AAA/P` | — | own record SA | (not asked) | that locator | provider | — |
| 8 | `IS0AAA/P` | — | unknown | `IS0AAA` record SA | that locator | provider | — |
| 9 | `K0AAA/6` | — | unknown | `K0AAA` record in the US | that locator | provider | — |
| 10 | `DL0AAA/MM` | — | unknown | (never asked) | none (no entity) | — | — |
| 11 | `DL0AAA` | — | locator in France | (no base) | Germany reference | cty / country | `qrz:outside-entity` |
| 12 | `DL0AAA` | — | 0°/0° | — | Germany reference | cty / country | `qrz:placeholder` |
| 13 | `DL0AAA` | — | `AA00AA` | — | Germany reference | cty / country | `qrz:placeholder` |
| 14 | `LU1ZAA` (Antarctica) | — | `AA00AA` | — | that locator | provider | — |
| 15 | `DL0AAA` | LAT/LON 0/0 | locator DE | — | that locator | provider | `logger:placeholder` |
| 16 | `DL0AAA` | GRIDSQUARE `AA00` | unknown | — | Germany reference | cty / country | `logger:placeholder` |
| 17 | `DL0AAA` | GRIDSQUARE in France | (not asked) | — | the logger locator | logger | — (logger trusted, FR-012) |
| 18 | `XX0AAA` (entity without rectangle) | — | any non-placeholder | — | that position | provider | — |

**With several providers,** each candidate is checked separately. Of the accepted ones, the most
precise wins, with ties broken by the configured order (unchanged). Every rejection is listed in
the note.

## Fields

- **Removed from a rejected candidate before merging:** `GRIDSQUARE`, `LAT`, `LON`, `QTH`,
  `STATE`, `CNTY`, `COUNTRY`, `DXCC`, `CQZ`, `ITUZ`, `CONT`, `IOTA`.
- **Kept and merged as today, without overwriting:** every other field (`NAME`, …).
- **`APP_LIVEQSOMAP_POS_REJECTED`:**
  - grammar: `entry ("," entry)*`, where `entry = source ["@base"] ":" reason`;
  - `source` is one of `logger`, `qrz`, `hamqth`, `qrzcq`, `cty`;
  - `reason` is one of `outside-entity`, `placeholder`;
  - the field is absent when nothing was rejected.

## Logging

One warning per rejection: `position rejected` with fields `call`, `source`, `reason` and
`round` (`logged` or `base`).

## Not changed

- **The callsign** stays as logged (HR-5).
- **The entity** still comes from cty.dat for the logged call (feature 005).
- **The HTTP and WebSocket payloads** are unchanged in shape: the new field travels inside the
  record like any other ADIF field (`docs/http-api.md`).
