// A small Chrome DevTools Protocol driver for the browser tests (feature 012, research R2, R7).
// Node built-ins only: the global WebSocket (Node 22+) and child_process. It starts Chrome,
// connects to its page, and records console messages, exceptions and network failures.

import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';

const START_TIMEOUT_MS = 10_000;

/**
 * Starts Chrome and connects to its page. Throws `Chrome did not start: <path>` when its debugging
 * port doesn't answer within 10 s. `ci` turns the sandbox off (CI containers have no user
 * namespaces, research R7).
 */
export async function launchChrome({ chrome, profileDir, headed = false, ci = false, port }) {
  mkdirSync(profileDir, { recursive: true });
  const args = [
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profileDir}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--window-size=1400,900',
    '--enable-unsafe-swiftshader',
    '--use-angle=swiftshader',
  ];
  if (!headed) args.push('--headless=new');
  if (ci) args.push('--no-sandbox');
  args.push('about:blank');
  const child = spawn(chrome, args, { stdio: 'ignore' });
  let spawnError = null;
  child.on('error', (error) => {
    spawnError = error;
  });

  let target = null;
  const deadline = Date.now() + START_TIMEOUT_MS;
  while (!target && !spawnError && Date.now() < deadline) {
    await sleep(200);
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      target = list.find((t) => t.type === 'page') ?? null;
    } catch {
      // Not listening yet.
    }
  }
  if (!target) {
    child.kill();
    throw new Error(`Chrome did not start: ${chrome}${spawnError ? ` (${spawnError.message})` : ''}`);
  }

  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', () => reject(new Error('DevTools connection failed')), { once: true });
  });
  return createPage(socket, child);
}

function createPage(socket, child) {
  let nextId = 0;
  const pending = new Map();
  const waiters = new Map();
  const requests = new Map();
  const events = [];
  const record = (event) => events.push({ at: new Date().toISOString(), ...event });

  socket.addEventListener('message', (message) => {
    const msg = JSON.parse(message.data);
    if (msg.id !== undefined) {
      const call = pending.get(msg.id);
      if (!call) return;
      pending.delete(msg.id);
      if (msg.error) call.reject(new Error(`${call.method}: ${msg.error.message}`));
      else call.resolve(msg.result);
      return;
    }
    const p = msg.params ?? {};
    switch (msg.method) {
      case 'Runtime.consoleAPICalled':
        record({
          kind: 'console',
          level: p.type === 'warning' ? 'warning' : p.type,
          text: p.args.map((a) => a.value ?? a.description ?? a.type).join(' '),
        });
        break;
      case 'Runtime.exceptionThrown': {
        const d = p.exceptionDetails;
        record({ kind: 'exception', text: `${d.exception?.description ?? d.text} @ ${d.url ?? ''}:${d.lineNumber}` });
        break;
      }
      case 'Log.entryAdded':
        record({ kind: 'console', level: p.entry.level, source: p.entry.source, text: `${p.entry.text} ${p.entry.url ?? ''}`.trim() });
        break;
      case 'Network.requestWillBeSent':
        requests.set(p.requestId, p.request.url);
        break;
      case 'Network.responseReceived':
        if (p.response.status >= 400) record({ kind: 'http-error', url: p.response.url, status: p.response.status });
        break;
      case 'Network.loadingFailed':
        record({ kind: 'request-failed', url: requests.get(p.requestId) ?? '', text: p.errorText });
        break;
      default:
        break;
    }
    const waiting = waiters.get(msg.method);
    if (waiting) {
      waiters.delete(msg.method);
      waiting(p);
    }
  });

  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { method, resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  const once = (method, timeout = 15_000) =>
    new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        waiters.delete(method);
        reject(new Error(`timed out after ${timeout} ms waiting for ${method}`));
      }, timeout);
      waiters.set(method, (params) => {
        clearTimeout(timer);
        resolve(params);
      });
    });

  return {
    events,
    send,
    async enable() {
      for (const domain of ['Runtime', 'Log', 'Network', 'Page']) await send(`${domain}.enable`);
    },
    async goto(url) {
      const loaded = once('Page.loadEventFired');
      await send('Page.navigate', { url });
      await loaded;
    },
    async reload() {
      const loaded = once('Page.loadEventFired');
      await send('Page.reload', { ignoreCache: true });
      await loaded;
    },
    /** The expression's value; throws with the page's message when it throws. */
    async eval(expression) {
      const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (r.exceptionDetails) {
        throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
      }
      return r.result.value;
    },
    async screenshot(file) {
      const r = await send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(file, Buffer.from(r.data, 'base64'));
    },
    async emulateDark(on) {
      await send('Emulation.setEmulatedMedia', {
        features: [{ name: 'prefers-color-scheme', value: on ? 'dark' : 'light' }],
      });
    },
    async press(key) {
      const code = { Escape: 27 }[key] ?? 0;
      for (const type of ['keyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key, code: key, windowsVirtualKeyCode: code });
      }
    },
    close() {
      try {
        socket.close();
      } catch {
        // Already closed.
      }
      child.kill();
    },
  };
}
