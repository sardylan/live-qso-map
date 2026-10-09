// Pure helpers for the last-QSO box (feature 004, research R8).
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

import { visible } from './map-state.js';

/** The `n` most recent displayed QSOs by QSO time, newest first (FR-019). */
export function recentQsos(state, n = 5) {
  const window = visible(state);
  return window.slice(Math.max(0, window.length - n)).reverse();
}

const ENTITY_NAME_MAX = 18;

/** `name` cut to `max` characters, the last being "…", when longer (feature 005, FR-007). */
export function shortName(name, max = ENTITY_NAME_MAX) {
  if (name.length <= max) return name;
  return `${name.slice(0, max - 1).trimEnd()}…`;
}

/** "JA · Japan" for a cty.dat entity, "—" when unknown (FR-008, FR-010). */
export function entityLabel(entity) {
  return entity ? `${entity.prefix} · ${entity.name}` : '—';
}

/** `[date, time]` in UTC from an RFC 3339 datetime: "2026-10-09", "15:04Z" (HR-1). */
function utcDateTime(datetime) {
  const match = typeof datetime === 'string' ? /^(\d{4}-\d\d-\d\d)T(\d\d:\d\d)/.exec(datetime) : null;
  return match ? [match[1], `${match[2]}Z`] : ['', ''];
}

const LABELS = {
  call: 'Call',
  band: 'Band',
  mode: 'Mode',
  date: 'Date',
  time: 'UTC',
  cqzone: 'CQ',
  country: 'Country',
};

/** The header row of the box: one short label per configured column (feature 006, FR-006a). */
export function headerCells(columns) {
  return columns.map((column) => ({ column, text: LABELS[column] ?? column }));
}

function cell(column, qso) {
  const [date, time] = utcDateTime(qso.datetime);
  const entity = qso.entity ?? null;
  switch (column) {
    case 'call':
      return { text: qso.call ?? '' };
    case 'band':
      return { text: qso.band ?? '' };
    case 'mode': {
      const mode = qso.mode ?? '';
      return { text: qso.submode ? `${mode}/${qso.submode}` : mode };
    }
    case 'date':
      return { text: date };
    case 'time':
      return { text: time };
    case 'cqzone':
      return { text: entity && entity.cq_zone != null ? String(entity.cq_zone) : '—' };
    case 'country':
      return entity
        ? { text: `${entity.prefix} · ${shortName(entity.name)}`, title: entityLabel(entity) }
        : { text: '—', title: '' };
    default:
      return { text: '' };
  }
}

/**
 * One box row: a cell per configured column, in order (feature 006, FR-004, FR-005). Text is
 * returned as is: the glue renders it with textContent only (feature 004, FR-022).
 */
export function rowCells(qso, columns) {
  return columns.map((column) => ({ column, title: '', ...cell(column, qso) }));
}

/** The marker key of a QSO's remote position (as `markers()` builds it), or null. */
export function markerKeyOf(qso) {
  return qso.remote ? `${qso.remote.lat},${qso.remote.lon}` : null;
}
