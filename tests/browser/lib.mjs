// Pure helpers of the browser test runner (feature 012, contracts/browser-test.md): arguments,
// finding Chrome, judging browser events, step bookkeeping, waiting, and the ADIF records it
// sends. No I/O here, so `web/browser-lib.test.js` covers it on every change.

import path from 'node:path';

import { subsolarPoint } from '../../src/web/assets/sun.js';
import { destination } from '../../src/web/assets/geodesic.js';

const USAGE = 'usage: node tests/browser/run.mjs [--binary PATH] [--chrome PATH] [--out DIR] [--headed]';

/** Command-line options, with the contract's defaults; `{ ok: false, error }` on bad usage. */
export function parseArgs(argv, env, platform) {
  const options = {
    binary: platform === 'win32' ? 'target/release/live-qso-map.exe' : 'target/release/live-qso-map',
    chrome: env.CHROME_PATH || null,
    out: 'target/browser-test',
    headed: false,
  };
  const valued = { '--binary': 'binary', '--chrome': 'chrome', '--out': 'out' };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--headed') {
      options.headed = true;
    } else if (arg in valued) {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        return { ok: false, error: `missing value for ${arg}\n${USAGE}` };
      }
      options[valued[arg]] = value;
      i += 1;
    } else {
      return { ok: false, error: `unknown argument '${arg}'\n${USAGE}` };
    }
  }
  return { ok: true, options };
}

/** Where Chrome or Chromium usually is, in the order to try (Chrome first). */
export function chromeCandidates(platform, env) {
  if (platform === 'darwin') {
    const apps = ['Google Chrome.app/Contents/MacOS/Google Chrome', 'Chromium.app/Contents/MacOS/Chromium'];
    const roots = ['/Applications', env.HOME ? path.posix.join(env.HOME, 'Applications') : null].filter(Boolean);
    return roots.flatMap((root) => apps.map((app) => path.posix.join(root, app)));
  }
  if (platform === 'win32') {
    const roots = [env.ProgramFiles, env['ProgramFiles(x86)'], env.LOCALAPPDATA].filter(Boolean);
    const apps = [
      ['Google', 'Chrome', 'Application', 'chrome.exe'],
      ['Chromium', 'Application', 'chrome.exe'],
    ];
    return apps.flatMap((app) => roots.map((root) => path.win32.join(root, ...app)));
  }
  const dirs = (env.PATH || '').split(':').filter(Boolean);
  const names = ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser'];
  return names.flatMap((name) => dirs.map((dir) => path.posix.join(dir, name)));
}

/** The first candidate for which `exists(path)` is true, else `null`. */
export function findChrome(candidates, exists) {
  return candidates.find((candidate) => exists(candidate)) ?? null;
}

function sameOrigin(url, origin) {
  try {
    return new URL(url).origin === origin;
  } catch {
    return false;
  }
}

/**
 * Whether a browser event fails the run (research R5). Requests are fatal only on the program's
 * own origin, and never for `/favicon.ico` (the program has none). Base-map tiles, the CDN and
 * the page's own `map error` reports are kept in the log but never decide a release.
 */
export function classifyEvent(event, origin) {
  switch (event.kind) {
    case 'http-error':
    case 'request-failed':
      return { fatal: sameOrigin(event.url, origin) && new URL(event.url).pathname !== '/favicon.ico' };
    case 'exception':
      return { fatal: true };
    case 'console':
      return {
        fatal: event.level === 'error' && event.source !== 'network' && !String(event.text).startsWith('map error'),
      };
    default:
      return { fatal: false };
  }
}

/** A run: its steps in order, and `pass` until one fails. */
export function createRun() {
  return { steps: [], result: 'pass' };
}

export function startStep(run, name, now = Date.now()) {
  const step = { name, status: 'running', startedAt: now };
  run.steps.push(step);
  return step;
}

/** Ends `step`: `error` null for a pass, else its message is kept and the run fails. */
export function finishStep(run, step, error, now = Date.now()) {
  step.ms = now - step.startedAt;
  delete step.startedAt;
  if (error) {
    step.status = 'fail';
    step.message = error.message ?? String(error);
    run.result = 'fail';
  } else {
    step.status = 'pass';
  }
  return step;
}

