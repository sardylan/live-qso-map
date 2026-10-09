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

/**
 * One box row (feature 005, FR-006): callsign, band, UTC date and time, and the entity with its
 * name cut; the full label is the title. Text is returned as is: the glue renders it with
 * textContent only (FR-022 of feature 004).
 */
export function rowView(qso) {
  const [date, time] = utcDateTime(qso.datetime);
  const entity = qso.entity ?? null;
  return {
    call: qso.call ?? '',
    band: qso.band ?? '',
    date,
    time,
    entity: entity ? `${entity.prefix} · ${shortName(entity.name)}` : '—',
    entityTitle: entity ? entityLabel(entity) : '',
    markerKey: qso.remote ? `${qso.remote.lat},${qso.remote.lon}` : null,
  };
}
