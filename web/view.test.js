import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildStyle } from '../src/web/assets/view.js';

test('a vector style base map is used as the style URL', () => {
  assert.equal(buildStyle({ kind: 'style', url: 'https://s/style.json', attribution: 'a' }), 'https://s/style.json');
});
