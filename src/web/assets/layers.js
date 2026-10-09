// Pure GeoJSON builders for the marker and line layers.
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

import { greatCircle } from './geodesic.js';

function byZRank(markers) {
  return [...markers.values()].sort((a, b) => a.zRank - b.zRank);
}

function props(marker) {
  return { key: marker.key, color: marker.color, zRank: marker.zRank };
}

/** One Point per marker, `[lon, lat]`, ordered by `zRank` (newest last, drawn on top). */
export function markerFeatures(markers) {
  return {
    type: 'FeatureCollection',
    features: byZRank(markers).map((m) => ({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: [m.lon, m.lat] },
      properties: props(m),
    })),
  };
}

/**
 * One black great-circle LineString from the QTH to every marker (feature 004, FR-004); none when
 * the QTH is unknown. Longitudes stay unwrapped.
 */
export function lineFeatures(markers, qth) {
  if (!qth) return { type: 'FeatureCollection', features: [] };
  return {
    type: 'FeatureCollection',
    features: byZRank(markers).map((m) => ({
      type: 'Feature',
      geometry: {
        type: 'LineString',
        coordinates: greatCircle([qth.lat, qth.lon], [m.lat, m.lon], 64).map(([lat, lon]) => [lon, lat]),
      },
      properties: { key: m.key, zRank: m.zRank },
    })),
  };
}
