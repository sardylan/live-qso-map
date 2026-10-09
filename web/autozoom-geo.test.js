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

import { boundingCap, fitsGlobe, globeZoomForCap, capScreenRadius } from '../src/web/assets/autozoom-geo.js';

const IS = [39.2, 9.1];
const JA = [35.7, 139.7];
const W6 = [34.0, -118.2];
const ZL = [-41.3, 174.8];
const PY = [-23.5, -46.6];
const KH6 = [21.3, -157.9];
const D2 = [-18.1, 178.4]; // Fiji (3D2)

function angle([lat1, lon1], [lat2, lon2]) {
  const r = Math.PI / 180;
  const c = Math.sin(lat1 * r) * Math.sin(lat2 * r) + Math.cos(lat1 * r) * Math.cos(lat2 * r) * Math.cos((lon2 - lon1) * r);
  return Math.acos(Math.min(1, Math.max(-1, c))) / r;
}

test('boundingCap of European points is small and centred in Europe', () => {
  const cap = boundingCap([IS, [51.5, -0.1], [52.5, 13.4], [41.9, 12.5]]);
  assert.ok(cap.radiusDeg < 30, `radius ${cap.radiusDeg}`);
  assert.ok(cap.center[0] > 35 && cap.center[0] < 60 && cap.center[1] > -10 && cap.center[1] < 30, `centre ${cap.center}`);
});

test('boundingCap contains every point', () => {
  const points = [IS, JA, W6, ZL, KH6];
  const cap = boundingCap(points);
  for (const p of points) assert.ok(angle(cap.center, p) <= cap.radiusDeg + 1e-6);
});

test('IS, JA and W6 fit on one side of the globe, seen from the north', () => {
  const cap = boundingCap([IS, JA, W6]);
  assert.ok(cap.radiusDeg < 70, `radius ${cap.radiusDeg}`);
  assert.ok(cap.center[0] > 60, `centre ${cap.center}`);
});

test('IS, ZL, PY and JA cannot fit on one side of the globe', () => {
  assert.ok(boundingCap([IS, ZL, PY, JA]).radiusDeg > 90);
});

test('a Pacific set is centred in the Pacific, not at longitude 0', () => {
  const cap = boundingCap([ZL, KH6, D2]);
  const lon = cap.center[1];
  assert.ok(lon > 150 || lon < -150, `centre ${cap.center}`);
});

test('boundingCap of one point has radius 0', () => {
  const cap = boundingCap([IS]);
  assert.ok(cap.radiusDeg < 1e-6);
  assert.ok(angle(cap.center, IS) < 1e-6);
});

test('boundingCap of 2,000 points stays fast', () => {
  let seed = 7;
  const random = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const points = Array.from({ length: 2000 }, () => [random() * 140 - 70, random() * 360 - 180]);
  const times = [];
  for (let i = 0; i < 20; i++) {
    const start = performance.now();
    boundingCap(points);
    times.push(performance.now() - start);
  }
  times.sort((a, b) => a - b);
  // A loose limit for slow machines; the 5 ms plan target is measured by hand (T051).
  assert.ok(times[10] < 50, `median ${times[10]} ms`);
});

test('fitsGlobe needs the cap plus the margin within a hemisphere', () => {
  assert.equal(fitsGlobe({ center: [0, 0], radiusDeg: 80 }, 5), true);
  assert.equal(fitsGlobe({ center: [0, 0], radiusDeg: 88 }, 5), false);
});

const VIEW = { width: 1280, height: 800, padding: { top: 12, right: 150, bottom: 60, left: 100 } };
const FREE_HALF = Math.min(1280 - 100 - 150, 800 - 12 - 60) / 2;

test('globeZoomForCap: a tiny cap stops at zoom 12', () => {
  assert.equal(globeZoomForCap({ center: [39, 9], radiusDeg: 0 }, VIEW), 12);
  assert.equal(globeZoomForCap({ center: [39, 9], radiusDeg: 0.001 }, VIEW), 12);
});

test('globeZoomForCap: a bigger cap needs a lower zoom', () => {
  const zooms = [5, 20, 40, 60, 80].map((r) => globeZoomForCap({ center: [20, 0], radiusDeg: r }, VIEW));
  for (let i = 1; i < zooms.length; i++) assert.ok(zooms[i] < zooms[i - 1], `${zooms}`);
});

test('globeZoomForCap: the cap edge lands inside the free area and in front of the limb', () => {
  for (const cap of [
    { center: [20, 0], radiusDeg: 30 },
    { center: [-30, 150], radiusDeg: 70 },
    { center: [60, 20], radiusDeg: 50 },
  ]) {
    const zoom = globeZoomForCap(cap, VIEW);
    assert.notEqual(zoom, null, JSON.stringify(cap));
    const edge = capScreenRadius(cap, zoom, VIEW);
    assert.ok(edge !== null && edge <= FREE_HALF + 1e-6, `${JSON.stringify(cap)}: ${edge} > ${FREE_HALF}`);
  }
});

test('globeZoomForCap: no zoom when the cap reaches the limb', () => {
  assert.equal(globeZoomForCap({ center: [0, 0], radiusDeg: 88 }, VIEW), null);
});

test('globeZoomForCap: IS, JA, W6 and DL fit on a globe seen from the Arctic', () => {
  // The bug seen in the browser: this set fits, but the camera zoomed into the Arctic. The zoom
  // must account for the globe growing with 1/cos(latitude) and for the perspective.
  const cap = boundingCap([IS, JA, W6, [51.5, 13.4]]);
  const zoom = globeZoomForCap(cap, VIEW);
  assert.notEqual(zoom, null);
  assert.ok(zoom < 3, `zoom ${zoom}`);
  const edge = capScreenRadius(cap, zoom, VIEW);
  assert.ok(edge !== null && edge <= FREE_HALF + 1e-6, `${edge}`);
});

test('globeZoomForCap: no zoom when even the minimum zoom shows too little', () => {
  // Near the pole the globe is drawn bigger, so the minimum zoom (-2) cannot shrink it enough.
  assert.equal(globeZoomForCap({ center: [85, 0], radiusDeg: 80 }, VIEW), null);
});
