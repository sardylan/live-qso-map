// Browser tests for live-qso-map (feature 012, contracts/browser-test.md).
//
//   node tests/browser/run.mjs [--binary PATH] [--chrome PATH] [--out DIR] [--headed]
//
// Starts the program and Chrome, plays a short viewer session on the map page, and writes a
// report, a log and screenshots to the output folder. Exit 0 = pass, 1 = a step failed,
// 2 = bad usage or a missing/unstartable program or browser. Node built-ins only.

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, openSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createServer } from 'node:net';
import { createSocket } from 'node:dgram';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

import {
  parseArgs,
  chromeCandidates,
  findChrome,
  classifyEvent,
  createRun,
  startStep,
  finishStep,
  skipRest,
  addScreenshot,
  waitFor,
  qsoAdif,
  luminance,
  parseView,
  limbSamples,
  apparentRadiusPx,
  sunSide,
  testInstant,
  lightSide,
  toScreenMercator,
  toScreenGlobe,
  nightAndDayPoints,
} from './lib.mjs';
import { subsolarPoint } from '../../src/web/assets/sun.js';
import { launchChrome } from './cdp.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const CTY = path.join(ROOT, 'tests', 'fixtures', 'cty-excerpt.dat');
const LOAD_MS = 15_000;
const STEP_MS = 5_000;
const CONTEST_BANDS = ['160m', '80m', '40m', '20m', '15m', '10m'];
const STEPS = [
  'load',
  'qsos',
  'panel-open',
  'themes',
  'panel-escape',
  'reload-keeps-settings',
  'device-dark',
  'reconnect',
  'legend-group',
  'atmosphere',
  'grayline',
];
// The house of `--station-locator JM49OF` (the square's centre), for the test clock.
const HOUSE = { lat: 39.229, lon: 9.208 };

function usageExit(message) {
  console.error(message);
  process.exit(2);
}

const parsed = parseArgs(process.argv.slice(2), process.env, process.platform);
if (!parsed.ok) usageExit(parsed.error);
const options = parsed.options;
const binary = path.resolve(options.binary);
if (!existsSync(binary)) usageExit(`program not found: ${binary} (build it with cargo build --release, or pass --binary)`);
const chrome = options.chrome ?? findChrome(chromeCandidates(process.platform, process.env), existsSync);
if (!chrome || !existsSync(chrome)) usageExit('Chrome or Chromium not found: install one, or pass --chrome or set CHROME_PATH');
const out = path.resolve(options.out);
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
// The program's history file (feature 014) lives in the output folder: it never lands next to the
// cty.dat fixture, and it is kept as an artifact.
const HISTORY = path.join(out, 'qso-history.adi');
// Chrome's throwaway profile lives outside the output folder, so it never becomes an artifact.
const profileDir = mkdtempSync(path.join(tmpdir(), 'lqm-browser-test-'));

// --- the program --------------------------------------------------------------------------

async function freeTcpPort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

async function freeUdpPort() {
  return new Promise((resolve, reject) => {
    const socket = createSocket('udp4');
    socket.on('error', reject);
    socket.bind(0, '127.0.0.1', () => {
      const { port } = socket.address();
      socket.close(() => resolve(port));
    });
  });
}

let program = null;

async function startProgram(phase, extraArgs) {
  // No callbook, no account: every LQM_* setting is removed (FR-013).
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('LQM_')));
  const log = openSync(path.join(out, `program-${phase}.log`), 'w');
  const child = spawn(
    binary,
    [
      '--http-port', String(ports.http),
      '--udp-port', String(ports.udp),
      '--cty-path', CTY,
      '--cty-max-age-days', '0',
      '--history-file', HISTORY,
      '--station-callsign', 'IS0GVH',
      '--station-locator', 'JM49OF',
      ...extraArgs,
    ],
    { env, stdio: ['ignore', log, log] },
  );
  const exited = new Promise((resolve) => child.on('exit', resolve));
  program = { child, exited };
  await waitFor(
    async () => {
      if (child.exitCode !== null) throw new Error(`the program exited with ${child.exitCode} (see program-${phase}.log)`);
      try {
        return (await fetch(`${origin}/healthz`)).ok;
      } catch {
        return false;
      }
    },
    { timeout: 10_000, what: `the program (phase ${phase}) to answer /healthz` },
  );
}

async function stopProgram() {
  if (!program) return;
  const { child, exited } = program;
  program = null;
  if (child.exitCode === null) {
    child.kill('SIGTERM');
    const done = await Promise.race([exited.then(() => true), sleep(5_000).then(() => false)]);
    if (!done) child.kill('SIGKILL');
  }
}

const udp = createSocket('udp4');
let qsoSecond = 0;