/** Notes a screenshot taken during `step`, so the report points at its evidence (FR-011). */
export function addScreenshot(step, file) {
  (step.screenshots ??= []).push(file);
}

/** After a failure, the remaining steps are reported as not run. */
export function skipRest(run, names) {
  for (const name of names) run.steps.push({ name, status: 'not-run' });
}

/**
 * Polls `check` (sync or async) every `interval` ms until it returns a truthy value, which is
 * returned. After `timeout` ms it rejects, naming `what` and the last value seen. `sleep` and
 * `now` can be injected for tests.
 */
export async function waitFor(
  check,
  {
    timeout,
    interval = 100,
    what,
    sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
    now = () => Date.now(),
  },
) {
  const start = now();
  let last;
  for (;;) {
    last = await check();
    if (last) return last;
    if (now() - start >= timeout) {
      throw new Error(`timed out after ${timeout} ms waiting for ${what} (last: ${JSON.stringify(last)})`);
    }
    await sleep(interval);
  }
}

function field(name, value) {
  return `<${name}:${new TextEncoder().encode(value).length}>${value}`;
}

/** One ADIF QSO record as a logger sends it over UDP; date and time in UTC (HR-1). */
export function qsoAdif({ call, band, mode, grid, at, operator }) {
  const iso = at.toISOString();
  const date = iso.slice(0, 10).replaceAll('-', '');
  const time = iso.slice(11, 19).replaceAll(':', '');
  return [
    field('CALL', call),
    field('QSO_DATE', date),
    field('TIME_ON', time),
    field('BAND', band),
    field('MODE', mode),
    field('GRIDSQUARE', grid),
    ...(operator ? [field('OPERATOR', operator)] : []),
    '<EOR>',
  ].join(' ');
}

// --- the glow and the grayline measured on screen (feature 013, contracts/browser-test.md) --

