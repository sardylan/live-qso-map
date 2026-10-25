import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  createConnection,
  backoffDelay,
  onOpen,
  onClose,
  onHello,
  loadLimit,
  saveLimit,
  LIMITS,
  shouldReload,
  pageVersion,
  RELOAD_KEY,
} from '../src/web/assets/connection.js';

test('backoff doubles from 1 s and caps at 30 s', () => {
  const noJitter = () => 0.5;
  const delays = [0, 1, 2, 3, 4, 5, 6].map((attempt) => backoffDelay(attempt, noJitter));
  assert.deepEqual(delays, [1000, 2000, 4000, 8000, 16000, 30000, 30000]);
});

test('jitter stays within ±20%', () => {
  assert.equal(backoffDelay(0, () => 0), 800);
  assert.equal(backoffDelay(0, () => 1), 1200);
  assert.equal(backoffDelay(10, () => 1), 36000);
});

test('attempts grow on close and reset on open', () => {
  const c = createConnection(20);
  assert.equal(onClose(c, () => 0.5), 1000);
  assert.equal(onClose(c, () => 0.5), 2000);
  assert.equal(onClose(c, () => 0.5), 4000);
  onOpen(c);
  assert.equal(onClose(c, () => 0.5), 1000);
});

test('hello triggers set_limit with the current selection', () => {
  const c = createConnection(100);
  assert.deepEqual(onHello(c), [{ type: 'set_limit', limit: 100 }]);
  c.limit = 1000;
  assert.deepEqual(onHello(c), [{ type: 'set_limit', limit: 1000 }]);
});

test('hello re-sends the chosen bands, and null for every band (feature 011)', () => {
  const c = createConnection(20);
  c.bands = ['30m'];
  assert.deepEqual(onHello(c), [{ type: 'set_limit', limit: 20, bands: ['30m'] }]);
  c.bands = null;
  assert.deepEqual(onHello(c), [{ type: 'set_limit', limit: 20, bands: null }]);
});

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

const throwing = {
  getItem() {
    throw new Error('denied');
  },
  setItem() {
    throw new Error('denied');
  },
};

test('loadLimit falls back to 20', () => {
  assert.equal(loadLimit(memoryStorage()), 20);
  assert.equal(loadLimit(throwing), 20);
  assert.equal(loadLimit(memoryStorage({ 'lqm.limit': '7' })), 20);
  assert.equal(loadLimit(memoryStorage({ 'lqm.limit': 'abc' })), 20);
  assert.equal(loadLimit(null), 20);
});

test('loadLimit returns a stored allowed value', () => {
  assert.equal(loadLimit(memoryStorage({ 'lqm.limit': '200' })), 200);
  assert.equal(loadLimit(memoryStorage({ 'lqm.limit': '1000' })), 1000);
});

test('the menu tops out at 1000, with no All (feature 014)', () => {
  assert.deepEqual(LIMITS, [10, 20, 50, 100, 200, 500, 1000]);
});

test('a saved "all" is read as 1000 and saved back as 1000 (feature 014, Principle IV)', () => {
  const storage = memoryStorage({ 'lqm.limit': 'all' });
  assert.equal(loadLimit(storage), 1000);
  assert.equal(storage.data['lqm.limit'], '1000');
  const readOnly = { getItem: () => 'all', setItem() { throw new Error('denied'); } };
  assert.equal(loadLimit(readOnly), 1000);
});

test('saveLimit stores and swallows storage errors', () => {
  const storage = memoryStorage();
  saveLimit(storage, 50);
  assert.equal(storage.data['lqm.limit'], '50');
  saveLimit(storage, 1000);
  assert.equal(storage.data['lqm.limit'], '1000');
  assert.doesNotThrow(() => saveLimit(throwing, 10));
  assert.doesNotThrow(() => saveLimit(null, 10));
});

test('the map limit behaves the same in any storage (moved to local storage, Principle IV)', () => {
  for (const storage of [memoryStorage(), memoryStorage()]) {
    assert.equal(loadLimit(storage), 20);
    saveLimit(storage, 50);
    assert.equal(loadLimit(storage), 50);
    saveLimit(storage, 1000);
    assert.equal(loadLimit(storage), 1000);
  }
  assert.equal(loadLimit(throwing), 20);
});

// Feature 018, research R6 and data-model "Reload decision".
test('shouldReload only when the page and the service versions differ', () => {
  assert.equal(shouldReload('3.3.0', '3.3.0', null), false);
  assert.equal(shouldReload('3.3.0', '3.4.0', null), true);
  assert.equal(shouldReload('3.3.0', '3.4.0', '3.2.0'), true);
});

test('shouldReload never twice for the same service version', () => {
  assert.equal(shouldReload('3.3.0', '3.4.0', '3.4.0'), false);
});

test('shouldReload never without both versions', () => {
  for (const missing of [undefined, null, '']) {
    assert.equal(shouldReload('3.3.0', missing, null), false);
    assert.equal(shouldReload(missing, '3.4.0', null), false);
  }
  assert.equal(shouldReload('__LQM_VERSION__', '3.4.0', null), false);
});

test('pageVersion reads the version meta', () => {
  const doc = (content) => ({
    querySelector: (selector) =>
      selector === 'meta[name="lqm-version"]' && content !== undefined ? { content } : null,
  });
  assert.equal(pageVersion(doc('3.3.0')), '3.3.0');
  assert.equal(pageVersion(doc(undefined)), null);
  assert.equal(pageVersion(doc('')), null);
  assert.equal(pageVersion(doc('__LQM_VERSION__')), null);
});

test('the reload flag key', () => {
  assert.equal(RELOAD_KEY, 'lqm.reloaded-for');
});
