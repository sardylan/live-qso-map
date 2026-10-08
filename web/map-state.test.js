import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applySnapshot, applyUpsert, visible, markers, diffLayers, setLimit } from '../src/web/assets/map-state.js';

const bands = { '40m': '#d62728', '20m': '#1f77b4' };
const STATION = { lat: 39.3125, lon: 9.125, precision: 'locator6', source: 'config' };

function qso(n, overrides = {}) {
  const minute = String(n).padStart(2, '0');
  return {
    id: `Q${minute}`,
    rev: 1,
    sort_key: `2026-10-08T15:${minute}:00Z`,
    call: `K${n}AA`,
    band: '40m',
    mode: 'SSB',
    remote: { lat: 50 + n, lon: 10, precision: 'locator6', source: 'logger' },
    station: STATION,
    ...overrides,
  };
}

function state(limit = 20) {
  return createState({ bands, fallbackColor: '#7f7f7f', limit });
}

test('applyUpsert adds a QSO', () => {
  const s = state();
  applyUpsert(s, qso(1));
  assert.equal(visible(s).length, 1);
  assert.equal(markers(s).size, 1);
});

test('QSOs at exactly the same position share one marker', () => {
  const s = state();
  const remote = { lat: 42.5, lon: -71, precision: 'locator4', source: 'logger' };
  applyUpsert(s, qso(1, { remote }));
  applyUpsert(s, qso(2, { remote: { ...remote } }));
  applyUpsert(s, qso(3));
  const m = markers(s);
  assert.equal(m.size, 2);
  assert.deepEqual(m.get('42.5,-71').qsos.map((q) => q.id), ['Q01', 'Q02']);
});

test('marker colour follows the band of its most recent QSO', () => {
  const s = state();
  const remote = { lat: 42.5, lon: -71, precision: 'locator4', source: 'logger' };
  applyUpsert(s, qso(2, { remote, band: '20m' }));
  applyUpsert(s, qso(1, { remote, band: '40m' }));
  assert.equal(markers(s).get('42.5,-71').color, '#1f77b4');
  applyUpsert(s, qso(3, { remote, band: '99m' }));
  assert.equal(markers(s).get('42.5,-71').color, '#7f7f7f');
  applyUpsert(s, qso(4, { remote, band: null }));
  assert.equal(markers(s).get('42.5,-71').color, '#7f7f7f');
});

test('z-rank follows sort_key, newest highest', () => {
  const s = state();
  applyUpsert(s, qso(3));
  applyUpsert(s, qso(1));
  applyUpsert(s, qso(2));
  const m = markers(s);
  const rank = (n) => m.get(`${50 + n},10`).zRank;
  assert.ok(rank(1) < rank(2) && rank(2) < rank(3));
});

test('line comes from the station of the most recent QSO', () => {
  const s = state();
  const remote = { lat: 42.5, lon: -71, precision: 'locator4', source: 'logger' };
  const other = { lat: 45, lon: 7, precision: 'coordinates', source: 'logger' };
  applyUpsert(s, qso(1, { remote }));
  applyUpsert(s, qso(2, { remote, station: other }));
  assert.deepEqual(markers(s).get('42.5,-71').station, other);
  applyUpsert(s, qso(3, { station: null }));
  assert.equal(markers(s).get('53,10').station, null);
});

test('a QSO without remote position is visible but draws nothing', () => {
  const s = state();
  applyUpsert(s, qso(1, { remote: null }));
  assert.equal(visible(s).length, 1);
  assert.equal(markers(s).size, 0);
});

test('applySnapshot replaces the state', () => {
  const s = state();
  applyUpsert(s, qso(9));
  applySnapshot(s, [qso(1), qso(2)]);
  assert.deepEqual(visible(s).map((q) => q.id), ['Q01', 'Q02']);
});

test('diffLayers classifies added, updated and removed markers', () => {
  const s = state();
  applyUpsert(s, qso(1));
  applyUpsert(s, qso(2));
  const before = markers(s);
  assert.deepEqual(diffLayers(before, before), { add: [], update: [], remove: [] });

  applySnapshot(s, [qso(2, { band: '20m' }), qso(3)]);
  const after = markers(s);
  const d = diffLayers(before, after);
  assert.deepEqual(d.add.map((m) => m.key), ['53,10']);
  assert.deepEqual(d.update.map((m) => m.key), ['52,10']);
  assert.deepEqual(d.remove, ['51,10']);
});

test('diffLayers detects z-rank, popup list and station changes', () => {
  const s = state();
  applyUpsert(s, qso(1));
  const before = markers(s);
  applyUpsert(s, qso(2, { remote: { lat: 51, lon: 10, precision: 'locator6', source: 'logger' } }));
  assert.deepEqual(diffLayers(before, markers(s)).update.map((m) => m.key), ['51,10']);

  const s2 = state();
  applyUpsert(s2, qso(1));
  const b2 = markers(s2);
  applyUpsert(s2, qso(1, { rev: 2, station: { ...STATION, lat: 1 } }));
  assert.equal(diffLayers(b2, markers(s2)).update.length, 1);
});

test('the window keeps only the last N by sort_key', () => {
  const s = state(20);
  for (let n = 1; n <= 25; n++) applyUpsert(s, qso(n));
  const ids = visible(s).map((q) => q.id);
  assert.equal(ids.length, 20);
  assert.equal(ids[0], 'Q06');
  assert.equal(ids[19], 'Q25');
});

test('a marker disappears when none of its QSOs is visible', () => {
  const s = state(1);
  applyUpsert(s, qso(1));
  applyUpsert(s, qso(2));
  assert.deepEqual([...markers(s).keys()], ['52,10']);
});

test('setLimit(null) shows everything', () => {
  const s = state(10);
  for (let n = 1; n <= 30; n++) applyUpsert(s, qso(n));
  setLimit(s, null);
  assert.equal(visible(s).length, 30);
  setLimit(s, 5);
  assert.equal(visible(s).length, 5);
});

test('late arrivals are placed by sort_key, not arrival', () => {
  const s = state(2);
  applyUpsert(s, qso(5));
  applyUpsert(s, qso(6));
  applyUpsert(s, qso(1));
  assert.deepEqual(visible(s).map((q) => q.id), ['Q05', 'Q06']);
});
