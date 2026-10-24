import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildStyle } from '../src/web/assets/view.js';

test('a vector style base map is used as the style URL', () => {
  assert.equal(buildStyle({ kind: 'style', url: 'https://s/style.json', attribution: 'a' }), 'https://s/style.json');
});

import { hasWebGL, isBasemapError, shouldRender } from '../src/web/assets/view.js';

function canvas(contexts) {
  return () => ({
    getContext(kind) {
      if (contexts === 'throw') throw new Error('blocked');
      return contexts[kind] ?? null;
    },
  });
}

test('hasWebGL accepts webgl2 or webgl', () => {
  assert.equal(hasWebGL(canvas({ webgl2: {} })), true);
  assert.equal(hasWebGL(canvas({ webgl: {} })), true);
});

test('hasWebGL is false when no context is available', () => {
  assert.equal(hasWebGL(canvas({})), false);
  assert.equal(hasWebGL(canvas('throw')), false);
});

const OVERLAYS = ['qso-lines', 'qso-markers'];

test('isBasemapError tells base map errors from overlay errors', () => {
  assert.equal(isBasemapError({ sourceId: 'openmaptiles' }, OVERLAYS), true);
  assert.equal(isBasemapError({ error: new Error('style') }, OVERLAYS), true);
  assert.equal(isBasemapError({ sourceId: 'qso-markers' }, OVERLAYS), false);
  assert.equal(isBasemapError({ sourceId: 'qso-lines' }, OVERLAYS), false);
});

test('shouldRender needs a loaded map and a non-empty diff', () => {
  const empty = { add: [], update: [], remove: [] };
  assert.equal(shouldRender(false, { ...empty, add: [{}] }), false);
  assert.equal(shouldRender(true, empty), false);
  assert.equal(shouldRender(true, { ...empty, add: [{}] }), true);
  assert.equal(shouldRender(true, { ...empty, update: [{}] }), true);
  assert.equal(shouldRender(true, { ...empty, remove: ['k'] }), true);
});

test('a raster base map becomes a version-8 raster style', () => {
  assert.deepEqual(buildStyle({ kind: 'raster', url: 'https://t/{z}/{x}/{y}.png', attribution: 'a' }), {
    version: 8,
    sources: { basemap: { type: 'raster', tiles: ['https://t/{z}/{x}/{y}.png'], tileSize: 256 } },
    layers: [{ id: 'basemap', type: 'raster', source: 'basemap' }],
  });
});

import { loadProjection, saveProjection, nextProjection } from '../src/web/assets/view.js';

function memoryStorage(initial = {}) {
  const data = { ...initial };
  return {
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = String(v);
    },
    data,
  };
}

const throwingStorage = {
  getItem() {
    throw new Error('denied');
  },
  setItem() {
    throw new Error('denied');
  },
};

test('loadProjection defaults to globe', () => {
  assert.equal(loadProjection(memoryStorage()), 'globe');
  assert.equal(loadProjection(null), 'globe');
  assert.equal(loadProjection(throwingStorage), 'globe');
  assert.equal(loadProjection(memoryStorage({ 'lqm.projection': 'spherical' })), 'globe');
  assert.equal(loadProjection(memoryStorage({ 'lqm.projection': 'mercator' })), 'mercator');
});

test('saveProjection stores and swallows errors', () => {
  const storage = memoryStorage();
  saveProjection(storage, 'mercator');
  assert.equal(storage.data['lqm.projection'], 'mercator');
  assert.doesNotThrow(() => saveProjection(throwingStorage, 'globe'));
  assert.doesNotThrow(() => saveProjection(null, 'globe'));
});

test('nextProjection toggles globe and mercator', () => {
  assert.equal(nextProjection('globe'), 'mercator');
  assert.equal(nextProjection('mercator'), 'globe');
});

import { popupToClose } from '../src/web/assets/view.js';

