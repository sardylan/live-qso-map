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
