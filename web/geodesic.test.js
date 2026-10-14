import { test } from 'node:test';
import assert from 'node:assert/strict';
import { greatCircle, destination } from '../src/web/assets/geodesic.js';

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

// --- destination (feature 013, research R7, T015) -----------------------------------------

const near = (a, b, eps = 1e-9) => Math.abs(a - b) < eps;
const angle = ([la1, lo1], [la2, lo2]) => {
  const d = Math.PI / 180;
  const c = Math.sin(la1 * d) * Math.sin(la2 * d) + Math.cos(la1 * d) * Math.cos(la2 * d) * Math.cos((lo2 - lo1) * d);
  return Math.acos(Math.min(1, Math.max(-1, c))) / d;
};

test('destination: zero distance is the start point', () => {
  const [lat, lon] = destination([39.2, 9.1], 77, 0);
  assert.ok(near(lat, 39.2) && near(lon, 9.1));
});

test('destination: due north and due east from the equator', () => {
  assert.ok(near(destination([0, 0], 0, 90)[0], 90));
  const east = destination([0, 0], 90, 90);
  assert.ok(near(east[0], 0) && near(east[1], 90), `${east}`);
});

test('destination: 180° gives the antipode', () => {
  const [lat, lon] = destination([45, 10], 33, 180);
  assert.ok(near(lat, -45, 1e-7), `${lat}`);
  assert.ok(near(((lon + 540) % 360) - 180, -170, 1e-7), `${lon}`);
});

test('destination: the great-circle distance is the one asked for', () => {
  for (const bearing of [0, 45, 135, 250, 359]) {
    const to = destination([-33.9, 151.2], bearing, 30);
    assert.ok(near(angle([-33.9, 151.2], to), 30, 1e-9), `${bearing}: ${angle([-33.9, 151.2], to)}`);
  }
});
