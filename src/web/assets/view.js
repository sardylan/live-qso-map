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
