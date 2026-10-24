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

import { boundingCap, globeZoomForCap, capScreenRadius } from '../src/web/assets/autozoom-geo.js';

const IS = [39.2, 9.1];
const JA = [35.7, 139.7];
const W6 = [34.0, -118.2];
const ZL = [-41.3, 174.8];
const PY = [-23.5, -46.6];
const KH6 = [21.3, -157.9];
const D2 = [-18.1, 178.4]; // Fiji (3D2)

// The operator's page snapshot (feature 016, spec Assumptions): IQ0ID in CQ WW SSB, 20 QSOs.
const SNAPSHOT = {
  qth: [39.051738, 8.37711],
  vk: [[-38.155567, 144.367475], [-34.938333, 117.873333]], // VK3ATL, VK6LG
  europe: [
    [58.571685, 16.553067], [57.955013, 12.455583], [53.600088, 11.428925], [52.77, -1.47],
    [51.803614, 1.140091], [51.675729, 9.003269], [51.02, 4.623333], [50.978099, -0.748722],
    [49.147049, 8.688986], [49.104532, 17.166542], [48.854167, 2.375], [48.069238, 1.334538],
    [44.895833, 20.375], [44.561667, 25.956667], [42.346452, -3.721318], [40.395833, -3.708333],
    [40.229167, -8.458333], [37.1578, -8.073143],
  ],
};
SNAPSHOT.all = [SNAPSHOT.qth, ...SNAPSHOT.vk, ...SNAPSHOT.europe];
// Panels of the live page at 1024 × 768 (research R1), reused at every size.
const PADDING = { top: 60, right: 100, bottom: 240, left: 110 };

test('the snapshot spans a cap of about 79.6°', () => {
  assert.ok(Math.abs(boundingCap(SNAPSHOT.all).radiusDeg - 79.6) < 0.5);
});

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

const VIEW = { width: 1280, height: 800, padding: { top: 12, right: 150, bottom: 60, left: 100 } };
const FREE_HALF = Math.min(1280 - 100 - 150, 800 - 12 - 60) / 2;

// The horizon angle, in degrees from the view centre, that the camera sees at `zoom` (research R3).
function horizonDeg(cap, zoom, height) {
  const c = (0.5 / Math.tan(0.6435011087932844 / 2)) * height;
  const r = (512 * 2 ** zoom) / (2 * Math.PI) / Math.cos((cap.center[0] * Math.PI) / 180);
  return Math.acos(r / (c + r)) / (Math.PI / 180);
}

// The highest zoom whose cap edge sits within half the free area, by bisection (R2).
function freeAreaFit(cap, view) {
  let lo = -2;
  let hi = 12;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    const edge = capScreenRadius(cap, mid, view);
    if (edge !== null && edge <= FREE_HALF) lo = mid;
    else hi = mid;
  }
  return lo;
}

test('globeZoomForCap: a tiny cap stops at zoom 12, whatever the border', () => {
  assert.equal(globeZoomForCap({ center: [39, 9], radiusDeg: 0 }, VIEW), 12);
  assert.equal(globeZoomForCap({ center: [39, 9], radiusDeg: 0.001 }, VIEW), 12);
  // At zoom 12 the horizon is about 4.7° away but far outside the map: the border is met.
  assert.equal(globeZoomForCap({ center: [39, 9], radiusDeg: 0.001 }, { ...VIEW, borderDeg: 45 }), 12);
});

test('globeZoomForCap: a bigger cap needs a lower zoom', () => {
  const zooms = [5, 20, 40, 60].map((r) => globeZoomForCap({ center: [20, 0], radiusDeg: r }, VIEW));
  for (let i = 1; i < zooms.length; i++) assert.ok(zooms[i] < zooms[i - 1], `${zooms}`);
});

test('globeZoomForCap: the zoom is the free-area fit, never lower (feature 016 FR-007, FR-008)', () => {
  for (const cap of [
    { center: [20, 0], radiusDeg: 30 },
    { center: [60, 20], radiusDeg: 50 },
    { center: [20, 0], radiusDeg: 60 },
  ]) {
    const zoom = globeZoomForCap(cap, VIEW);
    assert.notEqual(zoom, null, JSON.stringify(cap));
    assert.ok(Math.abs(zoom - freeAreaFit(cap, VIEW)) < 0.01, `${JSON.stringify(cap)}: ${zoom}`);
    const edge = capScreenRadius(cap, zoom, VIEW);
    assert.ok(edge !== null && edge <= FREE_HALF + 1e-6, `${JSON.stringify(cap)}: ${edge} > ${FREE_HALF}`);
    assert.ok(cap.radiusDeg + 10 <= horizonDeg(cap, zoom, VIEW.height), JSON.stringify(cap));
  }
});