test('popupToClose closes only a popup whose marker was removed', () => {
  const diff = { add: [], update: [{ key: 'u' }], remove: ['r'] };
  assert.equal(popupToClose(null, diff), false);
  assert.equal(popupToClose('r', diff), true);
  assert.equal(popupToClose('u', diff), false);
  assert.equal(popupToClose('x', diff), false);
});

import { loadAutoZoom, saveAutoZoom } from '../src/web/assets/view.js';


test('auto-zoom is on by default and its choice round-trips (FR-007)', () => {
  assert.equal(loadAutoZoom(memoryStorage()), true);
  assert.equal(loadAutoZoom(null), true);
  const storage = memoryStorage();
  saveAutoZoom(storage, false);
  assert.equal(storage.data['lqm.autozoom'], 'off');
  assert.equal(loadAutoZoom(storage), false);
  saveAutoZoom(storage, true);
  assert.equal(loadAutoZoom(storage), true);
  assert.equal(loadAutoZoom(memoryStorage({ 'lqm.autozoom': 'maybe' })), true);
});

test('auto-zoom preference survives a storage that throws', () => {
  assert.equal(loadAutoZoom(throwingStorage), true);
  assert.doesNotThrow(() => saveAutoZoom(throwingStorage, false));
});

import { loadRecent, saveRecent } from '../src/web/assets/view.js';

test('the last-QSO box is shown by default and its choice round-trips (FR-021)', () => {
  assert.equal(loadRecent(memoryStorage()), true);
  const storage = memoryStorage();
  saveRecent(storage, false);
  assert.equal(storage.data['lqm.recent'], 'hidden');
  assert.equal(loadRecent(storage), false);
  saveRecent(storage, true);
  assert.equal(loadRecent(storage), true);
  assert.equal(loadRecent(memoryStorage({ 'lqm.recent': 'maybe' })), true);
  assert.equal(loadRecent(throwingStorage), true);
  assert.doesNotThrow(() => saveRecent(throwingStorage, false));
});

import { scaleVisible } from '../src/web/assets/view.js';

test('the scale is shown only on the flat map (FR-024)', () => {
  assert.equal(scaleVisible('mercator'), true);
  assert.equal(scaleVisible('globe'), false);
});

import { loadRecentSize, saveRecentSize, loadRecentOrder, saveRecentOrder } from '../src/web/assets/view.js';

test('box size defaults to 5 and round-trips the allowed sizes (feature 006)', () => {
  assert.equal(loadRecentSize(memoryStorage()), 5);
  for (const size of [3, 5, 10, 15, 20]) {
    const storage = memoryStorage();
    saveRecentSize(storage, size);
    assert.equal(storage.data['lqm.recent.size'], String(size));
    assert.equal(loadRecentSize(storage), size);
  }
  assert.equal(loadRecentSize(memoryStorage({ 'lqm.recent.size': '7' })), 5);
  assert.equal(loadRecentSize(memoryStorage({ 'lqm.recent.size': 'many' })), 5);
  assert.equal(loadRecentSize(throwingStorage), 5);
  assert.doesNotThrow(() => saveRecentSize(throwingStorage, 10));
});

test('box order defaults to newest at the bottom and round-trips (feature 006)', () => {
  assert.equal(loadRecentOrder(memoryStorage()), 'bottom');
  const storage = memoryStorage();
  saveRecentOrder(storage, 'top');
  assert.equal(storage.data['lqm.recent.order'], 'top');
  assert.equal(loadRecentOrder(storage), 'top');
  assert.equal(loadRecentOrder(memoryStorage({ 'lqm.recent.order': 'sideways' })), 'bottom');
  assert.equal(loadRecentOrder(throwingStorage), 'bottom');
  assert.doesNotThrow(() => saveRecentOrder(throwingStorage, 'top'));
});

test('a box hidden before feature 007 stays hidden once the switch is a checkbox (Principle IV)', () => {
  // The value written by the feature-004 map control, under the same key the checkbox reads.
  const storage = memoryStorage({ 'lqm.recent': 'hidden' });
  assert.equal(loadRecent(storage), false);
  assert.equal(loadRecent(memoryStorage({ 'lqm.recent': 'shown' })), true);
});

