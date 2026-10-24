// DOM and MapLibre glue only: every decision is made in map-state.js, layers.js, view.js,
// geodesic.js, connection.js, qth.js, autozoom.js, autozoom-geo.js, recent.js and score.js.
// Popups are built with DOM nodes and textContent, never HTML strings, because QSO data comes
// from an open UDP port (FR-035). Only the operator-configured attribution is HTML.
/* global maplibregl */

import {
  createState,
  applySnapshot,
  applyUpsert,
  applyRemove,
  markers,
  diffLayers,
  setLimit,
  setMapLimit,
  serverLimit,
  setShownBands,
} from './map-state.js';
import { markerFeatures, lineFeatures } from './layers.js';
import { currentQth, houseFeature, HOUSE_SVG } from './qth.js';
import {
  buildStyle,
  hasWebGL,
  isBasemapError,
  shouldRender,
  loadProjection,
  saveProjection,
  nextProjection,
  popupToClose,
  loadAutoZoom,
  globeBorder,
  saveAutoZoom,
  loadRecent,
  saveRecent,
  scaleVisible,
  loadRecentSize,
  saveRecentSize,
  loadRecentOrder,
  saveRecentOrder,
  basemapFor,
  DEFAULT_STYLE_URL,
  loadAtmosphere,
  saveAtmosphere,
  loadGrayline,
  saveGrayline,
  loadSunMarker,
  saveSunMarker,
  skyFor,
  lightFor,
} from './view.js';
import { PALETTES, applyPalette, resolveTheme, loadTheme, saveTheme, themeStatus } from './themes.js';
import { solarElevation, subsolarPoint } from './sun.js';
import { tilePixels, graylineBeforeId, graylineTiles, parseGraylineUrl, rgbOf } from './grayline.js';
import { recentQsos, headerCells, rowCells, markerKeyOf, entityLabel, onMap } from './recent.js';
import { scoreText, isStale, updatedTitle } from './score.js';
import { stationText, STATION_STALE_MS, lastQsoTitle, localQsoTime } from './station.js';
import { createAutoZoom } from './autozoom.js';
import {
  framedPoints,
  fallbackFrame,
  flatBounds,
  paddingFor,
  boundingCap,
  globeZoomForCap,
  flatFrameAround,
} from './autozoom-geo.js';
import { createConnection, onOpen, onClose, onHello as handshake, loadLimit, saveLimit } from './connection.js';
import * as panelState from './panel.js';
import { offeredGroups, chosenBands, loadLegend, saveLegend, toggleGroup } from './legend.js';

const OVERLAYS = ['qso-lines', 'qth', 'qso-markers'];
const QTH_POPUP = 'qth';
// Everything auto-zoom must keep clear of (FR-009); hidden ones measure as empty.
const PANELS = [
  '#legend',
  '#recent',
  '#score',
  '#station',
  '#panel',
  '.maplibregl-ctrl-top-left',
  '.maplibregl-ctrl-top-right',
  '.maplibregl-ctrl-bottom-left',
  '.maplibregl-ctrl-bottom-right',
  '#status',
  '#basemap-error',
];
const FIT = { gap: 12, minFree: [240, 160] };
const MAX_ZOOM = 12;
const POPUP_IDLE_MS = 30_000;
const WORLD = { center: [0, 30], zoom: 1.5 };

let map = null;
let loaded = false;
let state = null;
let current = new Map();
let helloStation = null;
let entity = null;
// Degrees of arc every marker keeps inside the globe's horizon (feature 016), from hello.
let globeBorderDeg = globeBorder(null);
// The operator's box columns and header (feature 006), from hello.
let recentConfig = { columns: ['call', 'band', 'time', 'cqzone', 'country'], header: true };
let qth = null;
let socket = null;
// The open popup and the key of its marker, so removals and updates reach it (FR-012).
let openPopup = null;
let openKey = null;
let popupTimer = null;

function localStore() {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

// Every viewer setting lives in local storage and survives a browser restart (Principle IV).
const connection = createConnection(loadLimit(localStore()));
let projection = loadProjection(localStore());
// What the map shows: the preference, or flat while auto-zoom has switched it (FR-016).
let shown = projection;
let projectionControl = null;
let scaleControl = null;
const autozoom = createAutoZoom({ enabled: loadAutoZoom(localStore()) });
let autoZoomControl = null;
let recentShown = loadRecent(localStore());
let recentSize = loadRecentSize(localStore());
let recentOrder = loadRecentOrder(localStore());
const OFF_MAP_TITLE = 'Not on the map (outside its last N)';
// The latest contest score, with its receive time on this page's clock (feature 007, R4).
let scoreState = null;
// Band and operator of the latest QSO (feature 017); null until the service sends them.
let stationState = null;
// How often the stale look is re-checked (SC-003: within 10 s).
const SCORE_TICK_MS = 5_000;
// How far inside the free area a marker opened from the box is brought (FR-022a).
const PAN_INSET_PX = 40;
// The theme (feature 011): the viewer's choice, the theme shown, and what decides it.
let themeChoice = loadTheme(localStore());
let currentTheme = null;
let basemap = null;
// The style URL the map uses when the page picks it (default base map); null otherwise.
let activeStyleUrl = null;
const darkQuery = typeof window.matchMedia === 'function' ? window.matchMedia('(prefers-color-scheme: dark)') : null;
// The automatic theme is re-checked this often (FR-003: within 60 s).
const THEME_TICK_MS = 60_000;
// The globe's atmosphere (feature 011, FR-010): on unless the viewer switched it off.
let atmosphere = loadAtmosphere(localStore());
// The grayline (on by default) and the Sun symbol (off by default), feature 013 (FR-009, FR-018).
let grayline = loadGrayline(localStore());
let sunMarker = loadSunMarker(localStore());
// The Legend group (feature 011, US4): the groups from `hello`, every considered band (`hello.bands`,
// frequency order) and the viewer's choice, `all` or group names.
let bandGroups = [];
let helloBands = [];
let legendChoice = 'all';

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
      element('td', 'entity', entityLabel(qso.entity)),
      element('td', 'meta', qso.remote ? `${qso.remote.source} · ${qso.remote.precision}` : ''),
    );
    table.append(row);
  }
  box.append(table);
  return box;
}

