// Pure map state: no DOM, no Leaflet (data-model "Frontend state"). Every decision about what
// the map shows lives here, so it can be tested with `node --test`.

/**
 * `bands` is the ordered list from `hello` (`[{ band, color }]`, frequency order, HR-2).
 * `bandOrder` keeps that order for anything that lists bands.
 */
export function createState({ bands = [], fallbackColor = '#7f7f7f', limit = 20 } = {}) {
  const colors = {};
  for (const { band, color } of bands) colors[band.toLowerCase()] = color;
  // `limit` is the server window (the last snapshot's); `mapLimit` is the viewer's map "Last N"
  // (feature 006, research R1). They start equal.
  // `shownBands` is the Legend choice (feature 011): `null` for every band, else a set of
  // lower-case band names.
  return {
    bands: colors,
    bandOrder: bands.map((b) => b.band),
    fallbackColor,
    limit,
    mapLimit: limit,
    shownBands: null,
    qsos: new Map(),
  };
}

export function applySnapshot(state, qsos) {
  state.qsos = new Map();
  for (const qso of qsos) {
    state.qsos.set(qso.id, qso);
  }
}

/** Forgets a QSO the server removed; a later upsert with the same id is new (feature 003). */
export function applyRemove(state, id) {
  state.qsos.delete(id);
}

/** Applies a live upsert; a lower or equal revision of a known QSO is ignored (FR-009). */
export function applyUpsert(state, qso) {
  const known = state.qsos.get(qso.id);
  if (known && qso.rev <= known.rev) return;
  state.qsos.set(qso.id, qso);
}

function compare(a, b) {
  if (a.sort_key !== b.sort_key) return a.sort_key < b.sort_key ? -1 : 1;
  if (a.id !== b.id) return a.id < b.id ? -1 : 1;
  return 0;
}

/** Records the server window the page asked for (null = all); the map keeps its own limit. */
export function setLimit(state, limit) {
  state.limit = limit;
}

/** Changes how many recent QSOs the map shows (null = all) (feature 006). */
export function setMapLimit(state, limit) {
  state.mapLimit = limit;
}

/** The window to ask the server for: at least 20, the largest box (feature 006, TR-006). */
export function serverLimit(mapLimit) {
  return mapLimit == null ? null : Math.max(mapLimit, 20);
}

/** Shows only QSOs on `bands` (case-insensitive), or every QSO for `null` (feature 011). */
export function setShownBands(state, bands) {
  state.shownBands = bands == null ? null : new Set(bands.map((band) => band.toLowerCase()));
}

function shown(state, qso) {
  if (state.shownBands === null) return true;
  // With a band choice, a QSO without a BAND is hidden (data-model "Legend choice").
  return typeof qso.band === 'string' && state.shownBands.has(qso.band.toLowerCase());
}

/** Every shown QSO the page holds, ascending by sort key: what the box draws from. */
export function held(state) {
  return [...state.qsos.values()].filter((qso) => shown(state, qso)).sort(compare);
}

/** The last `mapLimit` QSOs by sort key (all when null), ascending: what the map draws. */
export function visible(state) {
  const all = held(state);
  return state.mapLimit == null ? all : all.slice(Math.max(0, all.length - state.mapLimit));
}

function colorOf(state, qso) {
  const band = typeof qso.band === 'string' ? qso.band.toLowerCase() : null;
  return (band && state.bands[band]) || state.fallbackColor;
}

/**
 * One marker per distinct remote position among the visible QSOs. Each marker takes its colour,
 * z-rank and station from its most recent QSO.
 */
export function markers(state) {
  const result = new Map();
  visible(state).forEach((qso, rank) => {
    if (!qso.remote) return;
    const key = `${qso.remote.lat},${qso.remote.lon}`;
    let marker = result.get(key);
    if (!marker) {
      marker = { key, lat: qso.remote.lat, lon: qso.remote.lon, qsos: [] };
      result.set(key, marker);
    }
    marker.qsos.push(qso);
    marker.zRank = rank;
    marker.color = colorOf(state, qso);
    marker.station = qso.station ?? null;
  });
  return result;
}

function signature(marker) {
  return JSON.stringify([
    marker.color,
    marker.zRank,
    marker.station ? [marker.station.lat, marker.station.lon] : null,
    marker.qsos.map((q) => [q.id, q.rev]),
  ]);
}

/** What changed between two `markers()` results, so the view only applies the difference. */
export function diffLayers(prev, next) {
  const add = [];
  const update = [];
  for (const [key, marker] of next) {
    const old = prev.get(key);
    if (!old) add.push(marker);
    else if (signature(old) !== signature(marker)) update.push(marker);
  }
  const remove = [...prev.keys()].filter((key) => !next.has(key));
  return { add, update, remove };
}
