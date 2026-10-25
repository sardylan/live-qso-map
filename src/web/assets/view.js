// Pure view helpers: base map style, projection preference, WebGL check.
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

/** The `style` option for the map: a style URL, or a raster style built from a tile template. */
export function buildStyle(basemap) {
  if (basemap.kind === 'raster') {
    return {
      version: 8,
      sources: { basemap: { type: 'raster', tiles: [basemap.url], tileSize: 256 } },
      layers: [{ id: 'basemap', type: 'raster', source: 'basemap' }],
    };
  }
  return basemap.url;
}

const OPENFREEMAP_STYLES = 'https://tiles.openfreemap.org/styles/';
/** The service's default base map (`DEFAULT_STYLE_URL` in src/config.rs). */
export const DEFAULT_STYLE_URL = `${OPENFREEMAP_STYLES}liberty`;

/**
 * The base map for a theme (feature 011, research R4): with the default OpenFreeMap map, the
 * theme's own style and no dimming; with any operator-configured map (another style, raster
 * tiles), the same style (`styleUrl` null) dimmed by the theme's opacity.
 */
export function basemapFor(basemap, palette, defaultUrl) {
  if (basemap.kind !== 'raster' && basemap.url === defaultUrl) {
    return { styleUrl: `${OPENFREEMAP_STYLES}${palette.basemapStyle}`, dimOpacity: 0 };
  }
  return { styleUrl: null, dimOpacity: palette.dimOpacity };
}

/** True when the browser can create a WebGL context (FR-012). */
export function hasWebGL(createCanvas) {
  try {
    const canvas = createCanvas();
    return Boolean(canvas.getContext('webgl2') || canvas.getContext('webgl'));
  } catch {
    return false;
  }
}

/** True for map errors that come from the base map rather than the QSO overlays. */
export function isBasemapError(event, overlaySources) {
  return !overlaySources.includes(event?.sourceId);
}

/** Draw only once the map has loaded and something changed. */
export function shouldRender(loaded, diff) {
  return loaded && (diff.add.length > 0 || diff.update.length > 0 || diff.remove.length > 0);
}

const PROJECTION_KEY = 'lqm.projection';
const PROJECTIONS = ['globe', 'mercator'];

/** The viewer's projection: globe unless they chose mercator (FR-010). Never throws. */
export function loadProjection(storage) {
  try {
    const value = storage?.getItem(PROJECTION_KEY);
    return PROJECTIONS.includes(value) ? value : 'globe';
  } catch {
    return 'globe';
  }
}

export function saveProjection(storage, projection) {
  try {
    storage?.setItem(PROJECTION_KEY, projection);
  } catch {
    // Storage may be unavailable (private mode); the choice then lasts for this page only.
  }
}

export function nextProjection(projection) {
  return projection === 'globe' ? 'mercator' : 'globe';
}

/** True when the open popup belongs to a marker that disappeared (FR-012, feature 003). */
export function popupToClose(openKey, diff) {
  return openKey != null && diff.remove.includes(openKey);
}

const AUTOZOOM_KEY = 'lqm.autozoom';

