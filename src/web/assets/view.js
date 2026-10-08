// Pure view helpers: base map style, projection preference, WebGL check.
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

/** The `style` option for the map: a style URL, or a raster style built from a tile template. */
export function buildStyle(basemap) {
  return basemap.url;
}
