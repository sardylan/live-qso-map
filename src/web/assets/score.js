// Pure score-line helpers (feature 007, research R5). No DOM.

// QARTest sends a score every minute: five missed ones mean the station has gone quiet (FR-007).
export const STALE_MS = 300_000;

/** `1234567` → `"1.234.567"`, the same in every browser locale; anything else → `"?"`. */
export function groupDigits(n) {
  if (!Number.isSafeInteger(n) || n < 0) return '?';
  return String(n).replace(/\B(?=(\d{3})+$)/g, '.');
}

/** The one-line score, or `Score —` before the first one (FR-005). */
export function scoreText(score) {
  if (!score) return 'Score —';
  return `${groupDigits(score.score)} pts · ${groupDigits(score.qsos)} QSO · ${groupDigits(score.mults)} mults`;
}

/** Whether a score received at `localReceived` (page clock, ms) is too old at `now`. */
export function isStale(localReceived, now, limitMs = STALE_MS) {
  return now - localReceived > limitMs;
}

/** `"2026-10-08T15:04:29Z"` → `"Last updated 15:04Z"` (HR-1); `""` when unreadable. */
export function updatedTitle(receivedAt) {
  const match = /^\d{4}-\d\d-\d\dT(\d\d):(\d\d):\d\dZ$/.exec(receivedAt ?? '');
  return match ? `Last updated ${match[1]}:${match[2]}Z` : '';
}
