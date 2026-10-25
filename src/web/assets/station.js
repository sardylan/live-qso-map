// Pure station-status helpers (feature 017, research R5, R6). No DOM.

/** `40m · IK0ABC`, with `—` for a missing value or before the first status. */
export function stationText(status) {
  return `${status?.band ?? '—'} · ${status?.operator ?? '—'}`;
}

// A contest station can go several minutes between QSOs without leaving the band (spec).
export const STATION_STALE_MS = 900_000;

/** `"2026-10-08T15:04:29Z"` → `"Last QSO 15:04Z"` (HR-1); `""` when missing or unreadable. */
export function lastQsoTitle(qsoAt) {
  const match = /^\d{4}-\d\d-\d\dT(\d\d):(\d\d):\d\dZ$/.exec(qsoAt ?? '');
  return match ? `Last QSO ${match[1]}:${match[2]}Z` : '';
}

/** When the QSO happened on this page's clock (ms), from the service's age; `null` without one. */
export function localQsoTime(ageS, now) {
  return ageS == null ? null : now - ageS * 1000;
}

/** The column's clock, `HH:MM:SS UTC` (feature 018, research R4; HR-1). */
export function utcClock(date) {
  const two = (n) => String(n).padStart(2, '0');
  return `${two(date.getUTCHours())}:${two(date.getUTCMinutes())}:${two(date.getUTCSeconds())} UTC`;
}
