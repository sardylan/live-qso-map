import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createConnection, backoffDelay, onOpen, onClose, onHello, loadLimit, saveLimit } from '../src/web/assets/connection.js';

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
  c.limit = null;
  assert.deepEqual(onHello(c), [{ type: 'set_limit', limit: null }]);
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
  assert.equal(loadLimit(memoryStorage({ 'lqm.limit': 'all' })), null);
});

test('saveLimit stores and swallows storage errors', () => {
  const storage = memoryStorage();
  saveLimit(storage, 50);
  assert.equal(storage.data['lqm.limit'], '50');
  saveLimit(storage, null);
  assert.equal(storage.data['lqm.limit'], 'all');
  assert.doesNotThrow(() => saveLimit(throwing, 10));
  assert.doesNotThrow(() => saveLimit(null, 10));
});

test('the map limit behaves the same in any storage (moved to local storage, Principle IV)', () => {
  for (const storage of [memoryStorage(), memoryStorage()]) {
    assert.equal(loadLimit(storage), 20);
    saveLimit(storage, 50);
    assert.equal(loadLimit(storage), 50);
    saveLimit(storage, null);
    assert.equal(loadLimit(storage), null);
  }
  assert.equal(loadLimit(throwing), 20);
});
