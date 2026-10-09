// Pure helpers for the last-QSO box (feature 004, research R8).
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

import { visible } from './map-state.js';

/** The `n` most recent displayed QSOs by QSO time, newest first (FR-019). */
export function recentQsos(state, n = 5) {
  const window = visible(state);
  return window.slice(Math.max(0, window.length - n)).reverse();
}

function utcTime(datetime) {
  const match = typeof datetime === 'string' ? /T(\d\d:\d\d)/.exec(datetime) : null;
  return match ? `${match[1]}Z` : '';
}

/** One box row. Text is returned as is: the glue renders it with textContent only (FR-022). */
export function rowView(qso) {
  const mode = qso.mode ?? '';
  return {
    call: qso.call ?? '',
    band: qso.band ?? '',
    mode: qso.submode ? `${mode}/${qso.submode}` : mode,
    time: utcTime(qso.datetime),
    markerKey: qso.remote ? `${qso.remote.lat},${qso.remote.lon}` : null,
  };
}