// --- base map per theme (feature 011, T010) -----------------------------------------------

import { basemapFor } from '../src/web/assets/view.js';

const DEFAULT_URL = 'https://tiles.openfreemap.org/styles/liberty';

test('the default base map switches to the theme style, undimmed', () => {
  const basemap = { kind: 'style', url: DEFAULT_URL };
  assert.deepEqual(basemapFor(basemap, { basemapStyle: 'dark', dimOpacity: 0.55 }, DEFAULT_URL), {
    styleUrl: 'https://tiles.openfreemap.org/styles/dark',
    dimOpacity: 0,
  });
  assert.deepEqual(basemapFor(basemap, { basemapStyle: 'liberty', dimOpacity: 0 }, DEFAULT_URL), {
    styleUrl: DEFAULT_URL,
    dimOpacity: 0,
  });
});

test('an operator base map keeps its style and is dimmed', () => {
  const palette = { basemapStyle: 'fiord', dimOpacity: 0.35 };
  assert.deepEqual(basemapFor({ kind: 'style', url: 'https://example.org/style.json' }, palette, DEFAULT_URL), {
    styleUrl: null,
    dimOpacity: 0.35,
  });
  assert.deepEqual(basemapFor({ kind: 'raster', url: 'https://t/{z}/{x}/{y}.png' }, palette, DEFAULT_URL), {
    styleUrl: null,
    dimOpacity: 0.35,
  });
});

// --- atmosphere (feature 011, User Story 3, T016) -----------------------------------------

import { loadAtmosphere, saveAtmosphere, skyFor } from '../src/web/assets/view.js';

function atmosphereStore(initial = {}) {
  const data = { ...initial };
  return {
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = String(v);
    },
    data,
  };
}

test('the atmosphere is on by default and its choice is saved', () => {
  assert.equal(loadAtmosphere(atmosphereStore()), true);
  const store = atmosphereStore();
  saveAtmosphere(store, false);
  assert.equal(store.data['lqm.atmosphere'], 'off');
  assert.equal(loadAtmosphere(store), false);
  saveAtmosphere(store, true);
  assert.equal(store.data['lqm.atmosphere'], 'on');
  assert.equal(loadAtmosphere(atmosphereStore({ 'lqm.atmosphere': 'maybe' })), true, 'unknown value');
  assert.equal(loadAtmosphere(null), true);
  const throwing = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };
  assert.equal(loadAtmosphere(throwing), true);
  assert.doesNotThrow(() => saveAtmosphere(throwing, false));
});

test('the sky shows the atmosphere only on the globe, with the theme colours', () => {
  const palette = { sky: '#0b1426', horizon: '#1b2a4a', fog: '#0e1a30' };
  const globe = skyFor(palette, { atmosphere: true, projection: 'globe' });
  assert.equal(globe['atmosphere-blend'], 1);
  assert.equal(globe['sky-color'], '#0b1426');
  assert.equal(globe['horizon-color'], '#1b2a4a');
  assert.equal(globe['fog-color'], '#0e1a30');
  assert.equal(skyFor(palette, { atmosphere: false, projection: 'globe' })['atmosphere-blend'], 0);
  assert.equal(skyFor(palette, { atmosphere: true, projection: 'mercator' })['atmosphere-blend'], 0, 'flat map');
});

// --- the glow follows the Earth (feature 013, research R1–R2, T004) -----------------------

import { lightFor } from '../src/web/assets/view.js';