function formatPosition(here) {
  if (here.locator) return here.locator;
  return `${here.lat.toFixed(4)}, ${here.lon.toFixed(4)}`;
}

/** The house popup (FR-006): plain text only. */
function qthPopupContent(here) {
  const box = element('div', 'qso-popup qth-popup');
  box.append(
    element('div', 'call', here.callsign || 'station'),
    element('div', null, formatPosition(here)),
    element('div', 'meta', `${here.source} · ${here.precision}`),
  );
  return box;
}

function qthKey(here) {
  return here ? `${here.lat},${here.lon},${here.callsign},${here.locator},${here.source}` : null;
}

function show(id, visible) {
  document.getElementById(id).hidden = !visible;
}

/** Draws HOUSE_SVG at 2x into an image MapLibre can use as an icon (research R3). */
function addHouseImage() {
  const size = 28;
  const image = new Image(size, size);
  image.onload = () => {
    const canvas = document.createElement('canvas');
    canvas.width = size * 2;
    canvas.height = size * 2;
    const context = canvas.getContext('2d');
    context.drawImage(image, 0, 0, size * 2, size * 2);
    if (!map.hasImage('qth-house')) {
      map.addImage('qth-house', context.getImageData(0, 0, size * 2, size * 2), { pixelRatio: 2 });
    }
  };
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(HOUSE_SVG)}`;
}

// The grayline's tiles, drawn in the page from the minute's Sun and the theme's night colour.
maplibregl.addProtocol('grayline', async ({ url }) => {
  const tile = parseGraylineUrl(url);
  if (!tile) throw new Error(`not a grayline tile: ${url}`);
  const palette = PALETTES[tile.theme] ?? PALETTES.light;
  const size = 256;
  const subsolar = subsolarPoint(new Date(tile.minute * 60_000));
  const pixels = tilePixels(tile.z, tile.x, tile.y, size, subsolar, rgbOf(palette.night), palette.nightOpacity);
  return { data: await createImageBitmap(new ImageData(pixels, size, size)) };
});

/** The Sun symbol (feature 013, research R10): a yellow disc with rays and a dark outline. */
function addSunImage() {
  if (map.hasImage('sun-icon')) return;
  const size = 32;
  const canvas = document.createElement('canvas');
  canvas.width = size * 2;
  canvas.height = size * 2;
  const context = canvas.getContext('2d');
  context.scale(2, 2);
  context.translate(size / 2, size / 2);
  context.lineCap = 'round';
  for (let i = 0; i < 8; i++) {
    const a = (i * Math.PI) / 4;
    context.beginPath();
    context.moveTo(Math.cos(a) * 9, Math.sin(a) * 9);
    context.lineTo(Math.cos(a) * 14, Math.sin(a) * 14);
    context.strokeStyle = '#5a3d00';
    context.lineWidth = 4;
    context.stroke();
    context.strokeStyle = '#ffcc00';
    context.lineWidth = 2;
    context.stroke();
  }
  context.beginPath();
  context.arc(0, 0, 7, 0, 2 * Math.PI);
  context.fillStyle = '#ffcc00';
  context.fill();
  context.strokeStyle = '#5a3d00';
  context.lineWidth = 1.5;
  context.stroke();
  map.addImage('sun-icon', context.getImageData(0, 0, size * 2, size * 2), { pixelRatio: 2 });
}

function addOverlays() {
  const empty = { type: 'FeatureCollection', features: [] };
  // The base map's own layers, before ours: the grayline goes under its borders and names.
  const basemapBefore = graylineBeforeId(map.getStyle().layers);
  map.addSource('qso-lines', { type: 'geojson', data: empty });
  map.addLayer({
    id: 'qso-lines',
    type: 'line',
    source: 'qso-lines',
    layout: { 'line-cap': 'round' },
    // Black from the QTH (FR-004); slight transparency keeps overlapping paths apart.
    paint: { 'line-color': PALETTES[currentTheme ?? 'light'].line, 'line-width': 1.5, 'line-opacity': 0.6 },
  });
  // Order: lines, then the house, then the markers, which stay clickable on top (FR-005).
  addHouseImage();
  map.addSource('qth', { type: 'geojson', data: empty });
  map.addLayer({
    id: 'qth',
    type: 'symbol',
    source: 'qth',
    layout: { 'icon-image': 'qth-house', 'icon-allow-overlap': true, 'icon-ignore-placement': true },
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
  // The grayline (feature 013, research R7–R9): raster tiles the page draws (`grayline://`),
  // above land, sea and roads, below the base map's borders and names, and always below the QSO
  // layers. No events are bound to it.
  map.addSource('grayline', {
    type: 'raster',
    tiles: graylineTiles(currentTheme ?? 'light', new Date()),
    tileSize: 256,
    // The fade spans hundreds of kilometres: deeper zooms stretch zoom-6 tiles.
    maxzoom: 6,
  });
  map.addLayer(
    { id: 'grayline', type: 'raster', source: 'grayline', paint: { 'raster-fade-duration': 0 } },
    basemapBefore ?? 'qso-lines',
  );
  // The Sun symbol (FR-018): just below the QSO layers, never clickable, never hidden by labels.
  addSunImage();
  map.addSource('sun', { type: 'geojson', data: empty });
  map.addLayer(
    {
      id: 'sun',
      type: 'symbol',
      source: 'sun',
      layout: { 'icon-image': 'sun-icon', 'icon-allow-overlap': true, 'icon-ignore-placement': true },
    },
    'qso-lines',
  );
}