/** Sends one QSO over UDP; each gets its own second, so times are distinct and ordered. */
async function sendQso({ call, band, grid }) {
  const at = new Date(Date.now() - 60_000 + qsoSecond++ * 1000);
  const record = Buffer.from(qsoAdif({ call, band, mode: 'CW', grid, at }));
  await new Promise((resolve, reject) => udp.send(record, ports.udp, '127.0.0.1', (e) => (e ? reject(e) : resolve())));
}

// --- the page -----------------------------------------------------------------------------

let page = null;
let seen = 0;
let allowWebSocketErrors = false;

/** Fatal events since the last call, as one error (research R5). */
function fatalSinceLastCheck() {
  const fresh = page.events.slice(seen);
  seen = page.events.length;
  const fatal = fresh.filter((event) => {
    event.fatal = classifyEvent(event, origin).fatal;
    // While the program is down on purpose (step 08), the page's reconnection attempts fail.
    if (allowWebSocketErrors && /WebSocket|\/ws\b/.test(`${event.text ?? ''} ${event.url ?? ''}`)) event.fatal = false;
    return event.fatal;
  });
  if (fatal.length === 0) return null;
  return new Error(fatal.map((e) => `${e.kind}: ${e.text ?? ''} ${e.url ?? ''} ${e.status ?? ''}`.trim()).join('; '));
}

const q = (expression) => page.eval(expression);
const until = (expression, what, timeout = STEP_MS) => waitFor(() => q(expression), { timeout, what });

const legendRows = () => q(`[...document.querySelectorAll('#legend > div')].map((d) => d.textContent.trim())`);
const boxCalls = () =>
  q(`[...document.querySelectorAll('#recent tbody tr')].map((tr) => tr.querySelector('td')?.textContent.trim())`);
const markerCount = () => q(`Number(document.getElementById('map').dataset.markers ?? 0)`);
const groupLabels = () => q(`[...document.querySelectorAll('#group-legend label')].map((l) => l.textContent.trim())`);

async function sameList(read, expected, what, timeout = STEP_MS) {
  await waitFor(async () => JSON.stringify(await read()) === JSON.stringify(expected), {
    timeout,
    what: `${what}: ${JSON.stringify(expected)}`,
  });
}

/** The page is connected, rendered, and its script ran: the 3.0.0 failure would stop here. */
async function pageReady(timeout = LOAD_MS) {
  await waitFor(
    async () => {
      const state = await q(`({
        maplibre: typeof maplibregl !== 'undefined',
        noWebgl: !document.getElementById('no-webgl').hidden,
        connected: document.getElementById('status').hidden,
        legend: !document.getElementById('legend').hidden,
        gear: !!document.querySelector('.settings-control button'),
      })`);
      if (state.noWebgl) throw new Error('WebGL unavailable: the page shows its no-WebGL message');
      const fatal = fatalSinceLastCheck();
      if (fatal) throw fatal;
      return state.maplibre && state.connected && state.legend && state.gear;
    },
    { timeout, what: 'the page to load, connect and draw its controls and legend' },
  ).catch(async (error) => {
    if (!(await q(`typeof maplibregl !== 'undefined'`).catch(() => false))) {
      throw new Error(`MapLibre could not be loaded from its CDN (${error.message})`);
    }
    throw error;
  });
}

/** `#map`'s data-view once it reads the same twice, 1 s apart (an animation isn't a move). */
async function stableView(what) {
  let previous = null;
  return waitFor(
    async () => {
      const view = await q(`document.getElementById('map').dataset.view ?? null`);
      const stable = view !== null && view === previous;
      previous = view;
      return stable ? view : null;
    },
    { timeout: 10_000, interval: 1_000, what: `the map view to settle ${what}` },
  );
}

function sameView(a, b) {
  const [la, oa, za] = a.split(',').map(Number);
  const [lb, ob, zb] = b.split(',').map(Number);
  return Math.abs(la - lb) <= 0.01 && Math.abs(oa - ob) <= 0.01 && Math.abs(za - zb) <= 0.01;
}

let currentStep = null;

/** Saves `<name>.png` and lists it on the running step (data-model Step.screenshot). */
async function shot(name) {
  await page.screenshot(path.join(out, `${name}.png`));
  if (currentStep) addScreenshot(currentStep, `${name}.png`);
}

async function setTheme(choice) {
  await q(`(() => { const s = document.getElementById('theme'); s.value = '${choice}'; s.dispatchEvent(new Event('change')); })()`);
}

// --- the Sun on the test clock (feature 013, contracts/browser-test.md) ---------------------

