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

test('globe preference: flat when the frame does not fit, globe again when it does', () => {
  const az = createAutoZoom({ enabled: true });
  assert.equal(az.fitted(false, 'globe'), 'mercator');
  assert.equal(az.state.autoFlat, true);
  assert.equal(az.fitted(true, 'globe'), 'globe');
  assert.equal(az.state.autoFlat, false);
});

test('flat preference: auto-zoom never switches to the globe (FR-017)', () => {
  const az = createAutoZoom({ enabled: true });
  assert.equal(az.fitted(true, 'mercator'), 'mercator');
  assert.equal(az.fitted(false, 'mercator'), 'mercator');
  assert.equal(az.state.autoFlat, false);
});

test('turning auto-zoom off keeps an automatic flat view (FR-017)', () => {
  const az = createAutoZoom({ enabled: true });
  az.fitted(false, 'globe');
  assert.deepEqual(kinds(az.toggle(0)), ['save']);
  assert.equal(az.state.autoFlat, true);

  const moved = createAutoZoom({ enabled: true });
  moved.fitted(false, 'globe');
  assert.deepEqual(kinds(moved.userMove()), ['save']);
  assert.equal(moved.state.autoFlat, true);
});

test('turning auto-zoom back on may return to the globe', () => {
  const az = createAutoZoom({ enabled: true });
  az.fitted(false, 'globe');
  az.toggle(0);
  assert.deepEqual(kinds(az.toggle(5000)), ['save', 'fit']);
  assert.equal(az.fitted(true, 'globe'), 'globe');
});

test('the projection button clears the automatic flat view', () => {
  const az = createAutoZoom({ enabled: true });
  az.fitted(false, 'globe');
  az.projectionChosen('mercator', false);
  assert.equal(az.state.autoFlat, false);
});

test('no action ever saves the projection (FR-018)', () => {
  const az = createAutoZoom({ enabled: true });
  const all = [
    ...az.toggle(0),
    ...az.toggle(1),
    ...az.change(),
    ...az.tick(5000),
    ...az.userMove(),
    ...az.popupOpened(),
    ...az.popupClosed(),
  ];
  assert.ok(all.every((a) => a.type === 'save' ? 'enabled' in a && !('projection' in a) : true));
});

test('a scripted run of 50 changes switches only by the rule (SC-006)', () => {
  const az = createAutoZoom({ enabled: true });
  let seed = 3;
  const random = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < 50; i++) {
    const preference = random() < 0.8 ? 'globe' : 'mercator';
    const fits = random() < 0.5;
    const shown = az.fitted(fits, preference);
    const expected = preference === 'globe' && fits ? 'globe' : 'mercator';
    assert.equal(shown, expected, `step ${i}`);
    assert.equal(az.state.autoFlat, preference === 'globe' && !fits);
  }
});

test('any projection choice switches auto-zoom off (owner, 2026-10-09)', () => {
  for (const [target, fits] of [['globe', false], ['globe', true], ['mercator', false], ['mercator', true]]) {
    const az = createAutoZoom({ enabled: true });
    az.fitted(false, 'globe');
    assert.deepEqual(az.projectionChosen(target, fits), [{ type: 'save', enabled: false }], `${target}`);
    assert.equal(az.state.enabled, false);
    assert.equal(az.state.autoFlat, false);
    az.change();
    assert.deepEqual(az.tick(10_000), [], 'no refit takes the choice back');
  }
});

test('with auto-zoom off, choosing a projection changes nothing else', () => {
  const az = createAutoZoom({ enabled: false });
  assert.deepEqual(az.projectionChosen('globe', false), []);
  assert.equal(az.state.enabled, false);
});