/** A map control with one button that switches between globe and flat (FR-010). */
class ProjectionControl {
  onAdd() {
    this.container = element('div', 'maplibregl-ctrl maplibregl-ctrl-group projection-control');
    this.button = element('button');
    this.button.type = 'button';
    this.button.addEventListener('click', () => {
      // The button switches the view on screen, whatever auto-zoom chose, and saves the choice.
      projection = nextProjection(shown);
      saveProjection(localStore(), projection);
      runActions(autozoom.projectionChosen());
      // Before `load` the choice is only stored; the load handler applies it.
      showProjection(projection);
      this.update();
    });
    this.container.append(this.button);
    this.update();
    return this.container;
  }

  /** The label offers the other view than the one on screen, automatic switches included. */
  update() {
    const target = nextProjection(shown) === 'globe' ? 'Globe' : 'Flat';
    this.button.textContent = target;
    this.button.title = autozoom.state.autoFlat
      ? 'Auto-zoom switched to flat because the QSOs do not fit on the globe'
      : `Switch to ${target.toLowerCase()} view`;
  }

  onRemove() {
    this.container.remove();
  }
}

/** A map control with one toggle button for auto-zoom (FR-007). */
class AutoZoomControl {
  onAdd() {
    this.container = element('div', 'maplibregl-ctrl maplibregl-ctrl-group toggle-control');
    this.button = element('button', null, 'Auto-zoom');
    this.button.type = 'button';
    this.button.addEventListener('click', () => runActions(autozoom.toggle(Date.now())));
    this.container.append(this.button);
    this.update();
    return this.container;
  }

  update() {
    const on = autozoom.state.enabled;
    this.button.setAttribute('aria-pressed', String(on));
    this.button.classList.toggle('active', on);
    const label = on ? 'Auto-zoom is on: click to stop following the QSOs' : 'Auto-zoom is off: click to show every QSO';
    this.button.title = label;
    this.button.setAttribute('aria-label', label);
  }

  onRemove() {
    this.container.remove();
  }
}

/** The gear button that opens and closes the settings panel (feature 011, FR-005). */
class SettingsControl {
  onAdd() {
    this.container = element('div', 'maplibregl-ctrl maplibregl-ctrl-group settings-control');
    this.button = element('button', null, '⚙');
    this.button.type = 'button';
    this.button.setAttribute('aria-label', 'Settings');
    this.button.setAttribute('aria-controls', 'panel');
    this.button.setAttribute('aria-expanded', 'false');
    this.button.title = 'Settings';
    this.button.addEventListener('click', () => applyPanel(panelState.toggle(settingsPanel, Date.now())));
    this.container.append(this.button);
    return this.container;
  }

  onRemove() {
    this.container.remove();
  }
}

const settingsPanel = panelState.createPanel();
let settingsControl = null;
// How often an idle open panel is checked for its 5-minute auto-close (FR-007).
const PANEL_TICK_MS = 15_000;

/** Shows or hides the panel after a state change, and moves focus (FR-008). */
function applyPanel(result) {
  if (!result) return;
  const panel = document.getElementById('panel');
  panel.hidden = !result.open;
  if (settingsControl) settingsControl.button.setAttribute('aria-expanded', String(result.open));
  if (result.open) {
    const first = panel.querySelector('select, input, button:not(.panel-close)');
    if (first) first.focus();
  } else if (result.focusGear && settingsControl) {
    settingsControl.button.focus();
  }
  // The panel covers part of the map: auto-zoom keeps clear of it while it is open.
  autozoom.change();
}

function setupPanel() {
  const panel = document.getElementById('panel');
  document.getElementById('panel-close').addEventListener('click', () => applyPanel(panelState.close(settingsPanel)));
  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || !settingsPanel.open) return;
    const inside = panel.contains(document.activeElement);
    const onGear = settingsControl && document.activeElement === settingsControl.button;
    if (inside || onGear) applyPanel(panelState.close(settingsPanel));
  });
  for (const kind of ['pointerdown', 'keydown', 'focusin', 'input', 'wheel']) {
    panel.addEventListener(kind, () => panelState.activity(settingsPanel, Date.now()));
  }
  setInterval(() => applyPanel(panelState.tick(settingsPanel, Date.now())), PANEL_TICK_MS);
}

