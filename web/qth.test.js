import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createState, applyUpsert, applyRemove } from '../src/web/assets/map-state.js';
import { currentQth, houseFeature, HOUSE_SVG } from '../src/web/assets/qth.js';

const CONFIGURED = {
  callsign: 'IS0MZK',
  locator: 'JM49NH',
  position: { lat: 39.3125, lon: 9.125, precision: 'locator6', source: 'config' },
};

function qso(n, station, overrides = {}) {
  const minute = String(n).padStart(2, '0');
  return {
    id: `Q${minute}`,
    rev: 1,
    sort_key: `2026-10-08T15:${minute}:00Z`,
    call: `K${n}AA`,
    band: '40m',
    station_call: `IS0S${n}`,
    remote: { lat: 50, lon: 10, precision: 'locator6', source: 'logger' },
    station,
    adif: station ? { MY_GRIDSQUARE: `JM4${n}` } : {},
    ...overrides,
  };
}

function stateOf(...qsos) {
  const s = createState({ limit: 20 });
  for (const q of qsos) applyUpsert(s, q);
  return s;
}

const at = (lat, lon) => ({ lat, lon, precision: 'locator4', source: 'logger' });

test('a configured position wins over QSO station positions', () => {
  const qth = currentQth(CONFIGURED, stateOf(qso(1, at(45, 7))));
  assert.deepEqual(qth, {
    lat: 39.3125,
    lon: 9.125,
    callsign: 'IS0MZK',
    locator: 'JM49NH',
    source: 'config',
    precision: 'locator6',
    origin: 'config',
  });
});

test('without a configured position, the newest QSO by sort_key supplies the QTH', () => {
  // Received out of order: QSO 3 is the newest by QSO time.
  const s = stateOf(qso(3, at(40, 9)), qso(1, at(41, 8)), qso(2, at(42, 7)));
  const qth = currentQth({ callsign: 'IS0MZK', locator: null, position: null }, s);
  assert.deepEqual(qth, {
    lat: 40,
    lon: 9,
    callsign: 'IS0S3',
    locator: 'JM43',
    source: 'logger',
    precision: 'locator4',
    origin: 'qso',
  });
});

test('QSOs without a station position are skipped', () => {
  const s = stateOf(qso(1, at(41, 8)), qso(2, null));
  assert.equal(currentQth(null, s).lat, 41);
});

test('no QTH when nothing is known', () => {
  assert.equal(currentQth(null, stateOf()), null);
  assert.equal(currentQth({ callsign: 'X', locator: null, position: null }, stateOf(qso(1, null))), null);
  assert.equal(currentQth(undefined, null), null);
});

test('removing the QSO that supplied the QTH falls back to the next newest', () => {
  const s = stateOf(qso(1, at(41, 8)), qso(2, at(42, 7)));
  assert.equal(currentQth(null, s).lat, 42);
  applyRemove(s, 'Q02');
  assert.equal(currentQth(null, s).lat, 41);
});

test('houseFeature gives zero or one Point in [lon, lat]', () => {
  assert.deepEqual(houseFeature(null), { type: 'FeatureCollection', features: [] });
  const fc = houseFeature(currentQth(CONFIGURED, stateOf()));
  assert.equal(fc.features.length, 1);
  assert.deepEqual(fc.features[0].geometry, { type: 'Point', coordinates: [9.125, 39.3125] });
});

test('HOUSE_SVG is a constant SVG document', () => {
  assert.equal(typeof HOUSE_SVG, 'string');
  assert.ok(HOUSE_SVG.startsWith('<svg'));
  assert.ok(!HOUSE_SVG.includes('{'));
  assert.ok(HOUSE_SVG.includes('width="28"') && HOUSE_SVG.includes('height="28"'));
});
