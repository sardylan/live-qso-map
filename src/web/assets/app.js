// DOM and Leaflet glue only: every decision is made in map-state.js and geodesic.js.
// Popups are built with DOM nodes and textContent, never HTML strings, because QSO data comes
// from an open UDP port (FR-035).
/* global L */

import { createState, applySnapshot, applyUpsert, markers, diffLayers } from './map-state.js';
import { greatCircle } from './geodesic.js';

let map = null;
let legend = null;
let state = null;
let current = new Map();
const layers = new Map();

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

function icon(color) {
  const dot = element('span');
  dot.style.background = color;
  return L.divIcon({ className: 'qso-dot', html: dot, iconSize: [14, 14], iconAnchor: [7, 7] });
}

function linePoints(marker) {
  return marker.station ? greatCircle([marker.station.lat, marker.station.lon], [marker.lat, marker.lon], 64) : null;
}

function addLayer(marker) {
  const leafletMarker = L.marker([marker.lat, marker.lon], {
    icon: icon(marker.color),
    zIndexOffset: marker.zRank * 1000,
  }).bindPopup(() => popupContent(layers.get(marker.key).data));
  leafletMarker.addTo(map);
  const points = linePoints(marker);
  const line = points ? L.polyline(points, { color: marker.color, weight: 1.5, opacity: 0.7, interactive: false }).addTo(map) : null;
  layers.set(marker.key, { marker: leafletMarker, line, data: marker });
}

function updateLayer(marker) {
  const layer = layers.get(marker.key);
  if (!layer) return addLayer(marker);
  layer.data = marker;
  layer.marker.setIcon(icon(marker.color));
  layer.marker.setZIndexOffset(marker.zRank * 1000);
  if (layer.marker.isPopupOpen()) layer.marker.setPopupContent(popupContent(marker));
  const points = linePoints(marker);
  if (layer.line && points) {
    layer.line.setLatLngs(points);
    layer.line.setStyle({ color: marker.color });
  } else if (layer.line) {
    layer.line.remove();
    layer.line = null;
  } else if (points) {
    layer.line = L.polyline(points, { color: marker.color, weight: 1.5, opacity: 0.7, interactive: false }).addTo(map);
  }
}

function removeLayer(key) {
  const layer = layers.get(key);
  if (!layer) return;
  layer.marker.remove();
  if (layer.line) layer.line.remove();
  layers.delete(key);
}

function render() {
  const next = markers(state);
  const { add, update, remove } = diffLayers(current, next);
  remove.forEach(removeLayer);
  add.forEach(addLayer);
  update.forEach(updateLayer);
  current = next;
}

function showLegend(bands) {
  if (legend) legend.remove();
  legend = L.control({ position: 'bottomright' });
  legend.onAdd = () => {
    const box = element('div', 'legend');
    for (const [band, color] of Object.entries(bands)) {
      const row = element('div');
      const swatch = element('span', 'swatch');
      swatch.style.background = color;
      row.append(swatch, document.createTextNode(band));
      box.append(row);
    }
    return box;
  };
  legend.addTo(map);
}

function onHello(hello) {
  if (!map) {
    map = L.map('map', { worldCopyJump: true }).setView([30, 0], 2);
    L.tileLayer(hello.tiles.url, { attribution: hello.tiles.attribution, maxZoom: 18 }).addTo(map);
  }
  showLegend(hello.bands);
  state = createState({ bands: hello.bands, fallbackColor: hello.fallback_color, limit: hello.default_limit });
}

function onMessage(message) {
  if (message.type === 'hello') {
    onHello(message);
    return;
  }
  if (!state) return;
  if (message.type === 'snapshot') {
    state.limit = message.limit;
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
  const socket = new WebSocket(url);
  socket.addEventListener('message', (event) => {
    try {
      onMessage(JSON.parse(event.data));
    } catch (error) {
      console.error('bad message', error);
    }
  });
  return socket;
}

connect();
