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