test('globeZoomForCap: no zoom when the border does not fit inside the horizon', () => {
  // 70° fits the free area with only about 3° to the horizon: under the default 10°, flat.
  assert.equal(globeZoomForCap({ center: [-30, 150], radiusDeg: 70 }, VIEW), null);
  // 60° leaves about 12.7°: kept at 10°, flat at 15°.
  assert.notEqual(globeZoomForCap({ center: [20, 0], radiusDeg: 60 }, { ...VIEW, borderDeg: 10 }), null);
  assert.equal(globeZoomForCap({ center: [20, 0], radiusDeg: 60 }, { ...VIEW, borderDeg: 15 }), null);
  assert.equal(globeZoomForCap({ center: [0, 0], radiusDeg: 88 }, { ...VIEW, borderDeg: 0 }), null);
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

test('globeZoomForCap: the snapshot goes flat at both sizes, whatever the border', () => {
  // Feature 016 R1: the VK QSOs lie 1–3° behind the horizon at the free-area fit.
  const cap = boundingCap(SNAPSHOT.all);
  for (const size of [{ width: 1024, height: 768 }, { width: 1920, height: 960 }]) {
    for (const borderDeg of [0, 10]) {
      assert.equal(globeZoomForCap(cap, { ...size, padding: PADDING, borderDeg }), null, `${size.width} ${borderDeg}`);
    }
  }
});

test('globeZoomForCap: the European snapshot stays on a large globe', () => {
  const cap = boundingCap([SNAPSHOT.qth, ...SNAPSHOT.europe]);
  const zoom = globeZoomForCap(cap, { width: 1920, height: 960, padding: PADDING, borderDeg: 10 });
  assert.notEqual(zoom, null);
  const radius = (512 * 2 ** zoom) / (2 * Math.PI) / Math.cos((cap.center[0] * Math.PI) / 180);
  assert.ok(radius >= 250, `globe radius ${radius} px: never the old 30–40 px dot`);
  // About 46° to the horizon: even the largest border keeps the globe.
  assert.notEqual(globeZoomForCap(cap, { width: 1920, height: 960, padding: PADDING, borderDeg: 45 }), null);
});

test('globeZoomForCap: no zoom when even the minimum zoom shows too little', () => {
  // Near the pole the globe is drawn bigger, so the minimum zoom (-2) cannot shrink it enough.
  assert.equal(globeZoomForCap({ center: [85, 0], radiusDeg: 80 }, VIEW), null);
});

// --- feature 016 US2: the flat view centred on the QTH (T016, T017) ------------------------

import { flatFrameAround } from '../src/web/assets/autozoom-geo.js';

const MERC = (lat) => Math.log(Math.tan(Math.PI / 4 + (Math.max(-85.051129, Math.min(85.051129, lat)) * Math.PI) / 360));
const wrap = (d) => ((((d + 180) % 360) + 360) % 360) - 180;

/**
 * Whether every `[lat, lon]` lies in the free area (1 px tolerance) with the camera on `center`
 * at `zoom`, after MapLibre's Mercator clamp keeps the view inside ±85.05° (research R4). The
 * camera centre is drawn at the free area's centre.
 */
function fitsFlat(center, zoom, points, { width, height, padding }) {
  const world = 512 * 2 ** zoom;
  const freeW = width - padding.left - padding.right;
  const freeH = height - padding.top - padding.bottom;
  const toY = (lat) => ((Math.PI - MERC(lat)) / (2 * Math.PI)) * world;
  let cy = toY(center[1]);
  // MapLibre clamps the camera centre to half the map height from the ±85° edges, padding aside.
  if (world >= height) cy = Math.min(Math.max(cy, height / 2), world - height / 2);
  return points.every(([lat, lon]) => {
    const x = (wrap(lon - center[0]) / 360) * world;
    const y = toY(lat) - cy;
    return Math.abs(x) <= freeW / 2 + 1 && Math.abs(y) <= freeH / 2 + 1;
  });
}

const qthOf = ([lat, lon]) => ({ lat, lon });
const HD = { width: 1920, height: 1080, padding: PADDING };

test('flatFrameAround: the snapshot is centred on the QTH at the closest zoom that holds it', () => {
  const { center, zoom } = flatFrameAround(qthOf(SNAPSHOT.qth), SNAPSHOT.all, HD);
  assert.deepEqual(center, [SNAPSHOT.qth[1], SNAPSHOT.qth[0]]);
  assert.ok(fitsFlat(center, zoom, SNAPSHOT.all, HD), `zoom ${zoom}`);
  assert.ok(!fitsFlat(center, zoom + 0.05, SNAPSHOT.all, HD), `zoom ${zoom} is not the closest`);
  assert.ok(zoom > 1 && zoom < 2, `zoom ${zoom}`);
});

test('flatFrameAround: longitudes count the short way round', () => {
  const zl = { lat: -41.3, lon: 174.8 };
  const view = { width: 1280, height: 800, padding: { top: 10, right: 10, bottom: 10, left: 10 } };
  // A QSO at the QTH's latitude, so only the longitude sets the zoom.
  const w6 = [zl.lat, W6[1]];
  const { center, zoom } = flatFrameAround(zl, [[zl.lat, zl.lon], w6], view);
  assert.ok(fitsFlat(center, zoom, [w6], view));
  // 63° east across the antimeridian, not 297° west: the free width spans about 2 × 63°.
  const degreesAcross = (1260 / (512 * 2 ** zoom)) * 360;
  assert.ok(degreesAcross < 140, `${degreesAcross}° across`);
});

test('flatFrameAround: everything at the QTH stops at the maximum zoom', () => {
  assert.equal(flatFrameAround({ lat: 39, lon: 9 }, [[39, 9]], HD).zoom, 12);
  assert.equal(flatFrameAround({ lat: 39, lon: 9 }, [], { ...HD, maxZoom: 10 }).zoom, 10);
});

test('flatFrameAround: a QSO half the world away shows the full world width', () => {
  const { zoom } = flatFrameAround({ lat: 0, lon: 9 }, [[0, -171]], HD);
  const freeWidth = 1920 - PADDING.left - PADDING.right;
  assert.ok(Math.abs(512 * 2 ** zoom - freeWidth) < 1, `world ${512 * 2 ** zoom} px`);
});

test('flatFrameAround: near a pole every point stays in view after the clamp', () => {
  const points = [[80, 20], [-40, 30], [70, -10]];
  const { center, zoom } = flatFrameAround({ lat: 80, lon: 20 }, points, HD);
  assert.ok(Number.isFinite(zoom));
  assert.ok(fitsFlat(center, zoom, points, HD), `zoom ${zoom}`);
});

test('flatFrameAround: no free area still gives a finite frame', () => {
  const tiny = { width: 200, height: 100, padding: { top: 60, right: 120, bottom: 60, left: 120 } };
  const { center, zoom } = flatFrameAround({ lat: 39, lon: 9 }, [[50, 0]], tiny);
  assert.deepEqual(center, [9, 39]);
  assert.ok(Number.isFinite(zoom));
});

test('flatFrameAround: with no QSOs the fallback frame is centred on the QTH', () => {
  // Spec edge case "No QSOs displayed": the entity box corners, centred on the house.
  const qth = { lat: 39.2, lon: 9.1 };
  const [[w, s], [e, n]] = fallbackFrame(qth, { bounds: [8.13, 38.86, 9.83, 41.31] });
  const corners = [[s, w], [s, e], [n, w], [n, e]];
  const { center, zoom } = flatFrameAround(qth, corners, HD);
  assert.deepEqual(center, [9.1, 39.2]);
  assert.ok(fitsFlat(center, zoom, corners, HD) && zoom < 12, `zoom ${zoom}`);
});

test('flatFrameAround: a short map clamps the centre and still holds every point', () => {
  // Seen in the browser at 1024 × 768 with the box on the left: zoom about 0.81, so the map is
  // about 900 px tall and centring 39° N would show past 85° N.
  const view = { width: 1024, height: 768, padding: { top: 12, right: 88, bottom: 44, left: 262 } };
  const { center, zoom } = flatFrameAround(qthOf(SNAPSHOT.qth), SNAPSHOT.all, view);
  assert.ok(Math.abs(zoom - 0.81) < 0.05, `zoom ${zoom}`);
  assert.ok(fitsFlat(center, zoom, SNAPSHOT.all, view), `zoom ${zoom}`);
});

// --- feature 016 US3: the zoom is the closest that fits (T021, SC-005) ---------------------

const SIZES = [{ width: 1024, height: 768, padding: PADDING }, { width: 1920, height: 1080, padding: PADDING }];
const SETS = {
  snapshot: { qth: SNAPSHOT.qth, points: SNAPSHOT.all },
  europe: { qth: SNAPSHOT.qth, points: [SNAPSHOT.qth, ...SNAPSHOT.europe] },
  pacific: { qth: ZL, points: [ZL, KH6, D2] },
};

/** The highest zoom, in steps of 0.01 from `top` down, for which `ok(zoom)` holds. */
function bruteForce(ok, top = 12) {
  for (let z = top; z >= -2; z -= 0.01) if (ok(z)) return z;
  return null;
}

test('flatFrameAround is within 0.25 of a brute-force search (SC-005)', () => {
  for (const [name, { qth, points }] of Object.entries(SETS)) {
    for (const view of SIZES) {
      const { center, zoom } = flatFrameAround(qthOf(qth), points, view);
      const best = bruteForce((z) => fitsFlat(center, z, points, view));
      assert.ok(best !== null && Math.abs(zoom - best) <= 0.25, `${name} ${view.width}: ${zoom} vs ${best}`);
    }
  }
});

test('globeZoomForCap is within 0.25 of a brute-force free-area search (SC-005)', () => {
  for (const [name, { points }] of Object.entries(SETS)) {
    for (const view of SIZES) {
      const cap = boundingCap(points);
      const zoom = globeZoomForCap(cap, { ...view, borderDeg: 0 });
      if (zoom === null) continue; // flat: nothing to compare (the snapshot)
      const half = Math.min(view.width - PADDING.left - PADDING.right, view.height - PADDING.top - PADDING.bottom) / 2;
      const best = bruteForce((z) => {
        const edge = capScreenRadius(cap, z, view);
        return edge !== null && edge <= half;
      });
      assert.ok(Math.abs(zoom - best) <= 0.25, `${name} ${view.width}: ${zoom} vs ${best}`);
    }
  }
});
