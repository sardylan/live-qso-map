// DOM and MapLibre glue only: every decision is made in map-state.js, layers.js, view.js,
// geodesic.js and connection.js.
// Popups are built with DOM nodes and textContent, never HTML strings, because QSO data comes
// from an open UDP port (FR-035). Only the operator-configured attribution is HTML.
/* global maplibregl */

import { createState, applySnapshot, applyUpsert, markers, diffLayers, setLimit } from './map-state.js';
import { markerFeatures, lineFeatures } from './layers.js';
import {
  buildStyle,
  hasWebGL,
  isBasemapError,
  shouldRender,
  loadProjection,
  saveProjection,
  nextProjection,
} from './view.js';
import { createConnection, onOpen, onClose, onHello as handshake, loadLimit, saveLimit } from './connection.js';

const OVERLAYS = ['qso-lines', 'qso-markers'];

let map = null;
let loaded = false;
let state = null;
let current = new Map();
let socket = null;

function sessionStore() {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

function localStore() {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

const connection = createConnection(loadLimit(sessionStore()));
let projection = loadProjection(localStore());

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = String(text);
  return node;
}

function formatTime(datetime) {
  return typeof datetime === 'string' ? datetime.replace('T', ' ').replace(/:\d\dZ$/, 'Z') : '';
}

function popupContent(marker) {
  const box = element('div', 'qso-popup');
  const table = element('table');
  for (const qso of [...marker.qsos].reverse()) {
    const row = element('tr');
    const mode = qso.submode ? `${qso.mode ?? ''}/${qso.submode}` : (qso.mode ?? '');
    row.append(
      element('td', 'call', qso.call),
      element('td', null, qso.band),
      element('td', null, mode),
      element('td', null, formatTime(qso.datetime)),
      element('td', 'meta', qso.remote ? `${qso.remote.source} · ${qso.remote.precision}` : ''),
    );
    table.append(row);
  }
  box.append(table);
  return box;
}

function show(id, visible) {
  document.getElementById(id).hidden = !visible;
}

function addOverlays() {
  const empty = { type: 'FeatureCollection', features: [] };
  map.addSource('qso-lines', { type: 'geojson', data: empty });
  map.addLayer({
    id: 'qso-lines',
    type: 'line',
    source: 'qso-lines',
    layout: { 'line-cap': 'round' },
    paint: { 'line-color': ['get', 'color'], 'line-width': 1.5, 'line-opacity': 0.7 },
  });
  map.addSource('qso-markers', { type: 'geojson', data: empty });
  map.addLayer({
    id: 'qso-markers',
    type: 'circle',
    source: 'qso-markers',
    layout: { 'circle-sort-key': ['get', 'zRank'] },
    paint: {
      'circle-radius': 6,
      'circle-color': ['get', 'color'],
      'circle-stroke-color': '#fff',
      'circle-stroke-width': 1,
    },
  });
}

/** A map control with one button that switches between globe and flat (FR-010). */
class ProjectionControl {
  onAdd() {
    this.container = element('div', 'maplibregl-ctrl maplibregl-ctrl-group projection-control');
    this.button = element('button');
    this.button.type = 'button';
    this.button.addEventListener('click', () => {
      projection = nextProjection(projection);
      // Before `load` the choice is only stored; the load handler applies it.
      if (loaded) map.setProjection({ type: projection });
      saveProjection(localStore(), projection);
      this.update();
    });
    this.container.append(this.button);
    this.update();
    return this.container;
  }

  update() {
    const target = nextProjection(projection) === 'globe' ? 'Globe' : 'Flat';
    this.button.textContent = target;
    this.button.title = `Switch to ${target.toLowerCase()} view`;
  }

  onRemove() {
    this.container.remove();
  }
}

function render() {
  if (!state) return;
  const next = markers(state);
  if (!shouldRender(loaded, diffLayers(current, next))) return;
  map.getSource('qso-markers').setData(markerFeatures(next));
  map.getSource('qso-lines').setData(lineFeatures(next));
  current = next;
}

function createMap(hello) {
  map = new maplibregl.Map({
    container: 'map',
    style: buildStyle(hello.basemap),
    center: [0, 30],
    zoom: 1.5,
    attributionControl: false,
    renderWorldCopies: true,
  });
  map.addControl(
    new maplibregl.AttributionControl({ compact: false, customAttribution: hello.basemap.attribution || undefined }),
  );
  map.addControl(new maplibregl.NavigationControl(), 'top-left');
  map.addControl(new ProjectionControl(), 'top-left');
  map.on('load', () => {
    map.setProjection({ type: projection });
    addOverlays();
    loaded = true;
    current = new Map();
    render();
  });
  map.on('click', 'qso-markers', (event) => {
    const feature = event.features && event.features[0];
    const marker = feature && current.get(feature.properties.key);
    if (!marker) return;
    new maplibregl.Popup({ maxWidth: 'none' })
      .setLngLat(feature.geometry.coordinates)
      .setDOMContent(popupContent(marker))
      .addTo(map);
  });
  map.on('mouseenter', 'qso-markers', () => {
    map.getCanvas().style.cursor = 'pointer';
  });
  map.on('mouseleave', 'qso-markers', () => {
    map.getCanvas().style.cursor = '';
  });
  map.on('error', (event) => {
    console.error('map error', event.error ?? event);
    if (isBasemapError(event, OVERLAYS)) show('basemap-error', true);
  });
}

function showLegend(bands) {
  const box = document.getElementById('legend');
  box.replaceChildren();
  // `bands` is already in frequency order (HR-2): never re-sort it.
  for (const { band, color } of bands) {
    const row = element('div');
    const swatch = element('span', 'swatch');
    swatch.style.background = color;
    row.append(swatch, document.createTextNode(band));
    box.append(row);
  }
  box.hidden = bands.length === 0;
}

function onHello(hello) {
  if (!map) createMap(hello);
  showLegend(hello.bands);
  state = createState({ bands: hello.bands, fallbackColor: hello.fallback_color, limit: connection.limit });
  for (const message of handshake(connection)) {
    socket.send(JSON.stringify(message));
  }
}

function onMessage(message) {
  if (message.type === 'hello') {
    onHello(message);
    return;
  }
  if (!state) return;
  if (message.type === 'snapshot') {
    setLimit(state, message.limit);
    applySnapshot(state, message.qsos);
  } else if (message.type === 'upsert') {
    applyUpsert(state, message.qso);
  } else {
    return;
  }
  render();
}

function connect() {
  const url = `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;
  socket = new WebSocket(url);
  socket.addEventListener('open', () => {
    onOpen(connection);
    show('status', false);
  });
  socket.addEventListener('message', (event) => {
    try {
      onMessage(JSON.parse(event.data));
    } catch (error) {
      console.error('bad message', error);
    }
  });
  socket.addEventListener('close', () => {
    show('status', true);
    setTimeout(connect, onClose(connection));
  });
}

function setupSelector() {
  const select = document.getElementById('limit');
  select.value = connection.limit === null ? 'all' : String(connection.limit);
  select.addEventListener('change', () => {
    connection.limit = select.value === 'all' ? null : Number.parseInt(select.value, 10);
    saveLimit(sessionStore(), connection.limit);
    if (socket && socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({ type: 'set_limit', limit: connection.limit }));
    }
  });
}

if (hasWebGL(() => document.createElement('canvas'))) {
  setupSelector();
  connect();
} else {
  // FR-012: no renderer can run, so do not connect either.
  show('map', false);
  show('no-webgl', true);
}