/** Opens a marker's popup from the box, panning only when the marker is hidden (FR-022a). */
function openFromBox(markerKey) {
  const marker = current.get(markerKey);
  if (!marker || !loaded) return;
  const rect = map.getContainer().getBoundingClientRect();
  const padding = paddingFor(rect, panelRects(), FIT);
  const point = map.project([marker.lon, marker.lat]);
  const left = padding.left + PAN_INSET_PX;
  const right = rect.width - padding.right - PAN_INSET_PX;
  const top = padding.top + PAN_INSET_PX;
  const bottom = rect.height - padding.bottom - PAN_INSET_PX;
  const dx = point.x < left ? point.x - left : point.x > right ? point.x - right : 0;
  const dy = point.y < top ? point.y - top : point.y > bottom ? point.y - bottom : 0;
  // A page-made move carries no browser event, so auto-zoom stays on (FR-011).
  if (dx !== 0 || dy !== 0) map.panBy([dx, dy], { duration: 400 });
  openTrackedPopup([marker.lon, marker.lat], popupContent(marker), markerKey);
}

/** The last-QSO box: plain text only (FR-019, FR-020, FR-022). */
function renderRecent() {
  const box = document.getElementById('recent');
  box.hidden = !recentShown || !state;
  if (box.hidden) return;
  const qsos = recentQsos(state, recentSize, recentOrder);
  if (qsos.length === 0) {
    // The empty-state message replaces the header too (feature 006, FR-006a).
    box.replaceChildren(element('div', 'empty', 'No QSOs received yet'));
    return;
  }
  // Columns and header come from the operator's configuration (feature 006, FR-004).
  const table = element('table');
  if (recentConfig.header) {
    const head = element('thead');
    const tr = element('tr');
    for (const { column, text } of headerCells(recentConfig.columns)) tr.append(element('th', column, text));
    head.append(tr);
    table.append(head);
  }
  const body = element('tbody');
  // From the state, not the drawn layers, so rows are right even before the map has loaded.
  const onMapMarkers = markers(state);
  for (const qso of qsos) {
    const tr = element('tr');
    for (const { column, text, title } of rowCells(qso, recentConfig.columns)) {
      const td = element('td', column, text);
      if (title) td.title = title;
      tr.append(td);
    }
    const markerKey = markerKeyOf(qso);
    if (!onMap(qso, onMapMarkers)) {
      // Outside the map's last N, or no position: faded and not clickable (FR-010).
      if (markerKey !== null) {
        tr.className = 'off-map';
        tr.title = OFF_MAP_TITLE;
      }
    } else {
      tr.className = 'clickable';
      tr.tabIndex = 0;
      tr.setAttribute('role', 'button');
      tr.title = `Show ${qso.call ?? ''} on the map`;
      tr.addEventListener('click', () => openFromBox(markerKey));
      tr.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          openFromBox(markerKey);
        }
      });
    }
    body.append(tr);
  }
  table.append(body);
  box.replaceChildren(table);
}

/** The score line: plain text only (FR-005, FR-007, FR-008). */
function renderScore() {
  const line = document.getElementById('score');
  const text = scoreText(scoreState);
  // Only real changes are written, so the live region doesn't repeat itself every tick.
  if (line.textContent !== text) {
    line.textContent = text;
    // Its width may have changed: auto-zoom must keep clear of it (FR-013).
    autozoom.change();
  }
  line.classList.toggle('stale', scoreState !== null && isStale(scoreState.localReceived, Date.now()));
  line.title = scoreState ? updatedTitle(scoreState.receivedAt) : '';
}

function onScore(message) {
  scoreState = {
    score: message.score,
    qsos: message.qsos,
    mults: message.mults,
    receivedAt: message.received_at,
    // Staleness is judged on this page's clock, whatever the service's clock says (R4).
    localReceived: Date.now() - (Number(message.age_s) || 0) * 1000,
  };
  renderScore();
}

/** The station status line: plain text only (feature 017, FR-001). */
function renderStation() {
  const line = document.getElementById('station');
  const text = stationText(stationState);
  if (line.textContent !== text) {
    line.textContent = text;
    // Its width may have changed: auto-zoom must keep clear of it.
    autozoom.change();
  }
  // Greyed 15 minutes after the QSO, judged on this page's clock from the service's age (R5).
  const localQso = stationState?.localQso ?? null;
  line.classList.toggle('stale', localQso !== null && isStale(localQso, Date.now(), STATION_STALE_MS));
  line.title = lastQsoTitle(stationState?.qsoAt);
}

function onStationStatus(message) {
  stationState = {
    band: message.band ?? null,
    operator: message.operator ?? null,
    qsoAt: message.qso_at ?? null,
    localQso: localQsoTime(message.age_s ?? null, Date.now()),
  };
  renderStation();
}

