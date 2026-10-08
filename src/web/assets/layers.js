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

/** One great-circle LineString per marker that has a station; longitudes stay unwrapped. */
export function lineFeatures(markers) {
  return {
    type: 'FeatureCollection',
    features: byZRank(markers)
      .filter((m) => m.station)
      .map((m) => ({
        type: 'Feature',
        geometry: {
          type: 'LineString',
          coordinates: greatCircle([m.station.lat, m.station.lon], [m.lat, m.lon], 64).map(([lat, lon]) => [lon, lat]),
        },
        properties: props(m),
      })),
  };
}
