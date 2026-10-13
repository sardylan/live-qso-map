import { test } from 'node:test';
import assert from 'node:assert/strict';
import { solarElevation } from '../src/web/assets/sun.js';

const at = (iso) => new Date(iso);

test('the sun is high at noon in Cagliari at the March equinox', () => {
  const e = solarElevation(at('2026-03-20T11:30:00Z'), 39.22, 9.12);
  assert.ok(e > 49 && e < 52, `${e}`);
});

test('the sun is far below the horizon at night in Cagliari', () => {
  assert.ok(solarElevation(at('2026-03-20T23:30:00Z'), 39.22, 9.12) < -40);
});

test('polar day and polar night in Tromsø', () => {
  assert.ok(solarElevation(at('2026-06-21T23:00:00Z'), 69.65, 18.96) > 0, 'midnight sun');
  assert.ok(solarElevation(at('2026-12-21T11:00:00Z'), 69.65, 18.96) < 0, 'polar night at noon');
});

test('the sun is almost overhead at the equator at the equinox, local noon', () => {
  assert.ok(solarElevation(at('2026-03-20T12:07:00Z'), 0, 0) > 85);
});

test('longitude moves noon: noon in Tokyo is night in Cagliari', () => {
  const when = at('2026-06-21T03:00:00Z');
  assert.ok(solarElevation(when, 35.68, 139.69) > 60, 'Tokyo noon');
  assert.ok(solarElevation(when, 39.22, 9.12) < 0, 'Cagliari 05:00 local, before sunrise');
});
