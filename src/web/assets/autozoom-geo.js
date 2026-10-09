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