let clockOffset = 0;
const sunNow = () => subsolarPoint(new Date(Date.now() + clockOffset));
const mapRect = () => q(`(() => { const r = document.getElementById('map').getBoundingClientRect(); return { left: r.left, top: r.top, width: r.width, height: r.height }; })()`);
const centre = (rect) => [rect.left + rect.width / 2, rect.top + rect.height / 2];
/** The globe's centre in page pixels, from `#map`'s `data-centre` (map pixels). */
async function globeCentre(rect) {
  const text = await q(`document.getElementById('map').dataset.centre ?? ''`);
  const [x, y] = text.split(',').map(Number);
  return Number.isFinite(x) && Number.isFinite(y) ? [rect.left + x, rect.top + y] : null;
}

async function ensureGlobe() {
  if ((await q(`document.querySelector('.projection-control button').textContent`)) === 'Globe') {
    await q(`document.querySelector('.projection-control button').click()`);
  }
  await until(`document.querySelector('.projection-control button').textContent === 'Flat'`, 'the globe on screen');
}

async function setAutoZoom(on) {
  if ((await q(`document.querySelector('.toggle-control button').classList.contains('active')`)) !== on) {
    await q(`document.querySelector('.toggle-control button').click()`);
  }
  await until(`document.querySelector('.toggle-control button').classList.contains('active') === ${on}`, `auto-zoom ${on ? 'on' : 'off'}`);
}

/** Zooms out until both limb boxes fit the map. */
async function wholeGlobe() {
  for (let i = 0; i < 8; i++) {
    const view = parseView(await stableView('before measuring the limb'));
    const rect = await mapRect();
    if (limbSamples(view, rect, await globeCentre(rect))) return;
    await q(`document.querySelector('.maplibregl-ctrl-zoom-out').click()`);
  }
  throw new Error('the whole globe never fitted the map');
}

/** The glow's brighter limb against the Sun's side (step 10, items 2–5); returns the reading. */
async function glowOnSunSide(shotName, { minAbsX = 0.5 } = {}) {
  const viewText = await stableView(`for ${shotName}`);
  const view = parseView(viewText);
  const rect = await mapRect();
  const boxes = limbSamples(view, rect, await globeCentre(rect));
  if (!boxes) throw new Error(`the limb is off the map at ${viewText}`);
  const expected = sunSide(view, sunNow());
  const png = await page.capture(path.join(out, `${shotName}.png`));
  if (currentStep) addScreenshot(currentStep, `${shotName}.png`);
  const [left, right] = (await page.measure(png, boxes)).map(luminance);
  const brighter = left > right ? 'left' : 'right';
  const reading = `glow brighter on the ${brighter} (L ${left.toFixed(1)} / R ${right.toFixed(1)}), Sun on the ${expected.side} (x ${expected.x.toFixed(2)}), view ${viewText}`;
  console.log(`   ${shotName}: ${reading}`);
  if (Math.abs(expected.x) < minAbsX) return { reading, skipped: true, expected };
  if (brighter !== expected.side) throw new Error(reading);
  if (Math.abs(left - right) < 0.1 * Math.max(left, right)) throw new Error(`the two limbs differ by less than 10 %: ${reading}`);
  return { reading, expected };
}

/**
 * SC-011, FR-019: away from the limb the atmosphere changes nothing. Boxes at the globe centre
 * and halfway to the limb on the Sun's side, with the atmosphere on and off: within 2 %.
 */
async function noHaze() {
  const view = parseView(await stableView('before the haze check'));
  const rect = await mapRect();
  const [cx, cy] = centre(rect);
  const r = apparentRadiusPx(view, rect.height);
  const side = sunSide(view, sunNow()).side === 'left' ? -1 : 1;
  const boxes = [
    { x: cx - 10, y: cy - 10, w: 20, h: 20 },
    { x: cx + side * r * 0.5 - 10, y: cy - 10, w: 20, h: 20 },
    { x: cx + side * r * 0.85 - 10, y: cy - 10, w: 20, h: 20 },
  ];
  const on = (await page.measure(await page.capture(), boxes)).map(luminance);
  await q(`document.getElementById('atmosphere').click()`);
  await sleep(500);
  const off = (await page.measure(await page.capture(), boxes)).map(luminance);
  await q(`document.getElementById('atmosphere').click()`);
  const reading = on.map((v, i) => `${v.toFixed(1)}/${off[i].toFixed(1)}`).join(', ');
  console.log(`   10-no-haze: on/off at centre, half, 85 %: ${reading}`);
  for (const [i, v] of on.entries()) {
    if (Math.abs(v - off[i]) > 0.02 * 255) throw new Error(`the atmosphere hazes the map (on/off ${reading})`);
  }
}

/** Drags the globe until its centre is within 15° of longitude `target`. */
async function turnToLongitude(target) {
  for (let i = 0; i < 10; i++) {
    const view = parseView(await stableView('before turning to a longitude'));
    const off = ((((target - view.lon + 180) % 360) + 360) % 360) - 180;
    if (Math.abs(off) <= 15) return;
    const [cx, cy] = centre(await mapRect());
    // Dragging left turns the centre east; about 0.5° per pixel at this zoom, at most 150 px.
    const dx = Math.max(-150, Math.min(150, -off * 2));
    await page.drag([cx, cy], [cx + dx, cy]);
  }
  throw new Error(`could not turn the globe to longitude ${target.toFixed(1)}`);
}

