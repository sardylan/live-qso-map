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
    for (const key of [...CSS_KEYS, 'sky', 'horizon', 'fog', 'basemapStyle', 'dimOpacity']) {
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
