// Pure helpers of the browser test runner (feature 012, contracts/browser-test.md): arguments,
// finding Chrome, judging browser events, step bookkeeping, waiting, and the ADIF records it
// sends. No I/O here, so `web/browser-lib.test.js` covers it on every change.

import path from 'node:path';

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
export function qsoAdif({ call, band, mode, grid, at }) {
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
    '<EOR>',
  ].join(' ');
}
