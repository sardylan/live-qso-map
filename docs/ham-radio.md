# Ham Radio Domain Context

Amateur radio concepts and rules that every feature of live-qso-map must take into account.

- **[Rules](#1-rules)** (§1) are **binding**: constitution *Technical Constraints* requires them,
  every plan's Constitution Check verifies them, and code review enforces them.
- **[Concepts](#2-concepts)** (§2) are reference material. They explain the domain, so specs and
  code can use the terms without redefining them.

The constitution takes precedence. This document adds detail and can be changed without amending
the constitution: edit this file, then note the change in the [changelog](#changelog) at the
bottom. Changing a rule in §1 also requires re-checking the active features against it.

## 1. Rules

### HR-1 Time is UTC

Every QSO date and time MUST be UTC, both inside the service and in everything it shows or
exports. ADIF `QSO_DATE` is `YYYYMMDD` and `TIME_ON` is `HHMM` or `HHMMSS`, both UTC. Times from a
logger are assumed to be UTC unless its format says otherwise. Local time MUST NOT be used to order,
identify or display QSOs.

### HR-2 Bands are ordered by frequency

Wherever bands are listed, shown, configured, documented or sorted, they MUST appear in frequency
order: lowest frequency (longest wavelength) first. Examples: 160m, 80m, 40m, 20m, 15m, 10m, 6m,
2m, 70cm. Alphabetical order (10m, 12m, 15m, 160m…) MUST NOT be used. This applies to legends,
selectors, configuration examples, tables and API payloads. A JSON object cannot guarantee key
order, so payloads that list bands MUST use an ordered array.

### HR-3 Band names follow the ADIF band plan

A band MUST be named as in the ADIF band enumeration (§2.4), for example `40m` or `70cm`. When a
non-ADIF source (e.g. QARTest) is converted, the band MUST be derived from its frequency, and a
band value the source gives separately is used only when there is no usable frequency. An ADIF
record that already carries `BAND` keeps it (HR-11). One that has `FREQ` but no `BAND` gets the
band derived from `FREQ`. A frequency outside every band leaves the band unknown; it MUST NOT be
rounded to the nearest band.

### HR-4 Modes follow the ADIF mode/submode enumeration

`MODE` and `SUBMODE` MUST use the ADIF enumerations. Sideband is `MODE=SSB` with `SUBMODE=LSB` or
`USB`. FT4 is `MODE=MFSK`, `SUBMODE=FT4`. Logger-specific names MUST be mapped. A mode that can't be
mapped is kept, uppercased, rather than dropped.

### HR-5 Callsigns are compared ignoring case and surrounding whitespace

Callsigns MUST be trimmed and compared case-insensitively; the canonical form is uppercase. Portable
designators are part of the callsign as logged (`IS0/DL1ABC`, `DL1ABC/P`) and MUST be kept as they
are in records. They are interpreted only when looking up the entity (§2.2).

### HR-6 "Country" means an amateur radio entity

A country MUST be a DXCC entity, or a WAE entity where the source distinguishes them, never a
political state. Italy, Sardinia (`IS0`/`IM0`) and Sicily (`IT9`) are three different countries;
so are the USA, Alaska and Hawaii. Country positions are the entity's reference point, not the
centroid of a political border.

### HR-7 Positions keep their precision and their source

Every position MUST state its precision and where it came from. Precision, from least to most
precise:

1. country (entity reference point);
2. 4-character locator;
3. 6-character locator or better;
4. exact coordinates.

A locator stands for the centre of its square. A less precise position MUST NOT be presented as
more precise:

- an entity's reference point MUST NOT be turned into a locator (`GRIDSQUARE`);
- whenever a derived point is written to ADIF `LAT`/`LON` (the map needs a point for every
  resolved QSO), `APP_LIVEQSOMAP_POS_PRECISION` and `APP_LIVEQSOMAP_POS_SOURCE` MUST be written
  with it;
- anything that displays a position MUST show its precision.

Some positions are not positions at all, and MUST be discarded like an absent one (feature 010):

- **Placeholders:** exactly 0° latitude and 0° longitude, from any source, and any point in
  locator square `AA00`, except for an Antarctic station (that square holds the South Pole).
- **Outside the entity:** a position found online that lies outside the area of the callsign's
  entity. A point inside an entity nested within that area (Sardinia within Italy) counts as
  outside. Positions sent by the logger are trusted, except placeholders.

A discarded position is noted in the QSO record (`APP_LIVEQSOMAP_POS_REJECTED`) and logged.

### HR-8 Coordinates use one sign convention inside the service

Inside the service, latitude is positive north and longitude positive east, in decimal degrees.
Every source MUST be converted at the boundary:

- ADIF `LAT`/`LON` use `XDDD MM.MMM` (for example `N039 18.750`, `W122 04.800`);
- AD1C cty.dat stores longitude **positive west**, so its sign MUST be inverted.

### HR-9 A contest dupe is still a QSO

A *dupe* (the same station worked again where the contest gives no points for it) is a real,
logged QSO and MUST be kept and shown like any other. It MUST NOT be confused with a *resend* (the
same QSO transmitted again by the logger) or a *correction* (the same QSO edited in the logger).
Only resends and corrections are merged. Judging dupes, points and multipliers is contest scoring,
which is out of scope unless a feature explicitly adds it.

### HR-10 Contest-agnostic by default

Features MUST work for any contest. Rules specific to one contest (exchange format, scoring,
multipliers, allowed bands) are added only when a specification explicitly asks for them. They are
then kept separate from the generic pipeline.

### HR-11 ADIF is the interchange format

Every QSO MUST be representable as an ADIF record. Field names are uppercase. Values supplied by
the logger MUST NOT be overwritten by derived or looked-up data, which only fills absent fields.
Fields specific to this application use the `APP_LIVEQSOMAP_` prefix.

The service lists the names of the fields it added to a record in `APP_LIVEQSOMAP_ADDED` (feature
014). Every field not listed is the logger's, or comes from a log the operator supplied. When every
online lookup failed without an answer (network error, timeout, quota, login), the record carries
`APP_LIVEQSOMAP_LOOKUP_FAILED` (`source:kind`), so the lookup can be tried again later.

## 2. Concepts

### 2.1 QSO

A two-way contact between two stations. A logged QSO records at least:

- the worked station's callsign (`CALL`);
- date and start time in UTC (`QSO_DATE`, `TIME_ON`);
- band and/or frequency (`BAND`, `FREQ` in MHz);
- mode (`MODE`, `SUBMODE`).

In contests it also records the exchange: signal report (`RST_SENT`, `RST_RCVD`) plus a serial
number, zone, locator or other contest-defined value (`SRX_STRING`/`STX_STRING`, `SRX`/`STX`).
`STATION_CALLSIGN` is the callsign used on the air by the logging station; `OPERATOR` is the
person operating, which in a multi-operator station can change during the contest. Loggers do not
broadcast an operator change on its own (QARTest's OPON command only stamps the QSOs logged after
it), so the current operator is the `OPERATOR` of the latest logged QSO.

### 2.2 Callsigns

A callsign is a **prefix** (one or more letters and/or digits that identify the entity, for
example `DL`, `IS`, `K`, `3D2`), a **call-area digit**, and a **suffix** (`DL5QFN`, `IS0MZK`,
`K6QA`).

**Portable and compound forms:**

| Form | Meaning | Entity taken from | Looked up online as |
|------|---------|-------------------|---------------------|
| `IS0/DL1ABC` | DL1ABC operating from Sardinia | the shorter, prefix-like part, `IS0` | `IS0/DL1ABC`, then `DL1ABC` if unknown |
| `DL1ABC/P`, `/M`, `/A`, `/QRP` | portable, mobile, alternative address, low power | `DL1ABC` (suffix ignored) | `DL1ABC/P`, then `DL1ABC` if unknown |
| `K1ABC/6` | operating in US call area 6 | `K6ABC` (still USA) | `K1ABC/6`, then `K1ABC` if unknown |
| `DL1ABC/MM`, `/AM` | maritime or aeronautical mobile | no entity | `DL1ABC/MM` only |

A position found through the base callsign is still judged against the entity of the callsign
as logged (HR-7): `IS0/DL1ABC` is never drawn at DL1ABC's home in Germany.

Some callsigns are exceptions to their prefix. The country data lists them as exact matches
(`=CALL` in cty.dat), which take precedence over any prefix rule.

### 2.3 Entities, zones and continents

| Concept | Description | ADIF field |
|---------|-------------|------------|
| **DXCC entity** | A "country" on the ARRL DXCC list (about 340 current entities), identified by a number. For example 230 = Federal Republic of Germany, 291 = United States, 225 = Sardinia, 248 = Italy. Many are not sovereign states. | `DXCC`, `COUNTRY` |
| **WAE entity** | Additional European entities from the DARC WAE list, e.g. Sicily (`*IT9` in cty.dat, where `*` marks WAE-only). | — |
| **CQ zone** | 40 zones used by CQ-sponsored contests and awards (e.g. CQ WW). | `CQZ` |
| **ITU zone** | 90 zones used by ITU-sponsored contests (e.g. IARU HF). | `ITUZ` |
| **Continent** | `AF`, `AN`, `AS`, `EU`, `NA`, `OC`, `SA`. | `CONT` |

**AD1C country files (cty.dat)** are the de-facto standard used by contest loggers:

- each entity has a header line with name, CQ zone, ITU zone, continent, latitude, longitude
  (west positive), UTC offset and primary prefix;
- the header is followed by its prefixes and exact callsigns;
- per-prefix overrides change a value for that prefix only: `(cq)`, `[itu]`, `<lat/lon>`,
  `{continent}`, `~utc~`;
- longest-prefix match decides the entity.

Source: <https://www.country-files.com/>.

### 2.4 Bands

Amateur bands are named by their approximate wavelength. ADIF band plan (ADIF 3.1), in frequency
order (HR-2); edges in MHz, inclusive:

| Band | From (MHz) | To (MHz) | Range |
|------|-----------:|---------:|-------|
| 2190m | 0.1357 | 0.1378 | LF |
| 630m | 0.472 | 0.479 | MF |
| 560m | 0.501 | 0.504 | MF |
| 160m | 1.8 | 2.0 | MF |
| 80m | 3.5 | 4.0 | HF |
| 60m | 5.06 | 5.45 | HF |
| 40m | 7.0 | 7.3 | HF |
| 30m | 10.1 | 10.15 | HF (WARC) |
| 20m | 14.0 | 14.35 | HF |
| 17m | 18.068 | 18.168 | HF (WARC) |
| 15m | 21.0 | 21.45 | HF |
| 12m | 24.89 | 24.99 | HF (WARC) |
| 10m | 28.0 | 29.7 | HF |
| 8m | 40.0 | 45.0 | VHF |
| 6m | 50.0 | 54.0 | VHF |
| 5m | 54.000001 | 69.9 | VHF |
| 4m | 70.0 | 71.0 | VHF |
| 2m | 144.0 | 148.0 | VHF |
| 1.25m | 222.0 | 225.0 | VHF |
| 70cm | 420.0 | 450.0 | UHF |
| 33cm | 902.0 | 928.0 | UHF |
| 23cm | 1240.0 | 1300.0 | UHF |
| 13cm | 2300.0 | 2450.0 | SHF |
| 9cm | 3300.0 | 3500.0 | SHF |
| 6cm | 5650.0 | 5925.0 | SHF |
| 3cm | 10000.0 | 10500.0 | SHF |
| 1.25cm | 24000.0 | 24250.0 | SHF |
| 6mm | 47000.0 | 47200.0 | EHF |
| 4mm | 75500.0 | 81000.0 | EHF |
| 2.5mm | 119980.0 | 123000.0 | EHF |
| 2mm | 134000.0 | 149000.0 | EHF |
| 1mm | 241000.0 | 250000.0 | EHF |
| submm | 300000.0 | 7500000.0 | — |

Notes:

- National allocations are narrower than the ADIF edges and differ by ITU region.
- HF contests normally use only 160m, 80m, 40m, 20m, 15m and 10m. The WARC bands (30m, 17m, 12m)
  carry no contests by convention.
- Loggers sometimes send the band in metres without a unit (`40`) or as a frequency in other units.
  QARTest's `<txfreq>`, for example, is in units of 10 Hz.

### 2.5 Modes

| Logged as | ADIF `MODE` | ADIF `SUBMODE` |
|-----------|-------------|----------------|
| CW | `CW` | — |
| LSB / USB (phone) | `SSB` | `LSB` / `USB` |
| FM, AM | `FM`, `AM` | — |
| RTTY | `RTTY` | — |
| FT8 | `FT8` | — |
| FT4 | `MFSK` | `FT4` |
| PSK31 | `PSK` | `PSK31` |

Contest rules often group modes as *CW*, *PH* (phone: SSB, FM, AM) and *DIGI* (RTTY, FT8, PSK…).
Some contests allow working the same station once per mode on each band.

### 2.6 Maidenhead locator

A worldwide grid of nested rectangles. Each pair of characters gives longitude first, then latitude:

| Level | Characters | Values | Size (lon × lat) |
|-------|-----------|--------|------------------|
| Field | 1–2 | `A`–`R` | 20° × 10° |
| Square | 3–4 | `0`–`9` | 2° × 1° |
| Subsquare | 5–6 | `a`–`x` | 5′ × 2.5′ |
| Extended square | 7–8 | `0`–`9` | 30″ × 15″ |

Examples: `JM49NH` is in southern Sardinia; `FN42` is New England. Locators are case-insensitive;
by convention the field is written in uppercase and the subsquare in lowercase (`JM49nh`). VHF and
UHF contests often use the 6-character locator as the exchange.

### 2.7 Contests

| Term | Meaning |
|------|---------|
| **Exchange** | Information both stations must copy: a report plus a serial number, zone, locator, state, age… depending on the contest |
| **Multiplier** | A new entity, zone, prefix, locator square… that multiplies the score |
| **Dupe** | The same station worked again on the same band (and, where it matters, the same mode); usually 0 points (HR-9) |
| **Categories** | Single-operator or multi-operator, power level, assisted, one or more transmitters |
| **Contest identifier** | ADIF `CONTEST_ID`, e.g. `CQ-WW-SSB`, `CQ-WPX-CW`, `IARU-HF` |
| **WPX prefix** | The prefix as defined by the CQ WPX rules, ADIF `PFX` (e.g. `DL5` for DL5QFN) |

### 2.8 Logging software and network messages

Contest loggers (QARTest, N1MM Logger+, Win-Test, DXLog…) can broadcast every logged, edited or
deleted QSO over UDP, as XML or ADIF. These messages are best-effort: datagrams can be lost,
duplicated or arrive out of order, and an edit may be sent as a new full copy of the QSO. Loggers
often attach their own QSO identifier (QARTest `<id>`), which is the most reliable way to recognise
the same QSO across edits.

### 2.9 Callbooks

Online databases that map a callsign to name, address, locator and coordinates:

| Callbook | Access |
|----------|--------|
| QRZ.com | full data needs a subscription; QRZ's `geoloc` field says how its coordinates were obtained |
| HamQTH | free account |
| QRZCQ | free and premium tiers |

Data is entered by the station owner and can be missing, coarse (country centre) or out of date,
which is why positions carry precision and source (HR-7).

### 2.10 Great-circle path

Radio signals travel along the shortest path on the sphere, which is a great circle. On a flat map
it appears as a curve. Paths that cross the 180° meridian must be drawn continuously, not wrapped
across the map.

### 2.11 Grayline

The grayline is the twilight band along the day/night terminator, where the Sun is between 0°
and 6° below the horizon (civil twilight). Low-band paths (160m, 80m, 40m) often open along it,
between stations that are both near their sunrise or sunset. The map shades the night side with a
fade that starts at the horizon and reaches full night at −12°, so the grayline is the lighter
part of the fade. Sun heights come from the UTC time (HR-1).

### 2.12 History file

The service's QSOs between runs: one ADIF file (HR-11), read at startup and written back at a
graceful stop. It is the service's own store, not the operator's log, although an operator may
place a log there to have it completed. Deleting it while the service is stopped empties the map.

## Changelog

| Date | Change |
|------|--------|
| 2026-10-09 | Initial version: rules HR-1 to HR-11, concepts §2.1–§2.10. |
| 2026-10-13 | HR-7: placeholder positions and positions outside the entity are discarded; §2.2: how compound callsigns are looked up online (feature 010). |
| 2026-10-14 | §2.11 Grayline (feature 013). |
| 2026-10-14 | HR-11: `APP_LIVEQSOMAP_ADDED` and `APP_LIVEQSOMAP_LOOKUP_FAILED`; §2.12 History file (feature 014). |
