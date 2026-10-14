import { test } from 'node:test';
import assert from 'node:assert/strict';
import { solarElevation, subsolarPoint } from '../src/web/assets/sun.js';

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

// --- subsolar point (feature 013, research R3, T002) --------------------------------------

test('the subsolar point is on the equator, near Greenwich, at the March equinox noon', () => {
  const p = subsolarPoint(at('2026-03-20T12:00:00Z'));
  assert.ok(Math.abs(p.lat) < 0.5, `lat ${p.lat}`);
  // The equation of time is about −7.5 minutes then, so the Sun is overhead ~1.9° east.
  assert.ok(Math.abs(p.lon) < 2.5, `lon ${p.lon}`);
});

test('the subsolar latitude is the tropic at the solstices', () => {
  const june = subsolarPoint(at('2026-06-21T12:00:00Z'));
  const december = subsolarPoint(at('2026-12-21T12:00:00Z'));
  assert.ok(Math.abs(june.lat - 23.44) < 0.1, `June ${june.lat}`);
  assert.ok(Math.abs(december.lat + 23.44) < 0.1, `December ${december.lat}`);
});

test('the subsolar point moves west 15° per hour', () => {
  const a = subsolarPoint(at('2026-08-01T10:00:00Z'));
  const b = subsolarPoint(at('2026-08-01T11:00:00Z'));
  const west = (((a.lon - b.lon) % 360) + 360) % 360;
  assert.ok(Math.abs(west - 15) < 0.1, `${west}°`);
});

test('the subsolar longitude stays in [−180, 180) around midnight UTC', () => {
  for (const iso of ['2026-01-10T00:00:00Z', '2026-05-10T23:59:00Z', '2026-11-03T00:20:00Z']) {
    const { lon } = subsolarPoint(at(iso));
    assert.ok(lon >= -180 && lon < 180, `${iso}: ${lon}`);
    assert.ok(Math.abs(Math.abs(lon) - 180) < 10, `${iso}: near the date line, ${lon}`);
  }
});

test('the Sun is overhead at the subsolar point, by the same equations', () => {
  for (let month = 0; month < 12; month++) {
    const date = new Date(Date.UTC(2026, month, 5 + month * 2, (month * 7) % 24, 13));
    const p = subsolarPoint(date);
    const e = solarElevation(date, p.lat, p.lon);
    assert.ok(e >= 89.5, `${date.toISOString()}: ${e}`);
  }
});
