import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applySnapshot, applyUpsert, applyRemove, setLimit, markers } from '../src/web/assets/map-state.js';
import { recentQsos, rowView, shortName, entityLabel } from '../src/web/assets/recent.js';

function qso(n, overrides = {}) {
  const minute = String(n).padStart(2, '0');
  return {
    id: `Q${minute}`,
    rev: 1,
    sort_key: `2026-10-08T15:${minute}:00Z`,
    datetime: `2026-10-08T15:${minute}:00Z`,
    call: `K${n}AA`,
    band: '20m',
    mode: 'CW',
    submode: null,
    remote: { lat: 40 + n, lon: -70 - n, precision: 'locator4', source: 'logger' },
    station: null,
    ...overrides,
  };
}

function stateOf(qsos, limit = 20) {
  const s = createState({ limit });
  applySnapshot(s, qsos);
  return s;
}

const ids = (list) => list.map((q) => q.id);

test('the five newest by QSO time, newest first', () => {
  const s = stateOf([1, 2, 3, 4, 5, 6, 7].map((n) => qso(n)));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q06', 'Q05', 'Q04', 'Q03']);
});

test('a late QSO with an older time does not enter the box', () => {
  const s = stateOf([3, 4, 5, 6, 7].map((n) => qso(n)));
  applyUpsert(s, qso(1));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q06', 'Q05', 'Q04', 'Q03']);
});

test('a correction that changes the time re-orders the box', () => {
  const s = stateOf([3, 4, 5, 6, 7].map((n) => qso(n)));
  applyUpsert(s, qso(3, { rev: 2, sort_key: '2026-10-08T15:59:00Z', datetime: '2026-10-08T15:59:00Z' }));
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q03', 'Q07', 'Q06', 'Q05', 'Q04']);
});

test('a removal moves the next one up', () => {
  const s = stateOf([1, 2, 3, 4, 5, 6, 7].map((n) => qso(n)));
  applyRemove(s, 'Q06');
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q07', 'Q05', 'Q04', 'Q03', 'Q02']);
});

test('fewer than five gives them all; none gives an empty list', () => {
  assert.deepEqual(ids(recentQsos(stateOf([qso(1), qso(2)]), 5)), ['Q02', 'Q01']);
  assert.deepEqual(recentQsos(stateOf([]), 5), []);
});

test('the box follows the limit window', () => {
  const s = stateOf(Array.from({ length: 20 }, (_, i) => qso(i + 1)), 20);
  setLimit(s, 10);
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q20', 'Q19', 'Q18', 'Q17', 'Q16']);
});

test('QSOs without a remote position are listed', () => {
  const s = stateOf([qso(1), qso(2, { remote: null })]);
  assert.deepEqual(ids(recentQsos(s, 5)), ['Q02', 'Q01']);
});

test('rowView: call, band, UTC date and time, entity and marker key; no mode', () => {
  const s = stateOf([qso(4, { mode: 'SSB', submode: 'USB', entity: { prefix: 'JA', name: 'Japan' } })]);
  const row = rowView(recentQsos(s, 5)[0]);
  const keys = [...markers(s).keys()];
  assert.deepEqual(row, {
    call: 'K4AA',
    band: '20m',
    date: '2026-10-08',
    time: '15:04Z',
    entity: 'JA · Japan',
    entityTitle: 'JA · Japan',
    markerKey: keys[0],
  });
  assert.ok(!('mode' in row));
});

test('rowView cuts a long entity name but keeps the full label as the title', () => {
  const row = rowView(qso(1, { entity: { prefix: 'TL', name: 'Central African Republic' } }));
  assert.equal(row.entity, 'TL · Central African R…');
  assert.equal(row.entityTitle, 'TL · Central African Republic');
});

test('rowView of an unknown entity shows a dash and no title', () => {
  const row = rowView(qso(1, { entity: null }));
  assert.equal(row.entity, '—');
  assert.equal(row.entityTitle, '');
  assert.equal(rowView(qso(2)).entity, '—', 'a QSO without the field');
});

test('rowView without a remote position has no marker key', () => {
  assert.equal(rowView(qso(1, { remote: null })).markerKey, null);
});

test('rowView shows different dates either side of midnight UTC', () => {
  const before = rowView(qso(1, { datetime: '2026-10-09T23:59:00Z' }));
  const after = rowView(qso(2, { datetime: '2026-10-10T00:01:00Z' }));
  assert.deepEqual([before.date, before.time], ['2026-10-09', '23:59Z']);
  assert.deepEqual([after.date, after.time], ['2026-10-10', '00:01Z']);
});

test("rowView keeps text verbatim; escaping is the page glue's job", () => {
  const row = rowView(qso(1, { call: '<b>X</b>', datetime: null, band: null }));
  assert.equal(row.call, '<b>X</b>');
  assert.equal(row.date, '');
  assert.equal(row.time, '');
  assert.equal(row.band, '');
});

test('shortName keeps names up to 18 characters and cuts longer ones to 18 with an ellipsis', () => {
  assert.equal(shortName('Japan'), 'Japan');
  assert.equal(shortName('Wallis & Futuna Is'), 'Wallis & Futuna Is');
  assert.equal(shortName('Central African Republic'), 'Central African R…');
  assert.equal(shortName('Central African Republic').length, 18);
  // A cut that ends in a space is trimmed before the ellipsis.
  assert.equal(shortName('Tristan da Cunha & Gough'), 'Tristan da Cunha…');
});

test('entityLabel gives prefix · name, or a dash', () => {
  assert.equal(entityLabel({ prefix: 'JA', name: 'Japan' }), 'JA · Japan');
  assert.equal(entityLabel({ prefix: 'TL', name: 'Central African Republic' }), 'TL · Central African Republic');
  assert.equal(entityLabel(null), '—');
  assert.equal(entityLabel(undefined), '—');
});
