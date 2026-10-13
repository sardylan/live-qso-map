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
  waitFor,
  qsoAdif,
} from './lib.mjs';
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
];

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

const shot = (name) => page.screenshot(path.join(out, `${name}.png`));

async function setTheme(choice) {
  await q(`(() => { const s = document.getElementById('theme'); s.value = '${choice}'; s.dispatchEvent(new Event('change')); })()`);
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
    // The history starts empty after a restart.
    await sendQso({ call: 'DL2BBB', band: '40m', grid: 'JO62' });
    await sendQso({ call: 'I3BBB', band: '30m', grid: 'JN45' });
    await waitFor(
      async () => {
        const calls = await boxCalls();
        return calls.includes('DL2BBB') && calls.includes('I3BBB') && (await markerCount()) === 2;
      },
      { timeout: STEP_MS, what: 'the QSOs sent after the restart' },
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
    await waitFor(async () => (await legendRows()).length === 10 && (await markerCount()) === 2, {
      timeout: STEP_MS,
      what: 'every band shown again after unticking WARC',
    });
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