/**
 * Item 7: switches auto-zoom on and samples every frame of the flight: the light the page set
 * (`data-light`) must point to the Sun's side for that frame's camera (`data-camera`), worked
 * out independently here (FR-003, SC-003). Three of the frames are kept as screenshots.
 */
async function flightFollowsSun() {
  const before = await q(`document.getElementById('map').dataset.view`);
  await q(`document.querySelector('.toggle-control button').click()`);
  const frames = [];
  let last = null;
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    const now = await q(`(() => { const d = document.getElementById('map').dataset; return { view: d.view, camera: d.camera, light: d.light }; })()`);
    if (now.view !== before) break; // `moveend`: the flight is over
    if (now.camera !== last && now.camera) {
      last = now.camera;
      const expected = sunSide(parseView(now.camera), sunNow());
      const actual = lightSide(now.light);
      frames.push({ camera: now.camera, expected: expected.side, x: expected.x, actual: actual?.side });
      // One screenshot, once 3 frames are in: a capture takes longer than a frame.
      if (frames.length === 3) {
        const name = '10-atmosphere-flight';
        await page.capture(path.join(out, `${name}.png`));
        if (currentStep) addScreenshot(currentStep, `${name}.png`);
      }
    }
  }
  console.log(`   10-flight: ${frames.length} frames, ${frames.map((f) => `${f.camera}→${f.actual}`).slice(0, 4).join(' | ')}`);
  if (frames.length < 3) throw new Error(`only ${frames.length} frames seen during the auto-zoom flight (need 3)`);
  const wrong = frames.filter((f) => Math.abs(f.x) >= 0.3 && f.actual !== f.expected);
  if (wrong.length > 0) {
    throw new Error(`during the flight the glow was off the Sun's side: ${wrong.map((f) => `${f.camera} light ${f.actual}, Sun ${f.expected}`).join('; ')}`);
  }
}

/** Ticks or unticks a settings checkbox, waiting for the map to redraw. */
async function setBox(id, on) {
  if ((await q(`document.getElementById('${id}').checked`)) !== on) await q(`document.getElementById('${id}').click()`);
  await sleep(400);
}

/** 9×9 boxes around page points. */
const boxesAt = (points, size = 9) => points.map(([x, y]) => ({ x: x - size / 2, y: y - size / 2, w: size, h: size }));

/**
 * Step 11, items 2–3: with the grayline on, a deep-night point is at least 15 % darker than with
 * it off, and a full-day point is within 2 %. `project` maps `[lat, lon]` to page pixels.
 */
async function shadingAt(shotName, project) {
  const viewText = await stableView(`for ${shotName}`);
  const view = parseView(viewText);
  const rect = await mapRect();
  const middleOf = await globeCentre(rect);
  const points = nightAndDayPoints(sunNow(), (p) => project(view, rect, p, middleOf), rect);
  if (!points) throw new Error(`no deep-night and full-day points on screen at ${viewText}`);
  const boxes = boxesAt([points.night.xy, points.day.xy]);
  await setBox('grayline', true);
  const on = (await page.measure(await page.capture(path.join(out, `${shotName}.png`)), boxes)).map(luminance);
  if (currentStep) addScreenshot(currentStep, `${shotName}.png`);
  await setBox('grayline', false);
  const off = (await page.measure(await page.capture(), boxes)).map(luminance);
  await setBox('grayline', true);
  const reading = `night ${points.night.at.map((v) => v.toFixed(1))} on/off ${on[0].toFixed(1)}/${off[0].toFixed(1)}, day ${points.day.at.map((v) => v.toFixed(1))} on/off ${on[1].toFixed(1)}/${off[1].toFixed(1)}, view ${viewText}`;
  console.log(`   ${shotName}: ${reading}`);
  if (!(on[0] <= 0.85 * off[0])) throw new Error(`deep night is not shaded: ${reading}`);
  if (Math.abs(on[1] - off[1]) > 0.02 * Math.max(off[1], 1)) throw new Error(`full day is shaded: ${reading}`);
}

