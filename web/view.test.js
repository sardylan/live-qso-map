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