function runActions(actions) {
  for (const action of actions) {
    if (action.type === 'save') {
      saveAutoZoom(localStore(), action.enabled);
      if (autoZoomControl) autoZoomControl.update();
    } else if (action.type === 'fit') {
      fitNow();
    }
  }
}

function panelRects() {
  return PANELS.flatMap((selector) => [...document.querySelectorAll(selector)])
    .filter((node) => !node.hidden && node.offsetParent !== null)
    .map((node) => node.getBoundingClientRect());
}

/** A metric scale at the bottom, only on the flat map (FR-023, FR-024). */
function updateScale() {
  if (!map) return;
  const wanted = scaleVisible(shown);
  if (wanted && !scaleControl) {
    scaleControl = new maplibregl.ScaleControl({ unit: 'metric', maxWidth: 120 });
    map.addControl(scaleControl, 'bottom-left');
  } else if (!wanted && scaleControl) {
    map.removeControl(scaleControl);
    scaleControl = null;
  }
  // The last-QSO box sits above the scale.
  document.body.classList.toggle('scale-on', wanted);
}

function showProjection(next) {
  if (next === shown) return;
  shown = next;
  if (loaded) map.setProjection({ type: shown });
  applySky();
  if (projectionControl) projectionControl.update();
  updateScale();
}

/** `[lat, lon]` points of what auto-zoom frames: the markers and QTH, or the fallback box corners. */
function framePoints(bounds) {
  if (current.size > 0) return framedPoints(qth, current);
  if (!bounds) return [];
  const [[w, s], [e, n]] = bounds;
  return [[s, w], [s, e], [n, w], [n, e], [(s + n) / 2, (w + e) / 2]];
}

/** What auto-zoom frames now: flat bounds, the bounding cap and the globe zoom that shows it. */
function currentFrame() {
  const rect = map.getContainer().getBoundingClientRect();
  const padding = paddingFor(rect, panelRects(), FIT);
  const bounds = current.size > 0 ? flatBounds(framedPoints(qth, current)) : fallbackFrame(qth, entity);
  const points = framePoints(bounds);
  const cap = points.length > 0 ? boundingCap(points) : null;
  // The globe keeps the frame only while every marker stays the border inside its horizon (016).
  const globeZoom = cap
    ? globeZoomForCap(cap, { width: rect.width, height: rect.height, padding, borderDeg: globeBorderDeg, maxZoom: MAX_ZOOM })
    : null;
  return { rect, padding, bounds, cap, globeZoom, fitsGlobe: !cap || globeZoom !== null };
}

/** Frames the house and every marker clear of the panels (FR-008–FR-010, FR-013, FR-015–FR-017). */
function fitNow() {
  if (!loaded) return;
  const { rect, padding, bounds, cap, globeZoom, fitsGlobe: fits } = currentFrame();
  // The free area this fit used, for the browser tests (feature 016): "top,right,bottom,left".
  document.getElementById('map').dataset.padding = [padding.top, padding.right, padding.bottom, padding.left].join(',');
  showProjection(autozoom.fitted(fits, projection));
  try {
    if (!bounds) {
      map.easeTo({ ...WORLD, padding, duration: 800 });
    } else if (shown === 'globe') {
      const [lat, lon] = cap.center;
      map.easeTo({ center: [lon, lat], zoom: globeZoom, padding, duration: 800 });
    } else if (qth) {
      // The flat view centres on the house (feature 016 FR-001).
      const size = { width: rect.width, height: rect.height, padding, maxZoom: MAX_ZOOM };
      const { center, zoom } = flatFrameAround(qth, framePoints(bounds), size);
      map.easeTo({ center, zoom, padding, duration: 800 });
    } else {
      map.fitBounds(bounds, { padding, maxZoom: MAX_ZOOM, duration: 800 });
    }
  } catch (error) {
    console.error('auto-zoom fit failed', error);
  }
}

function render() {
  if (!state) return;
  const next = markers(state);
  const diff = diffLayers(current, next);
  const nextQth = currentQth(helloStation, state);
  const qthChanged = qthKey(nextQth) !== qthKey(qth);
  if (!shouldRender(loaded, diff) && !(loaded && qthChanged)) return;
  map.getSource('qso-markers').setData(markerFeatures(next));
  map.getSource('qso-lines').setData(lineFeatures(next, nextQth));
  if (qthChanged) map.getSource('qth').setData(houseFeature(nextQth));
  current = next;
  // How many markers are drawn, for the browser tests (feature 012, US3 scenarios 1 and 5).
  document.getElementById('map').dataset.markers = String(next.size);
  qth = nextQth;
  // The sun mode follows the house marker: a new position may change the theme (FR-003).
  if (qthChanged) updateTheme();
  if (openKey === QTH_POPUP) {
    if (!qth) openPopup.remove();
    else if (qthChanged) openPopup.setLngLat([qth.lon, qth.lat]).setDOMContent(qthPopupContent(qth));
  } else if (popupToClose(openKey, diff)) {
    openPopup.remove();
  } else if (openKey !== null && diff.update.some((m) => m.key === openKey)) {
    openPopup.setDOMContent(popupContent(current.get(openKey)));
  }
}

