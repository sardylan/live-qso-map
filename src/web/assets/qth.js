// Pure QTH helpers: where the station is and the house drawn there (feature 004, research R1, R3).
// No DOM, no MapLibre: everything here is tested with `node --test web/`.

function compareNewestFirst(a, b) {
  if (a.sort_key !== b.sort_key) return a.sort_key < b.sort_key ? 1 : -1;
  if (a.id !== b.id) return a.id < b.id ? 1 : -1;
  return 0;
}

/**
 * The station's own location (FR-002): the configured position from `hello.station`, otherwise
 * the station position of the newest QSO (by QSO time) held by the page, otherwise `null`.
 */
export function currentQth(helloStation, state) {
  const configured = helloStation && helloStation.position;
  if (configured) {
    return {
      lat: configured.lat,
      lon: configured.lon,
      callsign: helloStation.callsign ?? null,
      locator: helloStation.locator ?? null,
      source: configured.source,
      precision: configured.precision,
      origin: 'config',
    };
  }
  if (!state) return null;
  const newest = [...state.qsos.values()].filter((q) => q.station).sort(compareNewestFirst)[0];
  if (!newest) return null;
  return {
    lat: newest.station.lat,
    lon: newest.station.lon,
    callsign: newest.station_call ?? null,
    locator: (newest.adif && newest.adif.MY_GRIDSQUARE) || null,
    source: newest.station.source,
    precision: newest.station.precision,
    origin: 'qso',
  };
}

/** Zero or one Point for the house layer. */
export function houseFeature(qth) {
  return {
    type: 'FeatureCollection',
    features: qth
      ? [{ type: 'Feature', geometry: { type: 'Point', coordinates: [qth.lon, qth.lat] }, properties: {} }]
      : [],
  };
}

/** A small house, readable on light and dark base maps (research R3). A constant: no data in it. */
export const HOUSE_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 28 28">' +
  '<path d="M14 3 L26 14 H22 V25 H6 V14 H2 Z" fill="#fff8e7" stroke="#222" stroke-width="2" stroke-linejoin="round"/>' +
  '<rect x="11.5" y="17" width="5" height="8" fill="#222"/>' +
  '</svg>';
