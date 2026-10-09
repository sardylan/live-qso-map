import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applySnapshot, applyUpsert, applyRemove, setLimit, markers } from '../src/web/assets/map-state.js';
import { recentQsos, rowView } from '../src/web/assets/recent.js';

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

test('rowView: call, band, mode with submode, UTC time and marker key', () => {
  const s = stateOf([qso(4, { mode: 'SSB', submode: 'USB' })]);
  const row = rowView(recentQsos(s, 5)[0]);
  const keys = [...markers(s).keys()];
  assert.deepEqual(row, { call: 'K4AA', band: '20m', mode: 'SSB/USB', time: '15:04Z', markerKey: keys[0] });
  assert.equal(rowView(qso(1)).mode, 'CW');
});

test('rowView without a remote position has no marker key', () => {
  assert.equal(rowView(qso(1, { remote: null })).markerKey, null);
});

test('rowView keeps text verbatim; escaping is the page glue\'s job', () => {
  const row = rowView(qso(1, { call: '<b>X</b>', datetime: null, band: null, mode: null }));
  assert.equal(row.call, '<b>X</b>');
  assert.equal(row.time, '');
  assert.equal(row.band, '');
  assert.equal(row.mode, '');
});
