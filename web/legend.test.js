import { test } from 'node:test';
import assert from 'node:assert/strict';
import { offeredGroups, chosenBands, loadLegend, saveLegend, toggleGroup } from '../src/web/assets/legend.js';

const GROUPS = [
  { name: 'hf', label: 'HF', bands: ['160m', '80m', '40m', '30m', '20m', '17m', '15m', '12m', '10m'] },
  { name: 'hf-contest', label: 'HF contest', bands: ['160m', '80m', '40m', '20m', '15m', '10m'] },
  { name: 'warc', label: 'WARC', bands: ['30m'] },
  { name: 'vhf', label: 'VHF', bands: ['6m', '2m'] },
];
const ALL = ['160m', '80m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m', '2m'];

function memory(initial = {}) {
  const data = { ...initial };
  return {
    data,
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = String(v);
    },
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

test('offered groups keep the server order and say how many bands each holds', () => {
  const offered = offeredGroups(GROUPS);
  assert.deepEqual(
    offered.map((g) => [g.name, g.text]),
    [
      ['hf', 'HF (9 bands)'],
      ['hf-contest', 'HF contest (6 bands)'],
      ['warc', 'WARC (1 band)'],
      ['vhf', 'VHF (2 bands)'],
    ],
  );
  assert.deepEqual(offered[2].bands, ['30m']);
  assert.deepEqual(offeredGroups(undefined), [], 'an older server sends no groups');
});

test('chosen bands are the union of the groups in frequency order, or null for all', () => {
  assert.equal(chosenBands('all', GROUPS, ALL), null);
  assert.deepEqual(chosenBands(['vhf', 'warc'], GROUPS, ALL), ['30m', '6m', '2m']);
  assert.deepEqual(chosenBands(['hf-contest', 'warc'], GROUPS, ALL), ['160m', '80m', '40m', '30m', '20m', '15m', '10m']);
  assert.deepEqual(chosenBands(['hf', 'hf-contest'], GROUPS, ALL), GROUPS[0].bands, 'no duplicates');
  assert.equal(chosenBands(['gone'], GROUPS, ALL), null, 'nothing left: every band');
});

test('the saved legend choice drops unknown names and falls back to all', () => {
  const names = GROUPS.map((g) => g.name);
  assert.equal(loadLegend(memory(), names), 'all');
  assert.equal(loadLegend(memory({ 'lqm.legend': 'all' }), names), 'all');
  assert.deepEqual(loadLegend(memory({ 'lqm.legend': '["warc","vhf"]' }), names), ['warc', 'vhf']);
  assert.deepEqual(loadLegend(memory({ 'lqm.legend': '["warc","uhf"]' }), names), ['warc']);
  assert.equal(loadLegend(memory({ 'lqm.legend': '["uhf"]' }), names), 'all', 'stale');
  assert.equal(loadLegend(memory({ 'lqm.legend': '[]' }), names), 'all', 'empty');
  assert.equal(loadLegend(memory({ 'lqm.legend': '{bad' }), names), 'all');
  assert.equal(loadLegend(memory({ 'lqm.legend': '"hf"' }), names), 'all', 'not an array');
  assert.equal(loadLegend(throwing, names), 'all');
  assert.equal(loadLegend(null, names), 'all');
});

test('the legend choice is saved as all or a JSON array', () => {
  const storage = memory();
  saveLegend(storage, ['warc']);
  assert.equal(storage.data['lqm.legend'], '["warc"]');
  saveLegend(storage, 'all');
  assert.equal(storage.data['lqm.legend'], 'all');
  assert.doesNotThrow(() => saveLegend(throwing, ['hf']));
});

test('"all" is exclusive: a group leaves it, the last group unticked returns to it', () => {
  assert.deepEqual(toggleGroup('all', 'warc'), ['warc']);
  assert.deepEqual(toggleGroup(['warc'], 'vhf'), ['warc', 'vhf']);
  assert.deepEqual(toggleGroup(['warc', 'vhf'], 'warc'), ['vhf']);
  assert.equal(toggleGroup(['vhf'], 'vhf'), 'all');
  assert.equal(toggleGroup(['vhf'], 'all'), 'all', 'ticking all clears the groups');
  assert.equal(toggleGroup('all', 'all'), 'all', 'all cannot be unticked alone');
});