/** Rec. 709 luma of an `[r, g, b, a]` pixel, 0–255. */
export function luminance([r, g, b]) {
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** `#map`'s `data-view` ("lat,lon,zoom") as `{ lat, lon, zoom }`, or `null`. */
export function parseView(text) {
  const parts = String(text ?? '').split(',').map(Number);
  if (parts.length !== 3 || parts.some((n) => !Number.isFinite(n))) return null;
  const [lat, lon, zoom] = parts;
  return { lat, lon, zoom };
}

/** The globe's radius on screen, as MapLibre 5 sizes it (`getGlobeRadiusPixels`). */
export function globeRadiusPx({ lat, zoom }) {
  return (512 * 2 ** zoom) / (2 * Math.PI) / Math.cos((lat * Math.PI) / 180);
}

/**
 * The globe's visible radius on a map `height` px tall: the perspective camera (MapLibre's
 * default 36.87° field of view) sees less than the sphere's radius.
 */
export function apparentRadiusPx(view, height) {
  const focal = height / 2 / Math.tan(((36.87 / 2) * Math.PI) / 180);
  const r = globeRadiusPx(view);
  return focal * Math.tan(Math.asin(r / (focal + r)));
}

/**
 * Two boxes in the atmosphere's thin band just outside the visible limb (0.3–1.4 % of its
 * radius; the shader's atmosphere is 1.6 % of the Earth's), left and right, at the globe
 * centre's height (`centre`, page pixels), in page pixels; `null` when either would leave the
 * map (`rect`: its box).
 */
export function limbSamples(view, rect, centre = null) {
  const r = apparentRadiusPx(view, rect.height);
  // The globe's centre on screen: the map centre's projection (`data-centre`), or the map's
  // middle when the page uses no padding.
  const [cx, cy] = centre ?? [rect.left + rect.width / 2, rect.top + rect.height / 2];
  const w = r * 0.011;
  const h = Math.max(4, r * 0.04);
  const boxes = [
    { x: cx - r * 1.014, y: cy - h / 2, w, h },
    { x: cx + r * 1.003, y: cy - h / 2, w, h },
  ];
  const inside = (b) => b.x >= rect.left && b.x + b.w <= rect.left + rect.width && b.y >= rect.top && b.y + b.h <= rect.top + rect.height;
  return boxes.every(inside) ? boxes : null;
}

/**
 * The Sun's direction in camera space for a view (bearing and pitch 0): the subsolar point on
 * MapLibre's globe axes, turned by the globe's rotations Rx(lat)·Ry(−lng) (research R2).
 */
function sunDirection(subsolar, view) {
  const D = Math.PI / 180;
  const φ = subsolar.lat * D;
  const λ = subsolar.lon * D;
  let [x, y, z] = [Math.cos(φ) * Math.sin(λ), Math.sin(φ), Math.cos(φ) * Math.cos(λ)];
  const tY = -view.lon * D;
  [x, z] = [x * Math.cos(tY) + z * Math.sin(tY), -x * Math.sin(tY) + z * Math.cos(tY)];
  const tX = view.lat * D;
  [y, z] = [y * Math.cos(tX) - z * Math.sin(tX), y * Math.sin(tX) + z * Math.cos(tX)];
  return [x, y, z];
}

/** Which limb the glow should be brighter on, for the view and the Sun: `{ side, x }`. */
export function sunSide(view, subsolar) {
  const [x] = sunDirection(subsolar, view);
  return { side: x < 0 ? 'left' : 'right', x };
}

/**
 * Which way the light in `#map`'s `data-light` ("azimuthal,polar") points on screen: MapLibre's
 * sun direction is −sphericalToCartesian, so its x is sin(az)·sin(polar). `{ side, x }` or null.
 */
export function lightSide(text) {
  const parts = String(text ?? '').split(',').map(Number);
  if (parts.length !== 2 || parts.some((n) => !Number.isFinite(n))) return null;
  const [az, polar] = parts.map((d) => (d * Math.PI) / 180);
  const x = Math.sin(az) * Math.sin(polar);
  return { side: x < 0 ? 'left' : 'right', x };
}

/**
 * The test clock's start (contracts/browser-test.md): the instant on the June solstice of `year`
 * when the subsolar longitude is the house's minus 90° (18:00 local solar time at the house).
 */
export function testInstant(house, year) {
  const target = ((((house.lon - 90 + 180) % 360) + 360) % 360) - 180;
  const day = Date.UTC(year, 5, 21);
  let minutes = 720 - 4 * target;
  for (let i = 0; i < 3; i++) {
    const when = new Date(day + ((((minutes % 1440) + 1440) % 1440) * 60_000));
    const error = ((((subsolarPoint(when).lon - target + 180) % 360) + 360) % 360) - 180;
    minutes += 4 * error; // 1° west is 4 minutes later
  }
  return new Date(day + ((((Math.round(minutes * 60) / 60) % 1440) + 1440) % 1440) * 60_000);
}

const DEG = Math.PI / 180;
const middle = (rect, centre) => centre ?? [rect.left + rect.width / 2, rect.top + rect.height / 2];

/** `[lat, lon]` on the flat map (Web Mercator, nearest world copy), in page pixels. */
export function toScreenMercator(view, rect, [lat, lon], centre = null) {
  const [cx, cy] = middle(rect, centre);
  const world = 512 * 2 ** view.zoom;
  const dLon = ((((lon - view.lon + 180) % 360) + 360) % 360) - 180;
  const merc = (φ) => Math.log(Math.tan(Math.PI / 4 + (φ * DEG) / 2));
  return [cx + (dLon / 360) * world, cy + ((merc(view.lat) - merc(lat)) / (2 * Math.PI)) * world];
}

/**
 * `[lat, lon]` on the globe (MapLibre's perspective camera, bearing and pitch 0), in page
 * pixels; `null` on the hidden side.
 */
export function toScreenGlobe(view, rect, [lat, lon], centre = null) {
  const [cx, cy] = middle(rect, centre);
  const focal = rect.height / 2 / Math.tan((36.87 / 2) * DEG);
  const r = globeRadiusPx(view);
  let [x, y, z] = [Math.cos(lat * DEG) * Math.sin(lon * DEG), Math.sin(lat * DEG), Math.cos(lat * DEG) * Math.cos(lon * DEG)];
  const tY = -view.lon * DEG;
  [x, z] = [x * Math.cos(tY) + z * Math.sin(tY), -x * Math.sin(tY) + z * Math.cos(tY)];
  const tX = view.lat * DEG;
  [y, z] = [y * Math.cos(tX) - z * Math.sin(tX), y * Math.sin(tX) + z * Math.cos(tX)];
  if (z <= r / (focal + r)) return null; // beyond the horizon the camera sees
  const depth = focal + r - r * z;
  return [cx + (focal * r * x) / depth, cy - (focal * r * y) / depth];
}

/**
 * A point in deep night (Sun below −12°) and one in full day (above +10°), both at least 20 px
 * inside `rect`, from the subsolar point and a projection; `null` when none fits (step 11).
 */
export function nightAndDayPoints(subsolar, project, rect) {
  const antisolar = [-subsolar.lat, ((((subsolar.lon + 360) % 360) + 360) % 360) - 180];
  const pick = (from) => {
    for (const distance of [0, 15, 30, 45]) {
      for (let bearing = 0; bearing < 360; bearing += 30) {
        const at = destination(from, bearing, distance);
        if (Math.abs(at[0]) > 70) continue;
        const xy = project(at);
        if (!xy) continue;
        const [x, y] = xy;
        if (x >= rect.left + 20 && x <= rect.left + rect.width - 20 && y >= rect.top + 20 && y <= rect.top + rect.height - 20) {
          return { at, xy };
        }
        if (distance === 0) break;
      }
    }
    return null;
  };
  const night = pick(antisolar);
  const day = pick([subsolar.lat, subsolar.lon]);
  return night && day ? { night, day } : null;
}

// --- auto-zoom on the operator's snapshot (feature 016, research R7) -----------------------

/**
 * The 20 QSOs of the IQ0ID page snapshot (spec Assumptions), by grid square: callsigns and
 * positions only. M5S had no grid; IO92fs holds its cty.dat position.
 */
export const SNAPSHOT_QSOS = [
  ['VK3ATL', 'QF21eu'], ['VK6LG', 'OF85wb'], ['SM5VTP', 'JO88gn'], ['SK6D', 'JO67fw'],
  ['DL9ZP', 'JO53ro'], ['M5S', 'IO92fs'], ['G0MBA', 'JO01nt'], ['DF6QV', 'JO41mq'],
  ['ON6IO', 'JO21ha'], ['M1STI', 'IO90px'], ['DM1SV', 'JN49id'], ['OK2TS', 'JN89nc'],
  ['TM1FC', 'JN18eu'], ['F4ICZ', 'JN08qb'], ['YT4M', 'KN04ev'], ['YO8SBQ', 'KN24xn'],
  ['EA1FUM', 'IN82di'], ['EE7I', 'IN80dj'], ['CT2FUN', 'IN50sf'], ['CT2IOV', 'IM57xd'],
].map(([call, grid]) => ({ call, grid }));

/** The centre `[lat, lon]` of a 6-character locator. */
export function locatorCentre(grid) {
  const g = grid.toUpperCase();
  const lon = -180 + (g.charCodeAt(0) - 65) * 20 + Number(g[2]) * 2 + (g.charCodeAt(4) - 65 + 0.5) * (5 / 60);
  const lat = -90 + (g.charCodeAt(1) - 65) * 10 + Number(g[3]) + (g.charCodeAt(5) - 65 + 0.5) * (2.5 / 60);
  return [lat, lon];
}

/**
 * Whether every `[lat, lon]` lies inside `free` (`{ left, top, right, bottom }`, page pixels, with
 * `tolerance` px) on the flat map at `view`, its centre drawn at `centre` (`data-centre`).
 */
export function fitsFree(view, rect, centre, points, free, tolerance = 1) {
  return points.every((p) => {
    const [x, y] = toScreenMercator(view, rect, p, centre);
    return x >= free.left - tolerance && x <= free.right + tolerance && y >= free.top - tolerance && y <= free.bottom + tolerance;
  });
}
