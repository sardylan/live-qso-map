import { test } from 'node:test';
import assert from 'node:assert/strict';
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
} from '../tests/browser/lib.mjs';

// The browser test runner's pure helpers (feature 012, contracts/browser-test.md, research R5).

test('arguments: defaults per platform', () => {
  const linux = parseArgs([], {}, 'linux');
  assert.deepEqual(linux, {
    ok: true,
    options: { binary: 'target/release/live-qso-map', chrome: null, out: 'target/browser-test', headed: false },
  });
  assert.equal(parseArgs([], {}, 'win32').options.binary, 'target/release/live-qso-map.exe');
});

test('arguments: every option, and CHROME_PATH when --chrome is absent', () => {
  const parsed = parseArgs(
    ['--binary', 'b', '--chrome', 'c', '--out', 'o', '--headed'],
    { CHROME_PATH: 'env-chrome' },
    'darwin',
  );
  assert.deepEqual(parsed.options, { binary: 'b', chrome: 'c', out: 'o', headed: true });
  assert.equal(parseArgs([], { CHROME_PATH: 'env-chrome' }, 'linux').options.chrome, 'env-chrome');
});

test('arguments: an unknown option or a missing value is a usage error', () => {
  for (const argv of [['--nope'], ['--binary'], ['--out', '--headed'], ['extra']]) {
    const parsed = parseArgs(argv, {}, 'linux');
    assert.equal(parsed.ok, false, argv.join(' '));
    assert.match(parsed.error, /usage|unknown|missing|value/i, argv.join(' '));
  }
});