/** While auto-zoom is on, a popup nobody touches closes after 30 s (FR-012a). */
function armPopupTimer(popup) {
  clearTimeout(popupTimer);
  popupTimer = setTimeout(() => {
    if (openPopup !== popup) return;
    if (autozoom.state.enabled) popup.remove();
    else armPopupTimer(popup);
  }, POPUP_IDLE_MS);
}

function openTrackedPopup(lngLat, content, key) {
  if (openPopup) openPopup.remove();
  const popup = new maplibregl.Popup({ maxWidth: 'none' }).setLngLat(lngLat).setDOMContent(content).addTo(map);
  popup.on('close', () => {
    if (openPopup === popup) {
      openPopup = null;
      openKey = null;
      clearTimeout(popupTimer);
      runActions(autozoom.popupClosed());
    }
  });
  for (const kind of ['pointerdown', 'pointermove', 'wheel', 'keydown']) {
    popup.getElement().addEventListener(kind, () => armPopupTimer(popup));
  }
  openPopup = popup;
  openKey = key;
  runActions(autozoom.popupOpened());
  armPopupTimer(popup);
}

/** What the automatic theme looks at: the device preference, and the sun at the house marker. */
function themeInputs() {
  const station = currentQth(helloStation, state);
  return {
    prefersDark: darkQuery ? darkQuery.matches : false,
    sunElevation: station ? solarElevation(new Date(), station.lat, station.lon) : null,
    place: station ? station.locator || station.callsign : null,
  };
}

/** Picks the theme and applies it to the page and the map when it changes (FR-001–FR-003). */
function updateTheme() {
  const inputs = themeInputs();
  const status = document.getElementById('theme-status');
  if (status) status.textContent = themeStatus(themeChoice, inputs);
  const theme = resolveTheme(themeChoice, inputs);
  if (theme === currentTheme) return;
  currentTheme = theme;
  applyPalette(document.documentElement, theme);
  applyBasemap();
}

/**
 * The default base map switches style with the theme; any other is dimmed instead (research
 * R4). A style switch drops every layer: the `style.load` handler adds them back. The camera
 * is not touched, so the view stays as it is.
 */
function applyBasemap() {
  if (!map || !basemap || !currentTheme) return;
  const { styleUrl } = basemapFor(basemap, PALETTES[currentTheme], DEFAULT_STYLE_URL);
  if (styleUrl && styleUrl !== activeStyleUrl) {
    activeStyleUrl = styleUrl;
    loaded = false;
    map.setStyle(styleUrl, { diff: false });
    return;
  }
  applyOverlayTheme();
}

/** The theme's colours on the page's own layers, and the dimming layer for other base maps. */
function applyOverlayTheme() {
  if (!loaded || !currentTheme) return;
  const palette = PALETTES[currentTheme];
  map.setPaintProperty('qso-lines', 'line-color', palette.line);
  applySky();
  applySun();
  const { dimOpacity } = basemapFor(basemap, palette, DEFAULT_STYLE_URL);
  if (dimOpacity > 0) {
    if (!map.getLayer('theme-dim')) {
      // A full background layer above the base map and below the QSO layers: the markers keep
      // their colours.
      map.addLayer(
        { id: 'theme-dim', type: 'background', paint: { 'background-color': '#000000', 'background-opacity': dimOpacity } },
        'sun',
      );
    } else {
      map.setPaintProperty('theme-dim', 'background-opacity', dimOpacity);
    }
  } else if (map.getLayer('theme-dim')) {
    map.removeLayer('theme-dim');
  }
}

/** The sky and atmosphere for the theme and the projection on screen (FR-010, FR-011). */
function applySky() {
  if (!loaded || !currentTheme) return;
  map.setSky(skyFor(PALETTES[currentTheme], { atmosphere, projection: shown }));
}

/**
 * The light that makes the atmosphere's rim brightest towards the Sun (feature 013, FR-001–
 * FR-004). It depends on the camera, so it runs on every `move` (inside the animation frame,
 * before drawing: the glow never lags) and on the minute tick; `sun` caches the Sun's position.
 */
let sun = null;
function applyLight() {
  if (!loaded) return;
  sun ??= subsolarPoint(new Date());
  const { lat, lng } = map.getCenter();
  const light = lightFor(sun, { lat, lng, bearing: map.getBearing(), pitch: map.getPitch() });
  map.setLight(light);
  // The camera and the light of this frame, read together by the browser tests (step 10).
  const el = document.getElementById('map');
  el.dataset.camera = [lat, lng, map.getZoom()].map((v) => v.toFixed(4)).join(',');
  el.dataset.light = light.position.slice(1).map((v) => v.toFixed(2)).join(',');
}

/**
 * Everything that follows the Sun (feature 013): its position, the glow's light, the grayline's
 * caps and the Sun symbol, each shown or hidden by its choice. After every style load and on the
 * minute tick; moving the map needs only `applyLight`.
 */
function applySun() {
  if (!loaded) return;
  sun = subsolarPoint(new Date());
  applyLight();
  // A new minute or theme names new tiles; the old ones stay on screen until those are drawn.
  const tiles = graylineTiles(currentTheme ?? 'light', new Date());
  const source = map.getSource('grayline');
  if (source.tiles?.[0] !== tiles[0]) source.setTiles(tiles);
  map.getSource('sun').setData({ type: 'Feature', properties: {}, geometry: { type: 'Point', coordinates: [sun.lon, sun.lat] } });
  map.setLayoutProperty('grayline', 'visibility', grayline ? 'visible' : 'none');
  map.setLayoutProperty('sun', 'visibility', sunMarker ? 'visible' : 'none');
}