/** Step 11, item 5: the globe's centre box, with the grayline on and off. */
async function centreShading(shotName) {
  const viewText = await stableView(`for ${shotName}`);
  const rect = await mapRect();
  const [x, y] = (await globeCentre(rect)) ?? centre(rect);
  const boxes = boxesAt([[x, y]], 15);
  await setBox('grayline', true);
  const [on] = (await page.measure(await page.capture(path.join(out, `${shotName}.png`)), boxes)).map(luminance);
  if (currentStep) addScreenshot(currentStep, `${shotName}.png`);
  await setBox('grayline', false);
  const [off] = (await page.measure(await page.capture(), boxes)).map(luminance);
  await setBox('grayline', true);
  console.log(`   ${shotName}: centre on/off ${on.toFixed(1)}/${off.toFixed(1)}, view ${viewText}`);
  return { on, off, viewText };
}

/** `requestAnimationFrame` callbacks per second during a 2 s scripted drag (step 11, item 7). */
async function framesDuringDrag() {
  const [cx, cy] = centre(await mapRect());
  await q(`(() => { window.__lqmFrames = 0; window.__lqmCount = true; const tick = () => { window.__lqmFrames++; if (window.__lqmCount) requestAnimationFrame(tick); }; requestAnimationFrame(tick); })()`);
  const started = Date.now();
  let dx = 120;
  while (Date.now() - started < 2_000) {
    await page.drag([cx - dx / 2, cy], [cx + dx / 2, cy], 10);
    dx = -dx;
  }
  const frames = await q(`(() => { window.__lqmCount = false; return window.__lqmFrames; })()`);
  return frames / ((Date.now() - started) / 1000);
}

/** Drags across the globe's centre by `dx` pixels until the Sun's |x| is ≥ 0.5 on `side`. */
async function turnUntil(side, dx) {
  for (let i = 0; i < 8; i++) {
    const view = parseView(await stableView('before a drag'));
    const s = sunSide(view, sunNow());
    if (Math.abs(s.x) >= 0.5 && (!side || s.side === side)) return;
    const [cx, cy] = centre(await mapRect());
    await page.drag([cx - dx / 2, cy], [cx + dx / 2, cy]);
  }
  throw new Error(`could not turn the globe to put the Sun on the ${side ?? 'either'} side`);
}

// --- the steps (contracts/browser-test.md) --------------------------------------------------

