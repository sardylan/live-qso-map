// Pure framing geometry for auto-zoom: bounds, bounding cap, padding (feature 004, research R5, R6).
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

const KM_PER_DEGREE = 111.195;

/** `[lat, lon]` of the QTH (first, when known) and of every marker: what auto-zoom keeps in view. */
export function framedPoints(qth, markers) {
  const points = qth ? [[qth.lat, qth.lon]] : [];
  for (const m of markers.values()) points.push([m.lat, m.lon]);
  return points;
}

/**
 * The frame with no markers (FR-010), as `[[w, s], [e, n]]` with `e > w`: the station entity's
 * extent, else `radiusKm` around the QTH, else `null` (the whole world).
 */
export function fallbackFrame(qth, entity, radiusKm = 500) {
  if (!qth) return null;
  if (entity && Array.isArray(entity.bounds)) {
    const [w, s, e, n] = entity.bounds;
    return [[w, s], [e < w ? e + 360 : e, n]];
  }
  const halfLat = radiusKm / KM_PER_DEGREE;
  const cosLat = Math.max(Math.cos((qth.lat * Math.PI) / 180), 0.01);
  const halfLon = Math.min(halfLat / cosLat, 180);
  return [
    [qth.lon - halfLon, Math.max(qth.lat - halfLat, -90)],
    [qth.lon + halfLon, Math.min(qth.lat + halfLat, 90)],
  ];
}

/**
 * `[[w, s], [e, n]]` over `[lat, lon]` points, along the shortest longitude span: when that span
 * crosses the antimeridian, `e` is above 180 so the flat map frames the short way. `null` when empty.
 */
export function flatBounds(points) {
  if (points.length === 0) return null;
  const lats = points.map(([lat]) => lat);
  const lons = [...new Set(points.map(([, lon]) => lon))].sort((a, b) => a - b);
  let west = lons[0];
  let east = lons[lons.length - 1];
  let gap = lons[0] + 360 - lons[lons.length - 1];
  for (let i = 0; i + 1 < lons.length; i++) {
    const g = lons[i + 1] - lons[i];
    if (g > gap) {
      gap = g;
      west = lons[i + 1];
      east = lons[i] + 360;
    }
  }
  return [[west, Math.min(...lats)], [east, Math.max(...lats)]];
}

const SNAP_PX = 48;

/**
 * Padding that keeps the fit clear of page panels (FR-009). Each visible panel pads one map edge:
 * among the edges it sits against (within SNAP_PX), the one that costs the least depth; a floating
 * panel pads its nearest edge. When the free area left is under `minFree`, the padding is a
 * uniform `gap` (the small-screen edge case).
 */
export function paddingFor(mapRect, rects, { gap = 12, minFree = [240, 160] } = {}) {
  const depth = { top: 0, right: 0, bottom: 0, left: 0 };
  for (const r of rects) {
    if (!r || r.width <= 0 || r.height <= 0) continue;
    const edges = [
      { edge: 'left', distance: r.left - mapRect.left, depth: r.right - mapRect.left },
      { edge: 'right', distance: mapRect.right - r.right, depth: mapRect.right - r.left },
      { edge: 'top', distance: r.top - mapRect.top, depth: r.bottom - mapRect.top },
      { edge: 'bottom', distance: mapRect.bottom - r.bottom, depth: mapRect.bottom - r.top },
    ];
    const against = edges.filter((e) => e.distance <= SNAP_PX);
    const choice = against.length > 0
      ? against.reduce((a, b) => (b.depth < a.depth ? b : a))
      : edges.reduce((a, b) => (b.distance < a.distance ? b : a));
    depth[choice.edge] = Math.max(depth[choice.edge], choice.depth);
  }
  const padding = {
    top: depth.top + gap,
    right: depth.right + gap,
    bottom: depth.bottom + gap,
    left: depth.left + gap,
  };
  const freeWidth = mapRect.width - padding.left - padding.right;
  const freeHeight = mapRect.height - padding.top - padding.bottom;
  if (freeWidth < minFree[0] || freeHeight < minFree[1]) {
    return { top: gap, right: gap, bottom: gap, left: gap };
  }
  return padding;
}

const RAD = Math.PI / 180;

function toVector([lat, lon]) {
  const φ = lat * RAD;
  const λ = lon * RAD;
  return [Math.cos(φ) * Math.cos(λ), Math.cos(φ) * Math.sin(λ), Math.sin(φ)];
}

function normalise(v) {
  const n = Math.hypot(v[0], v[1], v[2]);
  return n < 1e-12 ? null : [v[0] / n, v[1] / n, v[2] / n];
}

function angleBetween(a, b) {
  const dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  return Math.acos(Math.min(1, Math.max(-1, dot))) / RAD;
}

const CAP_ITERATIONS = 400;

/**
 * An approximately smallest spherical cap holding every `[lat, lon]` point (research R5): the
 * Bădoiu–Clarkson iteration on unit vectors, which moves the centre a shrinking step towards the
 * farthest point. The radius is exact for the centre found, so the cap always contains the points.
 */
export function boundingCap(points) {
  const vectors = points.map(toVector);
  let centre = normalise(vectors.reduce((s, v) => [s[0] + v[0], s[1] + v[1], s[2] + v[2]], [0, 0, 0])) ?? vectors[0];
  const farthest = () => {
    let best = vectors[0];
    let bestAngle = -1;
    for (const v of vectors) {
      const a = angleBetween(centre, v);
      if (a > bestAngle) {
        bestAngle = a;
        best = v;
      }
    }
    return [best, bestAngle];
  };
  for (let i = 1; i <= CAP_ITERATIONS; i++) {
    const [p, a] = farthest();
    if (a < 1e-9) break;
    const t = 1 / (i + 1);
    const next = normalise([centre[0] + (p[0] - centre[0]) * t, centre[1] + (p[1] - centre[1]) * t, centre[2] + (p[2] - centre[2]) * t]);
    if (next) centre = next;
  }
  const [, radiusDeg] = farthest();
  const lat = Math.asin(Math.max(-1, Math.min(1, centre[2]))) / RAD;
  const lon = Math.atan2(centre[1], centre[0]) / RAD;
  return { center: [lat, lon], radiusDeg };
}

/** True when the cap, plus a margin, lies on one side of the globe (FR-016). */
export function fitsGlobe(cap, marginDeg) {
  return cap.radiusDeg + marginDeg <= 90;
}

const TILE_SIZE = 512;

/**
 * A globe zoom that shows the cap inside the viewport (`[width, height]` of the free area), used
 * when the camera cannot compute one itself. Capped at 12 (FR-013).
 */
export function zoomForCap(cap, viewport) {
  const half = Math.min(viewport[0], viewport[1]) / 2;
  const reach = Math.sin(Math.min(cap.radiusDeg, 90) * RAD);
  if (reach < 1e-9) return 12;
  const radiusPx = half / reach;
  const zoom = Math.log2((radiusPx * 2 * Math.PI) / TILE_SIZE);
  return Math.max(0, Math.min(12, zoom));
}