/** Everything the page adds to a style, again after every style switch. */
function onStyleLoad() {
  map.setProjection({ type: shown });
  addOverlays();
  loaded = true;
  current = new Map();
  qth = null;
  applyOverlayTheme();
  applySky();
  applySun();
  render();
  autozoom.change();
}

function createMap(hello) {
  const themed = basemapFor(hello.basemap, PALETTES[currentTheme ?? 'light'], DEFAULT_STYLE_URL);
  activeStyleUrl = themed.styleUrl;
  map = new maplibregl.Map({
    container: 'map',
    style: themed.styleUrl ?? buildStyle(hello.basemap),
    center: [0, 30],
    zoom: 1.5,
    attributionControl: false,
    renderWorldCopies: true,
  });
  map.addControl(
    new maplibregl.AttributionControl({ compact: false, customAttribution: hello.basemap.attribution || undefined }),
  );
  map.addControl(new maplibregl.NavigationControl(), 'top-left');
  projectionControl = new ProjectionControl();
  map.addControl(projectionControl, 'top-left');
  autoZoomControl = new AutoZoomControl();
  map.addControl(autoZoomControl, 'top-left');
  settingsControl = new SettingsControl();
  map.addControl(settingsControl, 'top-right');
  updateScale();
  // Fires for the first style and after every theme switch of the default base map.
  map.on('style.load', onStyleLoad);
  // Only camera moves the viewer makes carry the browser event; the page's own fits do not.
  // `movestart` fires for every camera change, including rotation and pitch (FR-011).
  // The settled view, for the browser tests (feature 012, US3 scenario 3): "lat,lon,zoom".
  map.on('moveend', () => {
    const { lat, lng } = map.getCenter();
    document.getElementById('map').dataset.view = [lat, lng, map.getZoom()].map((v) => v.toFixed(4)).join(',');
    // Where the centre lies on screen (padding moves it), for the glow checks (feature 013).
    const { x, y } = map.project([lng, lat]);
    document.getElementById('map').dataset.centre = `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  map.on('move', applyLight);
  map.on('movestart', (event) => {
    if (event.originalEvent) runActions(autozoom.userMove());
  });
  map.on('click', 'qso-markers', (event) => {
    const feature = event.features && event.features[0];
    const marker = feature && current.get(feature.properties.key);
    if (!marker) return;
    openTrackedPopup(feature.geometry.coordinates, popupContent(marker), feature.properties.key);
  });
  map.on('click', 'qth', (event) => {
    // A QSO marker over the house wins (FR-005).
    if (map.queryRenderedFeatures(event.point, { layers: ['qso-markers'] }).length > 0) return;
    if (!qth) return;
    openTrackedPopup([qth.lon, qth.lat], qthPopupContent(qth), QTH_POPUP);
  });
  for (const layer of ['qso-markers', 'qth']) {
    map.on('mouseenter', layer, () => {
      map.getCanvas().style.cursor = 'pointer';
    });
    map.on('mouseleave', layer, () => {
      map.getCanvas().style.cursor = '';
    });
  }
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

/** The bands the Legend choice shows, or `null` for every band. */
function shownBands() {
  return chosenBands(legendChoice, bandGroups, helloBands.map((b) => b.band));
}

/** Tells the server the window and the bands; the session keeps both until the next one. */
function sendSetLimit() {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify({ type: 'set_limit', limit: serverLimit(connection.limit), bands: shownBands() }));
}

/** The Legend group's checkboxes, rebuilt from `hello.band_groups` on every hello. */
function renderLegendGroup() {
  const all = document.getElementById('legend-all');
  all.checked = legendChoice === 'all';
  const box = document.getElementById('legend-groups');
  box.replaceChildren();
  for (const group of bandGroups) {
    const label = element('label');
    const input = element('input');
    input.type = 'checkbox';
    input.value = group.name;
    input.checked = legendChoice !== 'all' && legendChoice.includes(group.name);
    input.addEventListener('change', () => changeLegend(group.name));
    label.append(input, document.createTextNode(` ${group.text}`));
    box.append(label);
  }
  // An older server offers no groups: the group then has nothing to choose.
  document.getElementById('group-legend').hidden = bandGroups.length === 0;
}

/** Applies the Legend choice to the legend and the map state (FR-013). */
function applyLegend() {
  const bands = shownBands();
  if (state) setShownBands(state, bands);
  const wanted = bands === null ? null : new Set(bands);
  showLegend(wanted === null ? helloBands : helloBands.filter((b) => wanted.has(b.band)));
}

function changeLegend(name) {
  legendChoice = toggleGroup(legendChoice, name);
  saveLegend(localStore(), legendChoice);
  renderLegendGroup();
  applyLegend();
  if (state) {
    render();
    renderRecent();
    autozoom.change();
  }
  sendSetLimit();
}

function onHello(hello) {
  helloStation = hello.station ?? null;
  entity = hello.entity ?? null;
  globeBorderDeg = globeBorder(hello);
  basemap = hello.basemap;
  // The theme first, so the map starts with the right style (no light flash at night).
  updateTheme();
  if (!map) createMap(hello);
  helloBands = hello.bands;
  bandGroups = offeredGroups(hello.band_groups);
  legendChoice = loadLegend(localStore(), bandGroups.map((g) => g.name));
  renderLegendGroup();
  if (hello.recent && Array.isArray(hello.recent.columns)) recentConfig = hello.recent;
  state = createState({ bands: hello.bands, fallbackColor: hello.fallback_color, limit: connection.limit });
  applyLegend();
  renderRecent();
  // The service may have restarted and lost its score: show none until it sends one (R7).
  scoreState = null;
  renderScore();
  stationState = null;
  renderStation();
  // Ask for at least 20 QSOs, so the box has its rows whatever the map shows (TR-006).
  // The band choice too, so a reconnected session gets the same filtered snapshot (FR-013).
  for (const message of handshake({ limit: serverLimit(connection.limit), bands: shownBands() })) {
    socket.send(JSON.stringify(message));
  }
}

function onMessage(message) {
  if (message.type === 'hello') {
    onHello(message);
    return;
  }
  if (message.type === 'score') {
    onScore(message);
    return;
  }
  if (message.type === 'station_status') {
    onStationStatus(message);
    return;
  }
  if (!state) return;
  if (message.type === 'snapshot') {
    setLimit(state, message.limit);
    applySnapshot(state, message.qsos);
  } else if (message.type === 'upsert') {
    applyUpsert(state, message.qso);
  } else if (message.type === 'remove') {
    applyRemove(state, message.id);
  } else {
    return;
  }
  render();
  // After a snapshot too, so the box refills after a reconnection.
  renderRecent();
  autozoom.change();
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

/** The Appearance group: the theme choice and its status line (FR-001, contracts/page-ui.md). */
function setupAppearance() {
  const select = document.getElementById('theme');
  select.value = themeChoice;
  select.addEventListener('change', () => {
    themeChoice = select.value;
    saveTheme(localStore(), themeChoice);
    updateTheme();
  });
  if (darkQuery && typeof darkQuery.addEventListener === 'function') {
    darkQuery.addEventListener('change', updateTheme);
  }
  // The Sun moves 0.25° a minute: the theme and the glow follow it on the same tick.
  setInterval(() => {
    updateTheme();
    applySun();
  }, THEME_TICK_MS);
  updateTheme();
}

function setupSelector() {
  const atmosphereBox = document.getElementById('atmosphere');
  atmosphereBox.checked = atmosphere;
  atmosphereBox.addEventListener('change', () => {
    atmosphere = atmosphereBox.checked;
    saveAtmosphere(localStore(), atmosphere);
    applySky();
  });
  const graylineBox = document.getElementById('grayline');
  graylineBox.checked = grayline;
  graylineBox.addEventListener('change', () => {
    grayline = graylineBox.checked;
    saveGrayline(localStore(), grayline);
    applySun();
  });
  const sunBox = document.getElementById('sun-marker');
  sunBox.checked = sunMarker;
  sunBox.addEventListener('change', () => {
    sunMarker = sunBox.checked;
    saveSunMarker(localStore(), sunMarker);
    applySun();
  });

  document.getElementById('legend-all').addEventListener('change', () => changeLegend('all'));

  const select = document.getElementById('limit');
  select.value = String(connection.limit);
  select.addEventListener('change', () => {
    connection.limit = Number.parseInt(select.value, 10);
    saveLimit(localStore(), connection.limit);
    if (state) {
      // The map takes its own window at once; the box keeps its own (FR-009).
      setMapLimit(state, connection.limit);
      render();
      renderRecent();
      autozoom.change();
    }
    sendSetLimit();
  });

  // The box's show/hide switch, in the settings panel since feature 007 (FR-010), with the
  // feature-004 key, so a viewer's choice survives the move (Principle IV).
  const shownBox = document.getElementById('recent-shown');
  shownBox.checked = recentShown;
  shownBox.addEventListener('change', () => {
    recentShown = shownBox.checked;
    saveRecent(localStore(), recentShown);
    renderRecent();
    autozoom.change();
  });

  const size = document.getElementById('recent-size');
  size.value = String(recentSize);
  size.addEventListener('change', () => {
    recentSize = Number.parseInt(size.value, 10);
    saveRecentSize(localStore(), recentSize);
    renderRecent();
    autozoom.change();
  });
  const order = document.getElementById('recent-order');
  order.value = recentOrder;
  order.addEventListener('change', () => {
    recentOrder = order.value === 'top' ? 'top' : 'bottom';
    saveRecentOrder(localStore(), recentOrder);
    renderRecent();
  });
}

if (hasWebGL(() => document.createElement('canvas'))) {
  setupSelector();
  setupPanel();
  setupAppearance();
  connect();
  window.addEventListener('resize', () => autozoom.change());
  setInterval(() => runActions(autozoom.tick(Date.now())), 250);
  setInterval(() => {
    renderScore();
    renderStation();
  }, SCORE_TICK_MS);
} else {
  // FR-012: no renderer can run, so do not connect either.
  show('map', false);
  show('corner-bl', false);
  show('no-webgl', true);
}