const steps = {
  async load() {
    await page.goto(`${origin}/`);
    await pageReady();
    const controls = await q(`({
      zoom: !!document.querySelector('.maplibregl-ctrl-zoom-in'),
      projection: !!document.querySelector('.projection-control button'),
      autozoom: !!document.querySelector('.toggle-control button'),
      gear: !!document.querySelector('.settings-control button'),
    })`);
    for (const [name, present] of Object.entries(controls)) {
      if (!present) throw new Error(`missing control: ${name}`);
    }
    await sameList(legendRows, CONTEST_BANDS, 'the legend in frequency order');
    await shot('01-load');
  },

  async qsos() {
    await sendQso({ call: 'DL1AAA', band: '40m', grid: 'JN58' });
    await sendQso({ call: 'K1AAA', band: '20m', grid: 'FN42' });
    await sendQso({ call: 'I1AAA', band: '30m', grid: 'JN45' }); // not considered
    await sendQso({ call: 'I2AAA', band: '2m', grid: 'JN61' }); // not considered
    await waitFor(
      async () => {
        const calls = await boxCalls();
        return calls.includes('DL1AAA') && calls.includes('K1AAA') && (await markerCount()) === 2;
      },
      { timeout: STEP_MS, what: 'the 40m and 20m QSOs in the box and as two markers' },
    );
    await sleep(1_000);
    const calls = await boxCalls();
    if (calls.includes('I1AAA') || calls.includes('I2AAA')) throw new Error(`a band not considered is shown: ${calls}`);
    if ((await markerCount()) !== 2) throw new Error(`expected 2 markers, got ${await markerCount()}`);
    await shot('02-qsos');
  },

  async 'panel-open'() {
    await q(`document.querySelector('.settings-control button').click()`);
    await until(`!document.getElementById('panel').hidden`, 'the panel to open');
    const panel = await q(`(() => {
      const panel = document.getElementById('panel');
      const first = panel.querySelector('select, input, button:not(.panel-close)');
      return {
        groups: [...panel.querySelectorAll('fieldset > legend')].map((l) => l.textContent.trim()),
        focusFirst: document.activeElement === first,
        expanded: document.querySelector('.settings-control button').getAttribute('aria-expanded'),
      };
    })()`);
    const groups = ['Map', 'Last QSOs box', 'Appearance', 'Legend'];
    if (JSON.stringify(panel.groups) !== JSON.stringify(groups)) throw new Error(`panel groups: ${panel.groups}`);
    if (!panel.focusFirst) throw new Error('focus is not on the panel\'s first control');
    if (panel.expanded !== 'true') throw new Error(`aria-expanded is ${panel.expanded}`);
    await sameList(groupLabels, ['All considered bands', 'HF contest (6 bands)'], 'the Legend group (B4)');
    await shot('03-panel-open');
  },

  async themes() {
    const rows = (await boxCalls()).length;
    for (const [choice, name] of [['light', 'Light'], ['dim', 'Dim'], ['dark', 'Dark']]) {
      const before = await stableView(`before ${choice}`);
      await setTheme(choice);
      await until(`document.documentElement.dataset.theme === '${choice}'`, `data-theme="${choice}"`);
      await until(
        `document.getElementById('theme-status').textContent.trim() === '${name}: chosen'`,
        `the status line "${name}: chosen"`,
      );
      const after = await stableView(`after ${choice}`);
      if (!sameView(before, after)) throw new Error(`the view moved with the ${choice} theme: ${before} → ${after}`);
      if (!(await q(`document.querySelector('.toggle-control button').classList.contains('active')`))) {
        throw new Error(`auto-zoom went off with the ${choice} theme`);
      }
      if ((await boxCalls()).length !== rows) throw new Error(`the box lost rows with the ${choice} theme`);
      await shot(`04-theme-${choice}`);
    }
  },

  async 'panel-escape'() {
    await q(`document.getElementById('theme').focus()`);
    await page.press('Escape');
    await until(`document.getElementById('panel').hidden`, 'Escape to close the panel');
    const state = await q(`({
      onGear: document.activeElement === document.querySelector('.settings-control button'),
      expanded: document.querySelector('.settings-control button').getAttribute('aria-expanded'),
    })`);
    if (!state.onGear) throw new Error('focus did not return to the settings button');
    if (state.expanded !== 'false') throw new Error(`aria-expanded is ${state.expanded}`);
    await shot('05-panel-escape');
  },

  async 'reload-keeps-settings'() {
    await page.reload();
    await pageReady();
    await until(`document.documentElement.dataset.theme === 'dark'`, 'the saved Dark theme after a reload');
    await shot('06-reload-keeps-settings');
  },

  async 'device-dark'() {
    await q(`localStorage.setItem('lqm.theme', 'auto-device')`);
    await page.emulateDark(true);
    await page.reload();
    await pageReady();
    await until(`document.documentElement.dataset.theme === 'dark'`, 'Dark from the device preference');
    await page.emulateDark(false);
    await until(`document.documentElement.dataset.theme === 'light'`, 'Light when the device turns light');
    await shot('07-device-dark');
  },

  async reconnect() {
    allowWebSocketErrors = true;
    await stopProgram();
    await until(`!document.getElementById('status').hidden`, 'the reconnection banner');
    await shot('08-reconnect-banner');
    await startProgram('b', ['--bands', 'hf']);
    await until(`document.getElementById('status').hidden`, 'the banner to go away after the restart', LOAD_MS);
    fatalSinceLastCheck();
    allowWebSocketErrors = false;
    // The QSOs of step 02 come back from the history file (feature 014).
    await waitFor(
      async () => {
        const calls = await boxCalls();
        return calls.includes('DL1AAA') && calls.includes('K1AAA') && (await markerCount()) === 2;
      },
      { timeout: STEP_MS, what: 'the QSOs restored from the history file after the restart' },
    );
    await sendQso({ call: 'DL2BBB', band: '40m', grid: 'JO62' });
    await sendQso({ call: 'I3BBB', band: '30m', grid: 'JN45' });
    await waitFor(
      async () => {
        const calls = await boxCalls();
        return calls.includes('DL2BBB') && calls.includes('I3BBB') && (await markerCount()) === 4;
      },
      { timeout: STEP_MS, what: 'the QSOs sent after the restart, next to the restored ones' },
    );
    await shot('08-reconnect');
  },

  async 'legend-group'() {
    await q(`document.querySelector('.settings-control button').click()`);
    await until(`!document.getElementById('panel').hidden`, 'the panel to open');
    await sameList(
      groupLabels,
      ['All considered bands', 'HF (10 bands)', 'HF contest (6 bands)', 'WARC (3 bands)'],
      'the groups of --bands hf',
    );
    await q(`document.querySelector('#legend-groups input[value="warc"]').click()`);
    await sameList(legendRows, ['30m', '17m', '12m'], 'the legend restricted to WARC');
    await sameList(boxCalls, ['I3BBB'], 'the box restricted to WARC');
    if ((await markerCount()) !== 1) throw new Error(`expected 1 marker with WARC, got ${await markerCount()}`);
    await shot('09-legend-group');
    await q(`document.querySelector('#legend-groups input[value="warc"]').click()`);
    await waitFor(async () => (await legendRows()).length === 10 && (await markerCount()) === 4, {
      timeout: STEP_MS,
      what: 'every band shown again after unticking WARC',
    });
  },

  async atmosphere() {
    // The test clock: the Sun 90° west of the house (contracts/browser-test.md, feature 013).
    clockOffset = await page.setClock(testInstant(HOUSE, new Date().getUTCFullYear()).toISOString());
    await page.reload();
    await pageReady();
    await setTheme('dark');
    await until(`document.documentElement.dataset.theme === 'dark'`, 'the Dark theme');
    if (!(await q(`document.getElementById('atmosphere').checked`))) await q(`document.getElementById('atmosphere').click()`);
    await setAutoZoom(false);
    await ensureGlobe();
    // Items 3–5: a hand drag (it also sets `data-view`, empty after a reload), the whole
    // globe in the window, then the glow on the Sun's side.
    const [cx, cy] = centre(await mapRect());
    await page.drag([cx - 60, cy], [cx + 60, cy]);
    await wholeGlobe();
    await turnUntil(null, 200);
    const first = await glowOnSunSide('10-atmosphere-drag');
    // Item 6: about half a turn, so the Sun changes side.
    const other = first.expected.side === 'left' ? 'right' : 'left';
    // Dragging right turns the view centre west, past a Sun on the left (and the reverse).
    await turnUntil(other, first.expected.side === 'left' ? 300 : -300);
    await glowOnSunSide('10-atmosphere-turned');
    await noHaze();

    // Item 7: auto-zoom flies back to the house from about 40° east of it. A QSO in Brazil keeps
    // the framing wide, so the whole globe stays in the window.
    await sendQso({ call: 'PY2AAA', band: '40m', grid: 'GG66' });
    await turnToLongitude(HOUSE.lon + 40);
    await flightFollowsSun();
    await glowOnSunSide('10-atmosphere-autozoom', { minAbsX: 0.3 });

    // US2 scenario 3: to the flat map and back, the glow is on the Sun's side again.
    await q(`document.querySelector('.projection-control button').click()`);
    await until(`document.querySelector('.projection-control button').textContent === 'Globe'`, 'the flat map');
    await ensureGlobe();
    await glowOnSunSide('10-atmosphere-projection', { minAbsX: 0.3 });
  },

  async grayline() {
    // Item 1: Light theme, flat map, auto-zoom off, the whole world width in the window.
    await setTheme('light');
    await until(`document.documentElement.dataset.theme === 'light'`, 'the Light theme');
    await setAutoZoom(false);
    if ((await q(`document.querySelector('.projection-control button').textContent`)) === 'Flat') {
      await q(`document.querySelector('.projection-control button').click()`);
    }
    await until(`document.querySelector('.projection-control button').textContent === 'Globe'`, 'the flat map');
    for (let i = 0; i < 6; i++) {
      const view = parseView(await stableView('before fitting the world'));
      if (512 * 2 ** view.zoom <= (await mapRect()).width) break;
      await q(`document.querySelector('.maplibregl-ctrl-zoom-out').click()`);
    }
    // Items 2–4: on the flat map, then after a 300 px pan.
    await shadingAt('11-grayline-flat', toScreenMercator);
    const [cx, cy] = centre(await mapRect());
    await page.drag([cx - 150, cy], [cx + 150, cy]);
    await shadingAt('11-grayline-panned', toScreenMercator);

    // Item 5: on the globe, the centre in deep night, then in full day.
    await ensureGlobe();
    await wholeGlobe();
    const sub = sunNow();
    await turnToLongitude(sub.lon + 180);
    const night = await centreShading('11-grayline-globe-night');
    if (!(night.on <= 0.85 * night.off)) throw new Error(`the globe's night side is not shaded (${night.on.toFixed(1)}/${night.off.toFixed(1)} at ${night.viewText})`);
    await turnToLongitude(sub.lon);
    const day = await centreShading('11-grayline-globe-day');
    if (Math.abs(day.on - day.off) > 0.02 * Math.max(day.off, 1)) throw new Error(`the globe's day side is shaded (${day.on.toFixed(1)}/${day.off.toFixed(1)} at ${day.viewText})`);

    // Item 7: smoothness, recorded for the report and never judged here (SC-004, owner's
    // decision): the test browser renders in software, where every full-screen layer costs CPU;
    // on a real GPU every case measured 60 frames/s. Judged by eye on real hardware.
    const SUN_BOXES = ['atmosphere', 'grayline', 'sun-marker'];
    const rates = {};
    // Nothing and everything only: the run stays within feature 012's 2-minute budget.
    for (const all of [false, true]) {
      for (const id of SUN_BOXES) await setBox(id, all);
      rates[all ? 'all' : 'none'] = await framesDuringDrag();
    }
    if (currentStep) currentStep.framesPerSecond = Object.fromEntries(Object.entries(rates).map(([k, v]) => [k, Number(v.toFixed(1))]));
    console.log(`   11-smoothness (frames/s, recorded only): ${Object.entries(rates).map(([k, v]) => `${k} ${v.toFixed(1)}`).join(', ')}`);
    await setBox('atmosphere', true);
    await setBox('grayline', true);

    // Item 6: the Sun symbol at the subsolar point, on the flat map, only when its choice is on.
    await q(`document.querySelector('.projection-control button').click()`);
    await until(`document.querySelector('.projection-control button').textContent === 'Globe'`, 'the flat map again');
    await turnToLongitude(sunNow().lon);
    const viewText = await stableView('for the Sun symbol');
    const rect = await mapRect();
    const sunXy = toScreenMercator(parseView(viewText), rect, [sunNow().lat, sunNow().lon], await globeCentre(rect));
    const sunBox = boxesAt([sunXy], 13);
    await setBox('sun-marker', true);
    const [withSun] = (await page.measure(await page.capture(path.join(out, '11-grayline-sun.png')), sunBox)).map(luminance);
    if (currentStep) addScreenshot(currentStep, '11-grayline-sun.png');
    await setBox('sun-marker', false);
    const [withoutSun] = (await page.measure(await page.capture(), sunBox)).map(luminance);
    console.log(`   11-sun: subsolar box with/without the symbol ${withSun.toFixed(1)}/${withoutSun.toFixed(1)}, at ${sunXy.map((v) => v.toFixed(0))}`);
    if (Math.abs(withSun - withoutSun) < 0.1 * Math.max(withSun, withoutSun)) throw new Error(`no Sun symbol at the subsolar point (${withSun.toFixed(1)}/${withoutSun.toFixed(1)})`);

    // Item 8: the choices survive a reload.
    await setBox('grayline', false);
    await setBox('sun-marker', true);
    await page.reload();
    await pageReady();
    const kept = await q(`({ grayline: document.getElementById('grayline').checked, sun: document.getElementById('sun-marker').checked, stored: [localStorage.getItem('lqm.grayline'), localStorage.getItem('lqm.sun')] })`);
    if (kept.grayline !== false || kept.sun !== true || kept.stored[0] !== 'off' || kept.stored[1] !== 'on') {
      throw new Error(`the choices were not kept after a reload: ${JSON.stringify(kept)}`);
    }
  },
};

