import { test } from 'node:test';
import assert from 'node:assert/strict';
import { groupDigits, scoreText, isStale, updatedTitle } from '../src/web/assets/score.js';

test('groupDigits uses a dot every three digits', () => {
  assert.equal(groupDigits(0), '0');
  assert.equal(groupDigits(999), '999');
  assert.equal(groupDigits(1000), '1.000');
  assert.equal(groupDigits(1234567), '1.234.567');
  assert.equal(groupDigits(12345678), '12.345.678');
});

test('groupDigits rejects anything but a non-negative integer', () => {
  for (const bad of [-1, 1.5, Number.NaN, '12', null, undefined]) {
    assert.equal(groupDigits(bad), '?', String(bad));
  }
});

test('scoreText shows a dash before the first score', () => {
  assert.equal(scoreText(null), 'Score —');
});

// One line each for score, QSOs and multipliers (feature 018, owner's request after the visual test).
test('scoreText formats score, QSOs and multipliers on three lines', () => {
  assert.equal(scoreText({ score: 1234567, qsos: 1523, mults: 183 }), '1.234.567 pts\n1.523 QSO\n183 mults');
  assert.equal(scoreText({ score: 0, qsos: 0, mults: 0 }), '0 pts\n0 QSO\n0 mults');
  assert.equal(scoreText({ score: 12345678, qsos: 9876, mults: 1234 }), '12.345.678 pts\n9.876 QSO\n1.234 mults');
});

test('isStale only after more than five minutes', () => {
  const received = 1_000_000;
  assert.equal(isStale(received, received), false);
  assert.equal(isStale(received, received + 300_000), false);
  assert.equal(isStale(received, received + 300_001), true);
  assert.equal(isStale(received, received + 1_001, 1_000), true);
  assert.equal(isStale(received, received + 1_000, 1_000), false);
});

test('updatedTitle shows the UTC time of the last update', () => {
  assert.equal(updatedTitle('2026-10-08T15:04:29Z'), 'Last updated 15:04Z');
  for (const bad of ['', null, undefined, 'yesterday']) {
    assert.equal(updatedTitle(bad), '', String(bad));
  }
});
