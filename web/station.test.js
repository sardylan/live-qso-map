import { test } from 'node:test';
import assert from 'node:assert/strict';
import { stationText, STATION_STALE_MS, lastQsoTitle, localQsoTime } from '../src/web/assets/station.js';
import { isStale } from '../src/web/assets/score.js';

test('stationText shows dashes before any status', () => {
  assert.equal(stationText(null), '— · —');
});

test('stationText shows band then operator', () => {
  assert.equal(stationText({ band: '40m', operator: 'IK0ABC' }), '40m · IK0ABC');
});

test('stationText puts a dash in place of a missing value', () => {
  assert.equal(stationText({ band: null, operator: 'IK0ABC' }), '— · IK0ABC');
  assert.equal(stationText({ band: '20m', operator: null }), '20m · —');
});

// --- staleness and tooltip (T016) ---------------------------------------------------------------

test('the status is stale after 15 minutes', () => {
  assert.equal(STATION_STALE_MS, 900_000);
  assert.equal(isStale(0, 900_000, STATION_STALE_MS), false);
  assert.equal(isStale(0, 900_001, STATION_STALE_MS), true);
});

test('lastQsoTitle gives the QSO time in UTC', () => {
  assert.equal(lastQsoTitle('2026-10-08T15:04:29Z'), 'Last QSO 15:04Z');
  assert.equal(lastQsoTitle(null), '');
  assert.equal(lastQsoTitle('yesterday'), '');
});

test('localQsoTime places the QSO on the page clock', () => {
  assert.equal(localQsoTime(42, 1_000_000), 958_000);
  assert.equal(localQsoTime(null, 1_000_000), null);
});
