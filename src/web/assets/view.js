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