// Copied from maplibre-gl 5.24.0 dist/maplibre-gl-dev.js, getSunPos/sphericalToCartesian.
// The rotations are gl-matrix's (right-handed, applied right to left). Re-check on upgrade.
function sphericalToCartesian([r, azimuthal, polar]) {
  azimuthal += 90;
  azimuthal *= Math.PI / 180;
  polar *= Math.PI / 180;
  return { x: r * Math.cos(azimuthal) * Math.sin(polar), y: r * Math.sin(azimuthal) * Math.sin(polar), z: r * Math.cos(polar) };
}
const rotX = (t, [x, y, z]) => [x, y * Math.cos(t) - z * Math.sin(t), y * Math.sin(t) + z * Math.cos(t)];
const rotY = (t, [x, y, z]) => [x * Math.cos(t) + z * Math.sin(t), y, -x * Math.sin(t) + z * Math.cos(t)];
const rotZ = (t, [x, y, z]) => [x * Math.cos(t) - y * Math.sin(t), x * Math.sin(t) + y * Math.cos(t), z];
function getSunPos(light, view) {
  const lp = sphericalToCartesian(light.position);
  let v = [-lp.x, -lp.y, -lp.z];
  if (light.anchor === 'map') {
    const D = Math.PI / 180;
    // lightMat = Rz(roll)·Rx(−pitch)·Rz(bearing)·Rx(lat)·Ry(−lng); applied to v right to left.
    v = rotY(-view.lng * D, v);
    v = rotX(view.lat * D, v);
    v = rotZ((view.bearing ?? 0) * D, v);
    v = rotX(-(view.pitch ?? 0) * D, v);
  }
  const n = Math.hypot(...v);
  return v.map((c) => c / n);
}

// The Sun's real direction in camera space: an Earth-fixed light, replayed the same way.
const D = Math.PI / 180;
function trueSun({ lat, lon }, camera) {
  const earth = [Math.cos(lat * D) * Math.sin(lon * D), Math.sin(lat * D), Math.cos(lat * D) * Math.cos(lon * D)];
  let v = rotY(-camera.lng * D, earth);
  v = rotX(camera.lat * D, v);
  v = rotZ((camera.bearing ?? 0) * D, v);
  return rotX(-(camera.pitch ?? 0) * D, v);
}

const cameras = [];
for (let lat = -60; lat <= 60; lat += 30) {
  for (let lng = -180; lng < 180; lng += 45) {
    for (const bearing of [0, 37, 200]) cameras.push({ lat, lng, bearing, pitch: (lat + 60) / 4 });
  }
}
const SUBSOLAR = [[0, 0], [-8.5, -30], [23.44, 120], [-23.44, -170], [10, 179]];

test('the light is a viewport light at a spherical position', () => {
  const light = lightFor({ lat: 10, lon: 20 }, { lat: 0, lng: 0, bearing: 0, pitch: 0 });
  assert.equal(light.anchor, 'viewport');
  assert.equal(light.position.length, 3);
  assert.equal(light.position[0], 1.15);
  assert.ok(light.position[2] >= 0 && light.position[2] <= 180, 'polar angle');
  assert.ok(light.position[1] >= -180 && light.position[1] <= 180, 'azimuthal angle');
});

test('the globe is always lit from behind: no haze over the map, whatever the view', () => {
  for (const [lat, lon] of SUBSOLAR) {
    for (const camera of cameras) {
      const [, , z] = getSunPos(lightFor({ lat, lon }, camera), camera);
      assert.ok(z <= -0.9, `${lat},${lon} from ${JSON.stringify(camera)}: z ${z}`);
    }
  }
});

test('the rim is brightest towards the Sun, as the Sun lies on screen', () => {
  for (const [lat, lon] of SUBSOLAR) {
    for (const camera of cameras) {
      const sun = trueSun({ lat, lon }, camera);
      const onScreen = Math.hypot(sun[0], sun[1]);
      if (onScreen < 0.3) continue; // the Sun nearly in front or behind: an even rim
      const [x, y] = getSunPos(lightFor({ lat, lon }, camera), camera);
      const cos = (x * sun[0] + y * sun[1]) / (Math.hypot(x, y) * onScreen);
      assert.ok(cos > Math.cos(Math.PI / 180), `${lat},${lon} from ${JSON.stringify(camera)}: cos ${cos}`);
    }
  }
});

