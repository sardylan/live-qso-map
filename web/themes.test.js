import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PALETTES, contrastRatio } from '../src/web/assets/themes.js';

// The CSS variables app.css reads (feature 011, T001), as palette keys.
const CSS_KEYS = [
  'bg', 'panel', 'text', 'muted', 'border', 'accent', 'highlight', 'banner', 'bannerLow',
  'bannerText', 'noticeBg', 'noticeText', 'shadow', 'space',
];

test('there are three palettes with every key the page needs', () => {
  assert.deepEqual(Object.keys(PALETTES), ['light', 'dim', 'dark']);
  for (const [name, palette] of Object.entries(PALETTES)) {
    for (const key of [...CSS_KEYS, 'sky', 'horizon', 'fog', 'line', 'basemapStyle', 'dimOpacity']) {
      assert.ok(palette[key] !== undefined, `${name}.${key}`);
    }
  }
  assert.deepEqual(
    Object.values(PALETTES).map((p) => [p.basemapStyle, p.dimOpacity]),
    [['liberty', 0], ['fiord', 0.35], ['dark', 0.55]],
  );
});

test('contrast ratio follows WCAG relative luminance', () => {
  assert.equal(Math.round(contrastRatio('#000000', '#ffffff') * 100) / 100, 21);
  assert.equal(contrastRatio('#808080', '#808080'), 1);
  assert.equal(contrastRatio('#fff', '#000'), contrastRatio('#000000', '#ffffff'), 'short hex');
  assert.equal(contrastRatio('#ffffffeb', '#000000'), contrastRatio('#ffffff', '#000000'), 'alpha ignored');
});

test('every text colour reaches 4.5:1 on its background, in every theme (SC-002)', () => {
  for (const [name, p] of Object.entries(PALETTES)) {
    for (const [fg, bg] of [
      ['text', 'bg'],
      ['text', 'panel'],
      ['text', 'highlight'],
      ['muted', 'panel'],
      ['muted', 'bg'],
      ['accent', 'panel'],
      ['bannerText', 'banner'],
      ['bannerText', 'bannerLow'],
      ['noticeText', 'noticeBg'],
    ]) {
      const ratio = contrastRatio(p[fg], p[bg]);
      assert.ok(ratio >= 4.5, `${name}: ${fg} on ${bg} is ${ratio.toFixed(2)}:1`);
    }
  }
});

// --- choosing the theme (User Story 1, T010) ----------------------------------------------

import { resolveTheme, loadTheme, saveTheme, themeStatus, THEME_CHOICES } from '../src/web/assets/themes.js';

test('automatic, device: follows the device preference', () => {
  assert.equal(resolveTheme('auto-device', { prefersDark: false, sunElevation: 30 }), 'light');
  assert.equal(resolveTheme('auto-device', { prefersDark: true, sunElevation: 30 }), 'dark');
});

test('automatic, sun at station: light, dim in civil twilight, dark at night', () => {
  const sun = (sunElevation) => resolveTheme('auto-sun', { prefersDark: false, sunElevation });
  assert.equal(sun(10), 'light');
  assert.equal(sun(0), 'light', 'the sun on the horizon still counts as day');
  assert.equal(sun(-0.1), 'dim');
  assert.equal(sun(-6), 'dim');
  assert.equal(sun(-6.1), 'dark');
  assert.equal(sun(-40), 'dark');
});

test('automatic, sun at station, with no position: follows the device', () => {
  assert.equal(resolveTheme('auto-sun', { prefersDark: true, sunElevation: null }), 'dark');
  assert.equal(resolveTheme('auto-sun', { prefersDark: false, sunElevation: null }), 'light');
});

test('fixed themes ignore every input', () => {
  for (const theme of ['light', 'dim', 'dark']) {
    assert.equal(resolveTheme(theme, { prefersDark: true, sunElevation: -50 }), theme);
    assert.equal(resolveTheme(theme, { prefersDark: false, sunElevation: 50 }), theme);
  }
});

function memory(initial = {}) {
  const data = { ...initial };
  return {
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = String(v);
    },
    data,
  };
}

test('the theme choice is saved, with a safe default (constitution IV)', () => {
  assert.deepEqual(THEME_CHOICES, ['auto-device', 'auto-sun', 'light', 'dim', 'dark']);
  assert.equal(loadTheme(memory()), 'auto-device');
  for (const choice of THEME_CHOICES) {
    const store = memory();
    saveTheme(store, choice);
    assert.equal(store.data['lqm.theme'], choice);
    assert.equal(loadTheme(store), choice);
  }
  assert.equal(loadTheme(memory({ 'lqm.theme': 'sepia' })), 'auto-device', 'unknown value');
  assert.equal(loadTheme(null), 'auto-device', 'no storage');
  const throwing = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };
  assert.equal(loadTheme(throwing), 'auto-device');
  assert.doesNotThrow(() => saveTheme(throwing, 'dark'));
});

test('the status line says which theme is shown and why', () => {
  assert.equal(themeStatus('auto-device', { prefersDark: true, sunElevation: null }), 'Dark: device preference');
  assert.equal(themeStatus('auto-device', { prefersDark: false, sunElevation: null }), 'Light: device preference');
  assert.equal(
    themeStatus('auto-sun', { prefersDark: false, sunElevation: -12.4, place: 'JM49NH' }),
    'Dark: sun 12° below the horizon at JM49NH',
  );
  assert.equal(
    themeStatus('auto-sun', { prefersDark: false, sunElevation: 35.6, place: 'JM49NH' }),
    'Light: sun 36° above the horizon at JM49NH',
  );
  assert.equal(
    themeStatus('auto-sun', { prefersDark: false, sunElevation: -3, place: null }),
    'Dim: sun 3° below the horizon at the station',
  );
  assert.equal(
    themeStatus('auto-sun', { prefersDark: true, sunElevation: null }),
    'No station position yet: following the device (Dark)',
  );
  assert.equal(themeStatus('dim', { prefersDark: false, sunElevation: null }), 'Dim: chosen');
});
