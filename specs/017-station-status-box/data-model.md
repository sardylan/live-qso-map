# Data Model: Station Status Box

**Feature**: `017-station-status-box` | **Date**: 2026-10-24

Nothing is stored. The status is derived from the history each time it is sent (research R2).

## StationStatus (derived, sent as `station_status`)

| Field | Type | Rule |
|-------|------|------|
| `band` | string or null | `BAND` of the most recent QSO; null with no QSO or no band |
| `operator` | string or null | first non-empty of `OPERATOR`, `STATION_CALLSIGN`, configured `[station] callsign`; trimmed, upper-case; null when none (R4) |
| `qso_at` | string (UTC, RFC 3339) or null | date and time of the most recent QSO; null with no QSO or no readable time |
| `age_s` | integer ≥ 0 or null | service clock minus `qso_at`, in whole seconds, 0 if negative; null when `qso_at` is null |

- **Most recent QSO**: the last QSO of the history by sort key (QSO date and time, ties by id).
- **Empty history**: every field null.
- **Not affected by**: the page's band choice and window size (FR-009).

## Page state

| Field | Meaning |
|-------|---------|
| `stationState` | null before the first `station_status`, and again on `hello` |
| `stationState.localQso` | `Date.now() - age_s × 1000`, page clock; absent when `age_s` is null |

Greyed when `localQso` is present and older than 900 s (R5).