test('the Sun 90° west of the view centre puts the bright rim on the left', () => {
  const [x, y] = getSunPos(lightFor({ lat: 0, lon: 0 }, { lat: 0, lng: 90, bearing: 0, pitch: 0 }), {});
  assert.ok(x < -0.3 && Math.abs(y) < 1e-9, `${x},${y}`);
});

test('the rim is even when the Sun is straight in front or straight behind', () => {
  for (const lng of [0, 180]) {
    const [x, y, z] = getSunPos(lightFor({ lat: 0, lon: 0 }, { lat: 0, lng, bearing: 0, pitch: 0 }), {});
    assert.ok(Math.abs(x) < 1e-9 && Math.abs(y) < 1e-9 && Math.abs(z + 1) < 1e-9, `lng ${lng}: ${x},${y},${z}`);
  }
});

test('regression A1/A2: the old fixed light ignores where the globe points', () => {
  const light = { anchor: 'viewport', position: [1.15, 210, 30] };
  assert.deepEqual(getSunPos(light, { lat: 0, lng: 0 }), getSunPos(light, { lat: 0, lng: 180 }));
  const a = lightFor({ lat: 0, lon: 0 }, { lat: 0, lng: 90, bearing: 0, pitch: 0 });
  const b = lightFor({ lat: 0, lon: 0 }, { lat: 0, lng: -90, bearing: 0, pitch: 0 });
  assert.notDeepEqual(a.position, b.position, 'the new light turns with the globe');
});

// --- Grayline and Sun position choices (feature 013, research R11, T017) ------------------

import { loadGrayline, saveGrayline, loadSunMarker, saveSunMarker } from '../src/web/assets/view.js';

test('the grayline is on by default and its choice is saved', () => {
  assert.equal(loadGrayline(atmosphereStore()), true);
  const store = atmosphereStore();
  saveGrayline(store, false);
  assert.equal(store.data['lqm.grayline'], 'off');
  assert.equal(loadGrayline(store), false);
  saveGrayline(store, true);
  assert.equal(store.data['lqm.grayline'], 'on');
  assert.equal(loadGrayline(atmosphereStore({ 'lqm.grayline': 'maybe' })), true, 'unknown value');
});

test('the Sun position is off by default and its choice is saved', () => {
  assert.equal(loadSunMarker(atmosphereStore()), false);
  const store = atmosphereStore();
  saveSunMarker(store, true);
  assert.equal(store.data['lqm.sun'], 'on');
  assert.equal(loadSunMarker(store), true);
  saveSunMarker(store, false);
  assert.equal(store.data['lqm.sun'], 'off');
  assert.equal(loadSunMarker(atmosphereStore({ 'lqm.sun': 'maybe' })), false, 'unknown value');
});

test('a browser that refuses storage keeps the defaults', () => {
  const throwing = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };
  assert.equal(loadGrayline(throwing), true);
  assert.equal(loadSunMarker(throwing), false);
  assert.doesNotThrow(() => saveGrayline(throwing, false));
  assert.doesNotThrow(() => saveSunMarker(throwing, true));
});

import { globeBorder } from '../src/web/assets/view.js';

// Feature 016 (T009): the operator's globe border from hello, 10° when absent or invalid.
test('globeBorder takes a valid hello value, else 10', () => {
  assert.equal(globeBorder({ globe_border_deg: 0 }), 0);
  assert.equal(globeBorder({ globe_border_deg: 25.5 }), 25.5);
  assert.equal(globeBorder({ globe_border_deg: 45 }), 45);
  for (const bad of [undefined, '20', Number.NaN, Number.POSITIVE_INFINITY, -1, 46, null]) {
    assert.equal(globeBorder({ globe_border_deg: bad }), 10, String(bad));
  }
  assert.equal(globeBorder({}), 10);
  assert.equal(globeBorder(null), 10);
});
