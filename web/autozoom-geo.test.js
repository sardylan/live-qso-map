import { test } from 'node:test';
import assert from 'node:assert/strict';
import { framedPoints, fallbackFrame, flatBounds, paddingFor } from '../src/web/assets/autozoom-geo.js';

const near = (a, b, eps = 1e-6) => Math.abs(a - b) < eps;
const QTH = { lat: 39.2, lon: 9.1 };

function markersAt(...points) {
  return new Map(points.map(([lat, lon]) => [`${lat},${lon}`, { key: `${lat},${lon}`, lat, lon }]));
}

test('framedPoints lists the QTH first, then every marker', () => {
  assert.deepEqual(framedPoints(QTH, markersAt([50, 10], [35, 139])), [[39.2, 9.1], [50, 10], [35, 139]]);
  assert.deepEqual(framedPoints(null, markersAt([50, 10])), [[50, 10]]);
  assert.deepEqual(framedPoints(null, new Map()), []);
});

test('fallbackFrame uses the entity bounds, unwrapped across the antimeridian', () => {
  assert.deepEqual(fallbackFrame(QTH, { bounds: [8.13, 38.86, 9.83, 41.31] }), [[8.13, 38.86], [9.83, 41.31]]);
  assert.deepEqual(fallbackFrame(QTH, { bounds: [176.8, -19.3, -178.2, -16.0] }), [[176.8, -19.3], [181.8, -16.0]]);
});

test('fallbackFrame without an entity is a 500 km box around the QTH', () => {
  const [[w, s], [e, n]] = fallbackFrame(QTH, null);
  const halfLat = 500 / 111.195;
  assert.ok(near(n - QTH.lat, halfLat, 1e-3) && near(QTH.lat - s, halfLat, 1e-3));
  const halfLon = halfLat / Math.cos((QTH.lat * Math.PI) / 180);
  assert.ok(near(e - QTH.lon, halfLon, 1e-3) && near(QTH.lon - w, halfLon, 1e-3));
});

test('fallbackFrame without a QTH is null (the whole world)', () => {
  assert.equal(fallbackFrame(null, { bounds: [8, 38, 9, 41] }), null);
  assert.equal(fallbackFrame(null, null), null);
});

test('flatBounds takes the ordinary span when it is the shortest', () => {
  assert.deepEqual(flatBounds([[39.2, 9.1], [35.7, 139.7]]), [[9.1, 35.7], [139.7, 39.2]]);
});

test('flatBounds takes the short way across the antimeridian', () => {
  // Sardinia (9°) and New Zealand (175°): 166° going east, 194° going west, so east wins.
  assert.deepEqual(flatBounds([[39.2, 9.1], [-41.3, 174.8]]), [[9.1, -41.3], [174.8, 39.2]]);
  // Japan and Hawaii: the short way crosses 180°, so east is above 180.
  const [[w, s], [e, n]] = flatBounds([[35.7, 139.7], [21.3, -157.9]]);
  assert.deepEqual([w, s, n], [139.7, 21.3, 35.7]);
  assert.ok(near(e, 202.1));
});

test('flatBounds of one point is a degenerate box; of none is null', () => {
  assert.deepEqual(flatBounds([[10, 20]]), [[20, 10], [20, 10]]);
  assert.equal(flatBounds([]), null);
});

const MAP = { left: 0, top: 0, right: 1000, bottom: 800, width: 1000, height: 800 };
const rect = (left, top, width, height) => ({ left, top, right: left + width, bottom: top + height, width, height });
const OPTIONS = { gap: 12, minFree: [240, 160] };

test('paddingFor pads each edge by the deepest panel against it, plus the gap', () => {
  // A panel against two edges pads the one that costs less depth.
  const legend = rect(870, 570, 120, 200); // bottom right, tall: the right edge
  const box = rect(10, 600, 300, 160); // bottom left, wide: the bottom edge
  const selector = rect(880, 10, 110, 30); // top right, flat: the top edge
  assert.deepEqual(paddingFor(MAP, [legend, box, selector], OPTIONS), { top: 52, right: 142, bottom: 212, left: 12 });
});

test('a floating panel pads its nearest edge', () => {
  assert.deepEqual(paddingFor(MAP, [rect(100, 300, 50, 50)], OPTIONS), { top: 12, right: 12, bottom: 12, left: 162 });
});

test('paddingFor ignores hidden panels', () => {
  assert.deepEqual(paddingFor(MAP, [null, rect(0, 0, 0, 0)], OPTIONS), { top: 12, right: 12, bottom: 12, left: 12 });
});

test('paddingFor falls back to a uniform gap when too little room is left', () => {
  const huge = rect(0, 200, 1000, 600); // bottom: leaves 200 px of height... and more panels
  const tall = rect(0, 0, 900, 150);
  assert.deepEqual(paddingFor(MAP, [huge, tall], OPTIONS), { top: 12, right: 12, bottom: 12, left: 12 });
});
