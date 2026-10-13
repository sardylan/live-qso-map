// Themes (feature 011, research R1–R2): three palettes and the rule that picks one. Pure: no
// DOM except `applyPalette`, which takes the element it writes to.

/**
 * Light, Dim and Dark. Each palette holds:
 * - the CSS variables of app.css (camelCase here, `--kebab-case` there);
 * - the globe's sky colours (`sky`, `horizon`, `fog`);
 * - the OpenFreeMap style used with the default base map (`basemapStyle`);
 * - how much an operator-configured base map is dimmed (`dimOpacity`).
 *
 * Every text colour reaches 4.5:1 on its background (FR-002, tested).
 */
export const PALETTES = {
  light: {
    bg: '#ffffff',
    panel: '#ffffffeb',
    text: '#1a1a1a',
    muted: '#666666',
    border: '#dddddd',
    accent: '#1f5fbf',
    highlight: '#dbe9ff',
    banner: '#b03a2e',
    bannerLow: '#7a5c00',
    bannerText: '#ffffff',
    noticeBg: '#fff4e5',
    noticeText: '#5c3a00',
    shadow: 'rgba(0, 0, 0, 0.3)',
    space: '#0b1020',
    sky: '#88c6fc',
    horizon: '#ffffff',
    fog: '#ffffff',
    basemapStyle: 'liberty',
    dimOpacity: 0,
  },
  dim: {
    bg: '#2b3240',
    panel: '#353d4deb',
    text: '#e6e9ef',
    muted: '#b4bccb',
    border: '#4a5366',
    accent: '#9dbcff',
    highlight: '#45526b',
    banner: '#a8392d',
    bannerLow: '#6e5300',
    bannerText: '#ffffff',
    noticeBg: '#3d3423',
    noticeText: '#f3dcb4',
    shadow: 'rgba(0, 0, 0, 0.5)',
    space: '#05070d',
    sky: '#2a3a5c',
    horizon: '#5a6f9a',
    fog: '#3a4660',
    basemapStyle: 'fiord',
    dimOpacity: 0.35,
  },
  dark: {
    bg: '#12151b',
    panel: '#1b2028eb',
    text: '#e8eaed',
    muted: '#a8b0bd',
    border: '#333a46',
    accent: '#8ab4f8',
    highlight: '#2c3a52',
    banner: '#a33327',
    bannerLow: '#6b5200',
    bannerText: '#ffffff',
    noticeBg: '#2e2716',
    noticeText: '#f0d9a8',
    shadow: 'rgba(0, 0, 0, 0.6)',
    space: '#000000',
    sky: '#0b1426',
    horizon: '#1b2a4a',
    fog: '#0e1a30',
    basemapStyle: 'dark',
    dimOpacity: 0.55,
  },
};

/** The palette keys app.css reads, with their CSS variable names. */
const CSS_VARIABLES = {
  bg: '--bg',
  panel: '--panel',
  text: '--text',
  muted: '--muted',
  border: '--border',
  accent: '--accent',
  highlight: '--highlight',
  banner: '--banner',
  bannerLow: '--banner-low',
  bannerText: '--banner-text',
  noticeBg: '--notice-bg',
  noticeText: '--notice-text',
  shadow: '--shadow',
  space: '--space',
};

/** `#rgb`, `#rrggbb` or `#rrggbbaa` → [r, g, b] in 0–255; the alpha channel is ignored. */
function rgb(hex) {
  let h = hex.replace('#', '');
  if (h.length === 3) h = [...h].map((c) => c + c).join('');
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
}

function luminance(hex) {
  const [r, g, b] = rgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio between two colours, from 1 to 21. */
export function contrastRatio(foreground, background) {
  const [hi, lo] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (hi + 0.05) / (lo + 0.05);
}

/** Writes the palette's CSS variables on `root` (the `<html>` element) and marks the theme. */
export function applyPalette(root, name) {
  const palette = PALETTES[name];
  for (const [key, variable] of Object.entries(CSS_VARIABLES)) {
    root.style.setProperty(variable, palette[key]);
  }
  root.dataset.theme = name;
}
