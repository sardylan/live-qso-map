import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createAutoZoom } from '../src/web/assets/autozoom.js';

const kinds = (actions) => actions.map((a) => a.type);

test('switching on fits at once and saves; switching off only saves', () => {
  const az = createAutoZoom({ enabled: false });
  assert.deepEqual(az.toggle(1000), [{ type: 'save', enabled: true }, { type: 'fit' }]);
  assert.equal(az.state.enabled, true);
  assert.deepEqual(az.toggle(1100), [{ type: 'save', enabled: false }]);
  az.change();
  assert.deepEqual(az.tick(5000), []);
});

test('a manual move switches auto-zoom off and no refit follows (SC-004)', () => {
  const az = createAutoZoom({ enabled: true });
  az.change();
  assert.deepEqual(az.userMove(), [{ type: 'save', enabled: false }]);
  assert.equal(az.state.enabled, false);
  for (let t = 0; t < 10_000; t += 250) {
    az.change();
    assert.deepEqual(az.tick(t), []);
  }
  assert.deepEqual(az.userMove(), [], 'a second move while off does nothing');
});

test('a burst of changes gives one fit per second at most (FR-014, SC-005)', () => {
  const az = createAutoZoom({ enabled: true });
  let fits = [];
  for (let t = 0; t < 1000; t += 100) {
    az.change();
    if (kinds(az.tick(t)).includes('fit')) fits.push(t);
  }
  assert.deepEqual(fits, [0]);
  fits = [];
  for (let t = 1000; t < 3000; t += 100) {
    az.change();
    if (kinds(az.tick(t)).includes('fit')) fits.push(t);
  }
  assert.deepEqual(fits, [1000, 2000]);
});

test('without a change, ticks never fit', () => {
  const az = createAutoZoom({ enabled: true });
  assert.deepEqual(az.tick(0), []);
  assert.deepEqual(az.tick(5000), []);
});

test('refits pause while a popup is open, with one catch-up when it closes (FR-012a)', () => {
  const az = createAutoZoom({ enabled: true });
  az.popupOpened();
  for (let t = 0; t < 5000; t += 250) {
    az.change();
    assert.deepEqual(az.tick(t), []);
  }
  az.popupClosed();
  assert.deepEqual(kinds(az.tick(5000)), ['fit']);
  assert.deepEqual(az.tick(7000), []);
});

test('the catch-up refit runs even when nothing changed while the popup was open', () => {
  const az = createAutoZoom({ enabled: true });
  az.popupOpened();
  az.popupClosed();
  assert.deepEqual(kinds(az.tick(0)), ['fit']);
});

test('closing a popup while auto-zoom is off does not fit', () => {
  const az = createAutoZoom({ enabled: false });
  az.popupOpened();
  az.popupClosed();
  assert.deepEqual(az.tick(0), []);
});

test('page-made camera moves are never fed to the controller, so they never switch it off', () => {
  const az = createAutoZoom({ enabled: true });
  az.change();
  assert.deepEqual(kinds(az.tick(0)), ['fit']);
  // The fit animation moves the camera, but only user moves call userMove().
  assert.equal(az.state.enabled, true);
});
