import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createPanel, toggle, close, activity, tick, AUTO_CLOSE_MS } from '../src/web/assets/panel.js';

const MINUTE = 60_000;

test('the panel starts closed and auto-closes after five minutes', () => {
  assert.equal(AUTO_CLOSE_MS, 5 * MINUTE);
  const panel = createPanel();
  assert.equal(panel.open, false);
});

test('the gear toggles the panel; closing asks for focus on the gear', () => {
  const panel = createPanel();
  assert.deepEqual(toggle(panel, 0), { open: true, focusGear: false });
  assert.equal(panel.open, true);
  assert.deepEqual(toggle(panel, 1), { open: false, focusGear: true });
  assert.equal(panel.open, false);
});

test('close (the ✕ button or Escape) closes an open panel only', () => {
  const panel = createPanel();
  assert.deepEqual(close(panel), { open: false, focusGear: false }, 'already closed: nothing to do');
  toggle(panel, 0);
  assert.deepEqual(close(panel), { open: false, focusGear: true });
});

test('five minutes without activity close it; activity restarts the count', () => {
  const panel = createPanel();
  toggle(panel, 0);
  assert.equal(tick(panel, 5 * MINUTE - 1000), null, 'not at 4 m 59 s');
  activity(panel, 4 * MINUTE);
  assert.equal(tick(panel, 5 * MINUTE + 1000), null, 'restarted at 4 minutes');
  assert.equal(tick(panel, 9 * MINUTE - 1), null);
  assert.deepEqual(tick(panel, 9 * MINUTE), { open: false, focusGear: true });
  assert.equal(panel.open, false);
});

test('a closed panel is untouched by ticks and activity', () => {
  const panel = createPanel();
  assert.equal(tick(panel, 60 * MINUTE), null);
  activity(panel, 10);
  assert.equal(panel.open, false);
});

test('opening restarts the count', () => {
  const panel = createPanel();
  toggle(panel, 0);
  toggle(panel, 1);
  toggle(panel, 10 * MINUTE);
  assert.equal(tick(panel, 14 * MINUTE), null, 'opened 4 minutes ago');
});
