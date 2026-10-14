// Themes (feature 011, research R1–R2): three palettes and the rule that picks one. Pure: no
// DOM except `applyPalette`, which takes the element it writes to.

/**
 * Light, Dim and Dark. Each palette holds:
 * - the CSS variables of app.css (camelCase here, `--kebab-case` there); `panelSolid` is
 *   `panel` without its alpha, for the settings panel (feature 012, B3);
 * - the globe's sky colours (`sky`, `horizon`, `fog`) and the great-circle line colour (`line`);
 * - the grayline's night colour and full night opacity (`night`, `nightOpacity`, feature 013);
 * - the OpenFreeMap style used with the default base map (`basemapStyle`);
 * - how much an operator-configured base map is dimmed (`dimOpacity`).
 *
 * Every text colour reaches 4.5:1 on its background (FR-002, tested).
 */
export const PALETTES = {
  light: {
    bg: '#ffffff',
    panel: '#ffffffeb',
    panelSolid: '#ffffff',
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
    line: '#000000',
    night: '#0b1a3c',
    nightOpacity: 0.4,
    basemapStyle: 'liberty',
    dimOpacity: 0,
  },
  dim: {
    bg: '#2b3240',
    panel: '#353d4deb',
    panelSolid: '#353d4d',
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
    line: '#e6e9ef',
    night: '#000814',
    nightOpacity: 0.45,
    basemapStyle: 'fiord',
    dimOpacity: 0.35,
  },
  dark: {
    bg: '#12151b',
    panel: '#1b2028eb',
    panelSolid: '#1b2028',
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
    line: '#ffffff',
    night: '#000000',
    nightOpacity: 0.55,
    basemapStyle: 'dark',
    dimOpacity: 0.55,
  },
};

/** The palette keys app.css reads, with their CSS variable names. */
const CSS_VARIABLES = {
  bg: '--bg',
  panel: '--panel',
  panelSolid: '--panel-solid',
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

/** The five theme choices of the Appearance group (FR-001); the first is the default. */
export const THEME_CHOICES = ['auto-device', 'auto-sun', 'light', 'dim', 'dark'];
const THEME_KEY = 'lqm.theme';

/**
 * The theme to show (research R2):
 * - `auto-device`: Dark when the device prefers dark, else Light;
 * - `auto-sun`: Light with the sun at or above the horizon, Dim in civil twilight (down to 6°
 *   below), Dark lower. With no station position (`sunElevation` null), like `auto-device`;
 * - `light`, `dim`, `dark`: that theme.
 */
export function resolveTheme(choice, { prefersDark, sunElevation }) {
  if (choice === 'light' || choice === 'dim' || choice === 'dark') return choice;
  if (choice === 'auto-sun' && sunElevation !== null && sunElevation !== undefined) {
    if (sunElevation >= 0) return 'light';
    return sunElevation >= -6 ? 'dim' : 'dark';
  }
  return prefersDark ? 'dark' : 'light';
}

/** The saved choice; anything missing, unknown or unreadable gives the default (constitution IV). */
export function loadTheme(storage) {
  try {
    const value = storage ? storage.getItem(THEME_KEY) : null;
    return THEME_CHOICES.includes(value) ? value : THEME_CHOICES[0];
  } catch {
    return THEME_CHOICES[0];
  }
}

export function saveTheme(storage, choice) {
  try {
    if (storage) storage.setItem(THEME_KEY, choice);
  } catch {
    // A browser that refuses storage keeps the choice for this visit only.
  }
}

const NAMES = { light: 'Light', dim: 'Dim', dark: 'Dark' };

/** The Appearance group's status line: the theme shown and why (contracts/page-ui.md). */
export function themeStatus(choice, inputs) {
  const theme = NAMES[resolveTheme(choice, inputs)];
  if (choice === 'auto-device') return `${theme}: device preference`;
  if (choice !== 'auto-sun') return `${theme}: chosen`;
  const { sunElevation, place } = inputs;
  if (sunElevation === null || sunElevation === undefined) {
    return `No station position yet: following the device (${theme})`;
  }
  const degrees = Math.round(Math.abs(sunElevation));
  const where = place ? `at ${place}` : 'at the station';
  const side = sunElevation >= 0 ? 'above' : 'below';
  return `${theme}: sun ${degrees}° ${side} the horizon ${where}`;
}
