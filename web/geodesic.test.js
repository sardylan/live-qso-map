import { test } from 'node:test';
import assert from 'node:assert/strict';
import { greatCircle } from '../src/web/assets/geodesic.js';

const R = 6371008.8;
const rad = (d) => (d * Math.PI) / 180;

function vec([lat, lon]) {
  return [Math.cos(rad(lat)) * Math.cos(rad(lon)), Math.cos(rad(lat)) * Math.sin(rad(lon)), Math.sin(rad(lat))];
}

function cross(a, b) {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function dot(a, b) {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

function norm(a) {
  const l = Math.sqrt(dot(a, a));
  return a.map((x) => x / l);
}

test('returns segments+1 points from start to end', () => {
  const a = [39.2, 9.2];
  const b = [52.5, 13.4];
  const pts = greatCircle(a, b, 64);
  assert.equal(pts.length, 65);
  assert.deepEqual(pts[0], a);
  assert.ok(Math.abs(pts[64][0] - b[0]) < 1e-9 && Math.abs(pts[64][1] - b[1]) < 1e-9);
});

test('every point lies on the great circle', () => {
  const a = [39.2, 9.2];
  const b = [52.5, 13.4];
  const n = norm(cross(vec(a), vec(b)));
  for (const p of greatCircle(a, b, 64)) {
    const crossTrack = Math.abs(Math.asin(dot(vec(p), n))) * R;
    assert.ok(crossTrack < 1, `cross-track ${crossTrack} m`);
  }
});

test('longitudes stay continuous across the antimeridian', () => {
  for (const b of [[21.3, -157.8], [-36.8, 174.7]]) {
    const pts = greatCircle([39.2, 9.2], b, 64);
    for (let i = 1; i < pts.length; i++) {
      assert.ok(Math.abs(pts[i][1] - pts[i - 1][1]) < 180, `jump at ${i}`);
    }
  }
  const pacific = greatCircle([35.7, 139.7], [37.8, -122.4], 64);
  assert.ok(pacific[pacific.length - 1][1] > 180, 'unwrapped past 180°');
});

test('identical endpoints give two points', () => {
  assert.deepEqual(greatCircle([10, 20], [10, 20], 64), [[10, 20], [10, 20]]);
});
