// The grayline (feature 013, research R7–R9): the night side as a shade that follows the Sun's
// height at each place, drawn by the page into raster tiles (Web Mercator, so MapLibre shows them
// on the flat map, on every world copy, and on the globe). One raster layer, one pass: the map
// stays as smooth as without it (FR-016). Pure.

const DEG = Math.PI / 180;

/** The night shade at a Sun height (degrees): none at or above 0°, full from −12°, linear between. */
export function shade(elevation) {
  if (elevation >= 0) return 0;
  if (elevation <= -12) return 1;
  return -elevation / 12;
}

/** The Sun's height (degrees) at `lat`/`lon` with the Sun overhead at `subsolar`. */
export function elevationAt(subsolar, lat, lon) {
  const sin =
    Math.sin(lat * DEG) * Math.sin(subsolar.lat * DEG) +
    Math.cos(lat * DEG) * Math.cos(subsolar.lat * DEG) * Math.cos((lon - subsolar.lon) * DEG);
  return Math.asin(Math.max(-1, Math.min(1, sin))) / DEG;
}

/** `[lat, lon]` of the point `px`, `py` (pixels, may be fractional) of tile `z/x/y`, `size` px wide. */
export function tileLatLon(z, x, y, size, px, py) {
  const n = 2 ** z;
  const lon = ((x + px / size) / n) * 360 - 180;
  const lat = Math.atan(Math.sinh(Math.PI * (1 - (2 * (y + py / size)) / n))) / DEG;
  return [lat, lon];
}

/**
 * The RGBA pixels of grayline tile `z/x/y`, `size` px wide: the theme's night colour `rgb`
 * everywhere, with an opacity of `shade × nightOpacity` from each pixel centre's Sun height.
 */
export function tilePixels(z, x, y, size, subsolar, rgb, nightOpacity) {
  const data = new Uint8ClampedArray(size * size * 4);
  const sinδ = Math.sin(subsolar.lat * DEG);
  const cosδ = Math.cos(subsolar.lat * DEG);
  const cosΔλ = new Float64Array(size);
  for (let px = 0; px < size; px++) {
    const [, lon] = tileLatLon(z, x, y, size, px + 0.5, 0);
    cosΔλ[px] = Math.cos((lon - subsolar.lon) * DEG);
  }
  for (let py = 0; py < size; py++) {
    const [lat] = tileLatLon(z, x, y, size, 0, py + 0.5);
    const a = Math.sin(lat * DEG) * sinδ;
    const b = Math.cos(lat * DEG) * cosδ;
    for (let px = 0; px < size; px++) {
      const elevation = Math.asin(Math.max(-1, Math.min(1, a + b * cosΔλ[px]))) / DEG;
      const i = (py * size + px) * 4;
      data[i] = rgb[0];
      data[i + 1] = rgb[1];
      data[i + 2] = rgb[2];
      data[i + 3] = Math.round(shade(elevation) * nightOpacity * 255);
    }
  }
  return data;
}

/**
 * The style layer to insert the grayline before: the first boundary layer, so borders and place
 * names stay above it (FR-013); else the first symbol layer; else none (on top).
 */
export function graylineBeforeId(styleLayers) {
  const layers = styleLayers ?? [];
  return (
    layers.find((layer) => layer['source-layer'] === 'boundary')?.id ??
    layers.find((layer) => layer.type === 'symbol')?.id
  );
}

/**
 * The raster source's tile URL for a theme and the current minute (`grayline://` is the page's
 * own protocol): a new minute or a new theme gives new tiles; MapLibre keeps the old ones on
 * screen until the new ones are drawn, so nothing flashes.
 */
export function graylineTiles(theme, date) {
  return [`grayline://${theme}/${Math.floor(date.getTime() / 60_000)}/{z}/{x}/{y}`];
}

/** `{ theme, minute, z, x, y }` of a grayline tile URL, or `null`. */
export function parseGraylineUrl(url) {
  const match = /^grayline:\/\/([a-z]+)\/(\d+)\/(\d+)\/(\d+)\/(\d+)$/.exec(url);
  if (!match) return null;
  const [, theme, minute, z, x, y] = match;
  return { theme, minute: Number(minute), z: Number(z), x: Number(x), y: Number(y) };
}

/** `[r, g, b]` of a `#rrggbb` colour. */
export function rgbOf(hex) {
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
}