/** Auto-zoom: on unless the viewer switched it off (FR-007). Never throws. */
export function loadAutoZoom(storage) {
  try {
    return storage?.getItem(AUTOZOOM_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function saveAutoZoom(storage, enabled) {
  try {
    storage?.setItem(AUTOZOOM_KEY, enabled ? 'on' : 'off');
  } catch {
    // Storage may be unavailable; the choice then lasts for this page only.
  }
}

const GLOBE_BORDER_DEG = 10;

/** The operator's globe border from hello (feature 016, FR-005): 10° unless valid (0 to 45). */
export function globeBorder(hello) {
  const value = hello?.globe_border_deg;
  return typeof value === 'number' && value >= 0 && value <= 45 ? value : GLOBE_BORDER_DEG;
}

const RECENT_KEY = 'lqm.recent';

/** The last-QSO box: shown unless the viewer hid it (FR-021). Never throws. */
export function loadRecent(storage) {
  try {
    return storage?.getItem(RECENT_KEY) !== 'hidden';
  } catch {
    return true;
  }
}

export function saveRecent(storage, shown) {
  try {
    storage?.setItem(RECENT_KEY, shown ? 'shown' : 'hidden');
  } catch {
    // Storage may be unavailable; the choice then lasts for this page only.
  }
}

/** The distance scale is only correct on the flat map (FR-023, FR-024). */
export function scaleVisible(projection) {
  return projection === 'mercator';
}

const RECENT_ORDER_KEY = 'lqm.recent.order';

/** Where the newest QSO goes in the box: 'bottom' unless the viewer chose 'top' (feature 006). */
export function loadRecentOrder(storage) {
  try {
    return storage?.getItem(RECENT_ORDER_KEY) === 'top' ? 'top' : 'bottom';
  } catch {
    return 'bottom';
  }
}

export function saveRecentOrder(storage, order) {
  try {
    storage?.setItem(RECENT_ORDER_KEY, order);
  } catch {
    // Storage may be unavailable; the choice then lasts for this page only.
  }
}

const ATMOSPHERE_KEY = 'lqm.atmosphere';

/** Whether the globe shows its atmosphere: on unless the viewer switched it off (FR-010). */
export function loadAtmosphere(storage) {
  try {
    return storage?.getItem(ATMOSPHERE_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function saveAtmosphere(storage, on) {
  try {
    storage?.setItem(ATMOSPHERE_KEY, on ? 'on' : 'off');
  } catch {
    // A browser that refuses storage keeps the choice for this visit only.
  }
}

const GRAYLINE_KEY = 'lqm.grayline';
const SUN_KEY = 'lqm.sun';

/** Whether the map shows the grayline: on unless the viewer switched it off (feature 013, FR-009). */
export function loadGrayline(storage) {
  try {
    return storage?.getItem(GRAYLINE_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function saveGrayline(storage, on) {
  try {
    storage?.setItem(GRAYLINE_KEY, on ? 'on' : 'off');
  } catch {
    // A browser that refuses storage keeps the choice for this visit only.
  }
}

/** Whether the map shows the Sun symbol: off unless the viewer switched it on (FR-018). */
export function loadSunMarker(storage) {
  try {
    return storage?.getItem(SUN_KEY) === 'on';
  } catch {
    return false;
  }
}

export function saveSunMarker(storage, on) {
  try {
    storage?.setItem(SUN_KEY, on ? 'on' : 'off');
  } catch {
    // A browser that refuses storage keeps the choice for this visit only.
  }
}

/**
 * The MapLibre sky for a theme (feature 011, research R5): the theme's colours, and the
 * atmosphere glow only on the globe and only when it is on (FR-010, FR-011).
 */
export function skyFor(palette, { atmosphere, projection }) {
  return {
    'sky-color': palette.sky,
    'horizon-color': palette.horizon,
    'fog-color': palette.fog,
    'sky-horizon-blend': 0.5,
    'horizon-fog-blend': 0.5,
    'atmosphere-blend': atmosphere && projection === 'globe' ? 1 : 0,
  };
}

/**
 * The MapLibre light for the atmosphere (feature 013, research R2): the glow is brightest on the
 * globe's edge nearest the Sun, as the Sun lies from the viewer, and the globe is always lit
 * from behind, so the scattering never hazes the map, its markers or its lines (owner's rule:
 * the data stays fully visible). `subsolar` is `{ lat, lon }`; `camera` is the map's `{ lat,
 * lng, bearing, pitch }`. A viewport light, recomputed on every camera move.
 */
export function lightFor(subsolar, camera) {
  const D = Math.PI / 180;
  const φ = subsolar.lat * D;
  const λ = subsolar.lon * D;
  // The Sun's direction on the Earth (MapLibre's globe axes), then into camera space with the
  // globe's own rotations: Rx(−pitch)·Rz(bearing)·Rx(lat)·Ry(−lng).
  let [x, y, z] = [Math.cos(φ) * Math.sin(λ), Math.sin(φ), Math.cos(φ) * Math.cos(λ)];
  let t = -camera.lng * D;
  [x, z] = [x * Math.cos(t) + z * Math.sin(t), -x * Math.sin(t) + z * Math.cos(t)];
  t = camera.lat * D;
  [y, z] = [y * Math.cos(t) - z * Math.sin(t), y * Math.sin(t) + z * Math.cos(t)];
  t = (camera.bearing ?? 0) * D;
  [x, y] = [x * Math.cos(t) - y * Math.sin(t), x * Math.sin(t) + y * Math.cos(t)];
  t = -(camera.pitch ?? 0) * D;
  [y, z] = [y * Math.cos(t) - z * Math.sin(t), y * Math.sin(t) + z * Math.cos(t)];
  // Keep the on-screen direction, put the light well behind the globe (z ≤ −0.93): a rim only,
  // with no crescent of haze over the land near the limb.
  const BEHIND = 2.5;
  const n = Math.hypot(x, y, BEHIND);
  const u = [x / n, y / n, -BEHIND / n];
  // MapLibre's sun direction is −sphericalToCartesian(position).
  const polar = Math.acos(Math.max(-1, Math.min(1, -u[2]))) / D;
  const azimuthal = Math.atan2(u[0], -u[1]) / D;
  return { anchor: 'viewport', position: [1.15, azimuthal, polar] };
}