test('Chrome candidates: Chrome then Chromium, per platform', () => {
  const linux = chromeCandidates('linux', { PATH: '/usr/bin:/opt/bin' });
  assert.deepEqual(linux.slice(0, 3), ['/usr/bin/google-chrome', '/opt/bin/google-chrome', '/usr/bin/google-chrome-stable']);
  assert.ok(linux.includes('/usr/bin/chromium') && linux.includes('/opt/bin/chromium-browser'));
  assert.ok(linux.indexOf('/usr/bin/google-chrome') < linux.indexOf('/usr/bin/chromium'));

  const mac = chromeCandidates('darwin', { HOME: '/Users/op' });
  assert.equal(mac[0], '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome');
  assert.ok(mac.includes('/Applications/Chromium.app/Contents/MacOS/Chromium'));
  assert.ok(mac.includes('/Users/op/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'));

  const win = chromeCandidates('win32', { ProgramFiles: 'C:\\Program Files', LOCALAPPDATA: 'C:\\Users\\op\\AppData\\Local' });
  assert.equal(win[0], 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe');
  assert.ok(win.includes('C:\\Users\\op\\AppData\\Local\\Google\\Chrome\\Application\\chrome.exe'));
  assert.ok(win.every((p) => !p.includes('undefined')), 'unset variables are skipped');
});

test('findChrome returns the first candidate that exists, or null', () => {
  assert.equal(findChrome(['/a', '/b', '/c'], (p) => p !== '/a'), '/b');
  assert.equal(findChrome(['/a'], () => false), null);
});

test('events: what fails a run (research R5)', () => {
  const origin = 'http://127.0.0.1:8090';
  const fatal = (event) => classifyEvent(event, origin).fatal;
  assert.equal(fatal({ kind: 'http-error', url: `${origin}/legend.js`, status: 404 }), true);
  assert.equal(fatal({ kind: 'request-failed', url: `${origin}/ws`, text: 'net::ERR_CONNECTION_REFUSED' }), true);
  assert.equal(fatal({ kind: 'exception', text: 'TypeError: x is undefined' }), true);
  assert.equal(fatal({ kind: 'console', level: 'error', text: 'bad message' }), true);

  assert.equal(fatal({ kind: 'http-error', url: `${origin}/favicon.ico`, status: 404 }), false, 'no favicon by design');
  assert.equal(fatal({ kind: 'http-error', url: 'https://tiles.openfreemap.org/x.pbf', status: 503 }), false, 'tiles');
  assert.equal(fatal({ kind: 'request-failed', url: 'https://tiles.openfreemap.org/y', text: 'net::ERR_ABORTED' }), false);
  assert.equal(fatal({ kind: 'console', level: 'error', text: 'map error Error: tile 503' }), false, "the page's own map errors");
  assert.equal(
    fatal({ kind: 'console', level: 'error', source: 'network', text: 'Failed to load resource: 503' }),
    false,
    'network log lines are judged by their own request events',
  );
  assert.equal(fatal({ kind: 'console', level: 'warning', text: 'performance warning' }), false);
  assert.equal(fatal({ kind: 'console', level: 'log', text: 'hello' }), false);
});

test('steps keep order, status, time and message; a failure marks the rest not run', () => {
  const run = createRun();
  assert.equal(run.result, 'pass');
  const load = startStep(run, 'load', 1000);
  finishStep(run, load, null, 1250);
  const qsos = startStep(run, 'qsos', 1300);
  finishStep(run, qsos, new Error('no 40m row within 5 s'), 6300);
  skipRest(run, ['panel-open', 'reconnect']);
  assert.deepEqual(
    run.steps.map((s) => [s.name, s.status, s.ms ?? null, s.message ?? null]),
    [
      ['load', 'pass', 250, null],
      ['qsos', 'fail', 5000, 'no 40m row within 5 s'],
      ['panel-open', 'not-run', null, null],
      ['reconnect', 'not-run', null, null],
    ],
  );
  assert.equal(run.result, 'fail');
});

function fakeClock() {
  let t = 0;
  return { now: () => t, sleep: async (ms) => { t += ms; } };
}

test('waitFor resolves with the first truthy value', async () => {
  const clock = fakeClock();
  let calls = 0;
  const value = await waitFor(async () => (++calls >= 3 ? 'ready' : null), { timeout: 1000, interval: 100, what: 'x', ...clock });
  assert.equal(value, 'ready');
  assert.equal(calls, 3);
});

test('waitFor gives up after its limit, naming what it waited for and the last value', async () => {
  const clock = fakeClock();
  await assert.rejects(
    waitFor(() => 0, { timeout: 500, interval: 100, what: 'the 40m row', ...clock }),
    (error) => /the 40m row/.test(error.message) && /500 ms/.test(error.message) && /last: 0/.test(error.message),
  );
});

test('the ADIF record has UTC date and time, and byte lengths', () => {
  const at = new Date(Date.UTC(2026, 9, 13, 7, 5, 9));
  assert.equal(
    qsoAdif({ call: 'DL1ABC', band: '40m', mode: 'CW', grid: 'JN58', at }),
    '<CALL:6>DL1ABC <QSO_DATE:8>20261013 <TIME_ON:6>070509 <BAND:3>40m <MODE:2>CW <GRIDSQUARE:4>JN58 <EOR>',
  );
  assert.match(qsoAdif({ call: 'Ä1A', band: '2m', mode: 'FM', grid: 'JN61', at }), /<CALL:4>Ä1A /, 'bytes, not characters');
  assert.match(
    qsoAdif({ call: 'DL1ABC', band: '40m', mode: 'CW', grid: 'JN58', at, operator: 'IK0ABC' }),
    /<GRIDSQUARE:4>JN58 <OPERATOR:6>IK0ABC <EOR>$/,
    'the operator when given (feature 017)',
  );
});

test('a step lists the screenshots taken during it (data-model Step.screenshot)', () => {
  const run = createRun();
  const themes = startStep(run, 'themes', 0);
  addScreenshot(themes, '04-theme-light.png');
  addScreenshot(themes, '04-theme-dim.png');
  finishStep(run, themes, null, 10);
  const escape = startStep(run, 'panel-escape', 10);
  finishStep(run, escape, null, 20);
  assert.deepEqual(run.steps[0].screenshots, ['04-theme-light.png', '04-theme-dim.png']);
  assert.equal(run.steps[1].screenshots, undefined, 'no screenshot, no list');
});

// --- the glow measured on screen (feature 013, contracts/browser-test.md step 10, T005) ---

import {
  luminance,
  parseView,
  globeRadiusPx,
  apparentRadiusPx,
  limbSamples,
  sunSide,
  testInstant,
  lightSide,
  toScreenMercator,
  toScreenGlobe,
  nightAndDayPoints,
} from '../tests/browser/lib.mjs';
import { subsolarPoint } from '../src/web/assets/sun.js';

test('luminance: Rec. 709 luma, 0 to 255', () => {
  assert.equal(luminance([0, 0, 0, 255]), 0);
  assert.ok(Math.abs(luminance([255, 255, 255, 255]) - 255) < 1e-9);
  assert.ok(luminance([0, 255, 0, 255]) > luminance([255, 0, 0, 255]), 'green weighs more than red');
});

test('parseView reads #map data-view "lat,lon,zoom"', () => {
  assert.deepEqual(parseView('12.5000,-45.2500,1.2000'), { lat: 12.5, lon: -45.25, zoom: 1.2 });
  assert.equal(parseView(null), null);
  assert.equal(parseView('nonsense'), null);
});

test('globe radius in pixels grows with zoom and towards the poles', () => {
  const r0 = globeRadiusPx({ lat: 0, lon: 0, zoom: 0 });
  assert.ok(Math.abs(r0 - 512 / (2 * Math.PI)) < 1e-9);
  assert.ok(Math.abs(globeRadiusPx({ lat: 0, lon: 0, zoom: 1 }) - 2 * r0) < 1e-9);
  assert.ok(globeRadiusPx({ lat: 60, lon: 0, zoom: 0 }) > 1.99 * r0, 'cos 60° halves the divisor');
});

test('the apparent radius is smaller than the sphere radius, as the camera sees it', () => {
  // Measured on a 1400×813 map at zoom 1.5, centre latitude 30: about 222 px (feature 013).
  const view = { lat: 30, lon: -29.83, zoom: 1.5 };
  const r = apparentRadiusPx(view, 813);
  assert.ok(Math.abs(r - 222) < 3, `${r}`);
  assert.ok(r < globeRadiusPx(view));
});

test('limb samples sit just outside the visible limb, left and right, inside the map', () => {
  const rect = { left: 0, top: 0, width: 1200, height: 800 };
  const view = { lat: 0, lon: 0, zoom: 1.5 };
  const r = apparentRadiusPx(view, 800);
  const [left, right] = limbSamples(view, rect);
  const cx = 600;
  assert.ok(left.x >= cx - r * 1.014 - 1e-9 && left.x + left.w <= cx - r * 1.003 + 1e-9, 'left box');
  assert.ok(right.x >= cx + r * 1.003 - 1e-9 && right.x + right.w <= cx + r * 1.014 + 1e-9, 'right box');
  assert.ok(left.y < 400 && left.y + left.h > 400, 'at the centre height');
  for (const b of [left, right]) assert.ok(b.w >= 1 && b.h >= 1);
  assert.equal(limbSamples({ lat: 0, lon: 0, zoom: 4 }, rect), null, 'limb off screen');
  const offset = limbSamples(view, { left: 100, top: 50, width: 1200, height: 800 });
  assert.ok(Math.abs(offset[0].x - left.x - 100) < 1e-9 && Math.abs(offset[0].y - left.y - 50) < 1e-9);
  // Auto-zoom frames with padding: the globe's centre is where the map centre projects.
  const padded = limbSamples(view, rect, [600, 330]);
  assert.ok(padded[0].y < 330 && padded[0].y + padded[0].h > 330, 'at the given centre');
  assert.ok(Math.abs(padded[0].x - left.x) < 1e-9);
});

test('sunSide: the Sun 90° west of the view centre is on the left', () => {
  const left = sunSide({ lat: 0, lon: 90, zoom: 1 }, { lat: 0, lon: 0 });
  assert.equal(left.side, 'left');
  assert.ok(left.x < -0.9);
  const right = sunSide({ lat: 0, lon: -90, zoom: 1 }, { lat: 0, lon: 0 });
  assert.equal(right.side, 'right');
  assert.ok(right.x > 0.9);
});

test('testInstant puts the Sun 90° west of the house on the June solstice', () => {
  for (const lon of [-170, 0, 9.12, 179]) {
    const when = testInstant({ lat: 39.2, lon }, 2026);
    assert.equal(when.getUTCFullYear(), 2026);
    assert.equal(when.getUTCMonth(), 5);
    assert.equal(when.getUTCDate(), 21);
    const target = ((((lon - 90 + 180) % 360) + 360) % 360) - 180;
    const diff = ((((subsolarPoint(when).lon - target + 180) % 360) + 360) % 360) - 180;
    assert.ok(Math.abs(diff) < 0.1, `house ${lon}: off by ${diff}°`);
  }
});

test('lightSide reads which way a light position points on screen', () => {
  // MapLibre's sun direction is −sphericalToCartesian([r, az, polar]): x = sin(az)·sin(polar).
  assert.equal(lightSide('-90.00,20.00').side, 'left');
  assert.equal(lightSide('90.00,20.00').side, 'right');
  assert.ok(Math.abs(lightSide('-90.00,20.00').x + Math.sin((20 * Math.PI) / 180)) < 1e-9);
  assert.equal(lightSide(null), null);
});

// --- the grayline measured on screen (step 11, T019) -------------------------------------

const RECT = { left: 0, top: 0, width: 1200, height: 800 };

test('Web Mercator: the centre is the middle, one world east is one world width away', () => {
  const view = { lat: 20, lon: 10, zoom: 1 };
  const [x, y] = toScreenMercator(view, RECT, [20, 10]);
  assert.ok(Math.abs(x - 600) < 1e-9 && Math.abs(y - 400) < 1e-9);
  const [xe] = toScreenMercator(view, RECT, [20, 100]);
  assert.ok(Math.abs(xe - 600 - 1024 / 4) < 1e-9, `${xe}`);
  const [, yn] = toScreenMercator(view, RECT, [40, 10]);
  assert.ok(yn < 400, 'north is up');
  const [xw] = toScreenMercator(view, RECT, [20, 10 + 360]);
  assert.ok(Math.abs(xw - 600) < 1e-9, 'the nearest world copy');
});

test('globe: the centre is the middle, the horizon is the visible limb, behind is null', () => {
  const view = { lat: 0, lon: 0, zoom: 1.5 };
  const [x, y] = toScreenGlobe(view, RECT, [0, 0]);
  assert.ok(Math.abs(x - 600) < 1e-9 && Math.abs(y - 400) < 1e-9);
  const [xe] = toScreenGlobe(view, RECT, [0, 60]);
  assert.ok(xe > 600 && xe < 600 + apparentRadiusPx(view, 800), `${xe}`);
  const [, yn] = toScreenGlobe(view, RECT, [45, 0]);
  assert.ok(yn < 400, 'north is up');
  assert.equal(toScreenGlobe(view, RECT, [0, 120]), null, 'the far side');
  const shifted = toScreenGlobe(view, RECT, [0, 0], [600, 330]);
  assert.ok(Math.abs(shifted[1] - 330) < 1e-9, 'around the given centre');
});

test('night and day points: deep night and full day, both on screen', () => {
  const view = { lat: 0, lon: 0, zoom: 0.5 };
  const sub = { lat: 10, lon: -60 };
  const project = (p) => toScreenMercator(view, RECT, p);
  const points = nightAndDayPoints(sub, project, RECT);
  assert.ok(points, 'found');
  const elevation = ([lat, lon]) => {
    const d = Math.PI / 180;
    const c = Math.sin(lat * d) * Math.sin(sub.lat * d) + Math.cos(lat * d) * Math.cos(sub.lat * d) * Math.cos((lon - sub.lon) * d);
    return 90 - Math.acos(c) / d;
  };
  assert.ok(elevation(points.night.at) < -12, `night ${points.night.at}`);
  assert.ok(elevation(points.day.at) > 10, `day ${points.day.at}`);
  for (const { xy } of [points.night, points.day]) {
    assert.ok(xy[0] >= 20 && xy[0] <= 1180 && xy[1] >= 20 && xy[1] <= 780);
  }
  assert.equal(nightAndDayPoints(sub, () => null, RECT), null, 'nothing visible');
});

// --- auto-zoom on the operator's snapshot (feature 016, step 12, T003) --------------------

import { SNAPSHOT_QSOS, locatorCentre, fitsFree } from '../tests/browser/lib.mjs';

test('snapshot: 20 QSOs, each with a callsign and a 6-character locator', () => {
  assert.equal(SNAPSHOT_QSOS.length, 20);
  assert.equal(new Set(SNAPSHOT_QSOS.map((q) => q.call)).size, 20);
  for (const { grid } of SNAPSHOT_QSOS) assert.match(grid, /^[A-R]{2}[0-9]{2}[a-x]{2}$/);
});

test('locatorCentre: the middle of the subsquare', () => {
  const [lat, lon] = locatorCentre('IO92fs');
  assert.ok(Math.abs(lat - 52.771) < 0.001 && Math.abs(lon - -1.542) < 0.001, `${lat},${lon}`);
  const [vlat, vlon] = locatorCentre('QF21eu'); // VK3ATL, Geelong: −38.16, 144.37
  assert.ok(Math.abs(vlat - -38.16) < 0.05 && Math.abs(vlon - 144.37) < 0.05, `${vlat},${vlon}`);
});

test('fitsFree: points inside the free area pass, one outside fails', () => {
  const view = { lat: 20, lon: 10, zoom: 2 };
  const free = { left: 100, top: 100, right: 1100, bottom: 700 };
  assert.equal(fitsFree(view, RECT, null, [[20, 10], [40, 60]], free), true);
  assert.equal(fitsFree(view, RECT, null, [[20, 10], [20, 160]], free), false);
  // The centre drawn elsewhere (padding) moves every point with it.
  assert.equal(fitsFree(view, RECT, [150, 400], [[20, 10]], free), true);
  assert.equal(fitsFree(view, RECT, [50, 400], [[20, 10]], free), false);
});
