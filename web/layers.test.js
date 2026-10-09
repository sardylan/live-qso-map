import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applyUpsert, markers } from '../src/web/assets/map-state.js';
import { greatCircle } from '../src/web/assets/geodesic.js';
import { markerFeatures, lineFeatures } from '../src/web/assets/layers.js';

const bands = [
  { band: '40m', color: '#d62728' },
  { band: '20m', color: '#1f77b4' },
];
const STATION = { lat: 39.2, lon: 9.2, precision: 'locator6', source: 'config' };

function qso(n, overrides = {}) {
  const minute = String(n).padStart(2, '0');
  return {
    id: `Q${minute}`,
    rev: 1,
    sort_key: `2026-10-08T15:${minute}:00Z`,
    call: `K${n}AA`,
    band: '40m',
    remote: { lat: 50 + n, lon: 10 + n, precision: 'locator6', source: 'logger' },
    station: STATION,
    ...overrides,
  };
}

function markersOf(...qsos) {
  const s = createState({ bands, fallbackColor: '#7f7f7f', limit: 20 });
  for (const q of qsos) applyUpsert(s, q);
  return markers(s);
}

test('markerFeatures gives one Point per marker in [lon, lat]', () => {
  const fc = markerFeatures(markersOf(qso(1), qso(2, { band: '20m' })));
  assert.equal(fc.type, 'FeatureCollection');
  assert.equal(fc.features.length, 2);
  const [a, b] = fc.features;
  assert.equal(a.geometry.type, 'Point');
  assert.deepEqual(a.geometry.coordinates, [11, 51]);
  assert.deepEqual(a.properties, { key: '51,11', color: '#d62728', zRank: 0 });
  assert.deepEqual(b.properties, { key: '52,12', color: '#1f77b4', zRank: 1 });
});

test('markerFeatures are ordered by zRank ascending', () => {
  const fc = markerFeatures(markersOf(qso(3), qso(1), qso(2)));
  assert.deepEqual(fc.features.map((f) => f.properties.zRank), [0, 1, 2]);
});

const QTH = { lat: 39.2, lon: 9.2 };

test('every line starts at the QTH, whatever the QSO station', () => {
  const fc = lineFeatures(markersOf(qso(1, { station: { lat: 45, lon: 7, precision: 'locator4', source: 'logger' } })), QTH);
  assert.equal(fc.features.length, 1);
  const line = fc.features[0];
  assert.equal(line.geometry.type, 'LineString');
  const expected = greatCircle([39.2, 9.2], [51, 11], 64).map(([lat, lon]) => [lon, lat]);
  assert.deepEqual(line.geometry.coordinates, expected);
  assert.deepEqual(line.properties, { key: '51,11', zRank: 0 });
});

test('lines are drawn even for QSOs without a station, but not without a QTH', () => {
  assert.equal(lineFeatures(markersOf(qso(1, { station: null })), QTH).features.length, 1);
  assert.equal(lineFeatures(markersOf(qso(1)), null).features.length, 0);
  assert.equal(markerFeatures(markersOf(qso(1, { station: null }))).features.length, 1);
});

test('antimeridian lines stay continuous', () => {
  // From JM49 to Hawaii.
  const fc = lineFeatures(markersOf(qso(1, { remote: { lat: 21.3, lon: -157.8, precision: 'locator4', source: 'logger' } })), { lat: 39.5, lon: 9 });
  const coords = fc.features[0].geometry.coordinates;
  for (let i = 1; i < coords.length; i++) {
    assert.ok(Math.abs(coords[i][0] - coords[i - 1][0]) < 180, `jump at ${i}`);
  }
});

test('empty markers give empty collections', () => {
  assert.deepEqual(markerFeatures(new Map()), { type: 'FeatureCollection', features: [] });
  assert.deepEqual(lineFeatures(new Map(), QTH), { type: 'FeatureCollection', features: [] });
});
