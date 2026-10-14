// Pure connection helpers: reconnect backoff, the post-connect handshake and the remembered
// window size (FR-031, data-model "Connection state"). No DOM, no WebSocket.

// The menu tops out at 1000: the service keeps every QSO, a page shows at most 1000 (feature 014).
export const LIMITS = [10, 20, 50, 100, 200, 500, 1000];
const MAX_LIMIT = 1000;
const DEFAULT_LIMIT = 20;
const STORAGE_KEY = 'lqm.limit';

export function createConnection(limit = DEFAULT_LIMIT) {
  return { attempt: 0, limit };
}

/** 1, 2, 4, 8, 16, then 30 s, with ±20% jitter from `rand()` in [0, 1]. */
export function backoffDelay(attempt, rand = Math.random) {
  const base = Math.min(1000 * 2 ** attempt, 30000);
  return Math.round(base * (0.8 + 0.4 * rand()));
}

export function onOpen(connection) {
  connection.attempt = 0;
}

/** Returns how long to wait before the next attempt. */
export function onClose(connection, rand = Math.random) {
  const delay = backoffDelay(connection.attempt, rand);
  connection.attempt += 1;
  return delay;
}

/**
 * Messages to send after every (re)connect, once the server said hello. `bands` (feature 011,
 * protocol 4) is sent once the page has a band choice: a list, or `null` for every band.
 */
export function onHello(connection) {
  const message = { type: 'set_limit', limit: connection.limit };
  if (connection.bands !== undefined) message.bands = connection.bands;
  return [message];
}

export function loadLimit(storage) {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (raw === 'all') {
      // "All" is gone (feature 014): it becomes 1000, saved back so it is read as such next time.
      saveLimit(storage, MAX_LIMIT);
      return MAX_LIMIT;
    }
    const value = Number.parseInt(raw, 10);
    return LIMITS.includes(value) ? value : DEFAULT_LIMIT;
  } catch {
    return DEFAULT_LIMIT;
  }
}

export function saveLimit(storage, limit) {
  try {
    storage?.setItem(STORAGE_KEY, String(limit));
  } catch {
    // Storage may be unavailable (private mode); the selection then lasts for this page only.
  }
}