// --- the run --------------------------------------------------------------------------------

const ports = { http: await freeTcpPort(), udp: await freeUdpPort() };
const origin = `http://127.0.0.1:${ports.http}`;
const run = createRun();
const started = new Date().toISOString();

async function cleanup() {
  if (page) page.close();
  await stopProgram();
  // Chrome may still hold its profile for a moment after being killed.
  await sleep(500);
  rmSync(profileDir, { recursive: true, force: true, maxRetries: 3 });
  try {
    udp.close();
  } catch {
    // Already closed.
  }
}

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, async () => {
    await cleanup();
    process.exit(1);
  });
}

function writeReport() {
  writeFileSync(
    path.join(out, 'report.json'),
    `${JSON.stringify({ binary, chrome, started, result: run.result, steps: run.steps, events: page?.events ?? [] }, null, 2)}\n`,
  );
  const lines = (page?.events ?? []).map(
    (e) => `${e.at} ${e.fatal ? 'FATAL ' : ''}${e.kind}${e.level ? `.${e.level}` : ''} ${e.text ?? ''} ${e.url ?? ''} ${e.status ?? ''}`.trimEnd(),
  );
  writeFileSync(path.join(out, 'console.log'), `${lines.join('\n')}\n`);
}

let exitCode = 0;
try {
  await startProgram('a', []);
  try {
    page = await launchChrome({
      chrome,
      profileDir,
      headed: options.headed,
      ci: process.env.CI === 'true',
      port: await freeTcpPort(),
    });
  } catch (error) {
    console.error(error.message);
    exitCode = 2;
    throw error;
  }
  await page.enable();
  await page.emulateDark(false);

  for (const [index, name] of STEPS.entries()) {
    const step = startStep(run, name);
    currentStep = step;
    let error = null;
    try {
      await steps[name]();
      error = fatalSinceLastCheck();
    } catch (caught) {
      error = caught;
    }
    finishStep(run, step, error);
    console.log(`${String(index + 1).padStart(2, '0')} ${name.padEnd(22)} ${step.status} (${step.ms} ms)${error ? `: ${step.message}` : ''}`);
    if (error) {
      await shot(`${String(index + 1).padStart(2, '0')}-${name}-failed`).catch(() => {});
      skipRest(run, STEPS.slice(index + 1));
      break;
    }
  }
} catch (error) {
  if (exitCode === 0) {
    console.error(error.message);
    run.result = 'fail';
  }
} finally {
  writeReport();
  await cleanup();
}

if (exitCode === 0 && run.result === 'fail') exitCode = 1;
console.log(`browser tests: ${exitCode === 0 ? 'pass' : 'fail'} (${out})`);
process.exit(exitCode);
