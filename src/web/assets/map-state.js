// Pure map state: no DOM, no Leaflet (data-model "Frontend state"). Every decision about what
// the map shows lives here, so it can be tested with `node --test`.

export function createState({ bands = {}, fallbackColor = '#7f7f7f', limit = 20 } = {}) {
  return { bands, fallbackColor, limit, qsos: new Map() };
}

export function applySnapshot(state, qsos) {
  state.qsos = new Map();
  for (const qso of qsos) {
    state.qsos.set(qso.id, qso);
  }
}

export function applyUpsert(state, qso) {
  state.qsos.set(qso.id, qso);
}

function compare(a, b) {
  if (a.sort_key !== b.sort_key) return a.sort_key < b.sort_key ? -1 : 1;
  if (a.id !== b.id) return a.id < b.id ? -1 : 1;
  return 0;
}

/** Changes how many recent QSOs are shown (null = all). */
export function setLimit(state, limit) {
  state.limit = limit;
}

/** The last `limit` QSOs by sort key (all when `limit` is null), ascending. */
export function visible(state) {
  const all = [...state.qsos.values()].sort(compare);
  return state.limit == null ? all : all.slice(Math.max(0, all.length - state.limit));
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
