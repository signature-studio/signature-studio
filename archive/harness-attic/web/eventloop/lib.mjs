// Shared helpers of the event-loop prototypes (docs/investigations/browser-event-loop.md). Dependency-free; uses ../cdp.mjs read-only.
//   startServer: localhost-only static server. /coi/<file> adds COOP same-origin + COEP require-corp (+ CORP same-origin), /plain/<file> adds
//                nothing. <file> is looked up in this directory, then in $EL_BUILD (default lift/out-web2/mb) under the name build/<file>.
//   withBrowser: one throw-away headless Chrome (own profile directory), one page, CDP session, worker auto-attach with console capture.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { Cdp, descendants, freePort, sleep, shutdown } from '../cdp.mjs';

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO = path.resolve(HERE, '../../../..');
export const OUT = process.env.EL_OUT || path.join(REPO, 'lift/out-web2');
export const BUILD = process.env.EL_BUILD || path.join(OUT, 'mb');
export const RESULTS = path.join(HERE, 'results');
const CHROME = process.env.CHROME_BIN || '/usr/bin/google-chrome';
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json' };

export function startServer({ port = 0 } = {}) {
  const log = [];
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    let p = decodeURIComponent(u.pathname);
    const headers = { 'Cache-Control': 'no-store' };
    if (p.startsWith('/coi/')) {
      p = p.slice(4);
      headers['Cross-Origin-Opener-Policy'] = 'same-origin';
      headers['Cross-Origin-Embedder-Policy'] = 'require-corp';
      headers['Cross-Origin-Resource-Policy'] = 'same-origin';
    } else if (p.startsWith('/plain/')) p = p.slice(6);
    else { res.writeHead(404).end('use /coi/ or /plain/'); return; }
    if (p.includes('..')) { res.writeHead(403).end('forbidden'); return; }
    const file = p.startsWith('/build/') ? path.join(BUILD, p.slice(7)) : path.join(HERE, p);
    fs.stat(file, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); log.push({ url: req.url, status: 404 }); return; }
      headers['Content-Type'] = TYPES[path.extname(file)] || 'application/octet-stream';
      headers['Content-Length'] = st.size;
      res.writeHead(200, headers);
      fs.createReadStream(file).pipe(res);
    });
  });
  return new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve({ server, port: server.address().port, log })));
}

export async function launch({ profile, args = [], gpu = false, mute = true }) {
  const port = await freePort();
  fs.mkdirSync(profile, { recursive: true });
  const a = ['--headless=new', '--no-sandbox', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-dev-shm-usage'];
  if (!gpu) a.push('--disable-gpu');
  if (mute) a.push('--mute-audio');
  a.push(...args, 'about:blank');
  const proc = spawn(CHROME, a, { stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => { if (stderr.length < 100000) stderr += d; });
  let ver = null;
  for (let i = 0; i < 100 && !ver; i++) {
    try { ver = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); } catch { await sleep(100); }
  }
  if (!ver) { proc.kill('SIGKILL'); throw new Error('Chrome did not start: ' + stderr.slice(0, 500)); }
  return { proc, port, profile, version: ver, args: a.filter((x) => !x.startsWith('--remote-debugging-port')), getStderr: () => stderr };
}

// fn({S, evalJSON, waitDone, cdp, url, base, log, chrome, sessionId, newPage}) ; returns what fn returns. page = path under /coi/ or /plain/ (with query)
export async function withBrowser({ name, page, args = [], gpu = false, mute = true, timeoutS = 120, keepProfile = false }, fn) {
  const { server, port } = await startServer();
  const profile = path.join(OUT, 'chrome-profile', name);
  fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launch({ profile, args, gpu, mute });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const console_ = [];
  const t0 = Date.now();
  cdp.on((m) => {
    const who = m.sessionId ? 'w' + m.sessionId.slice(0, 4) : 'page';
    if (m.method === 'Runtime.consoleAPICalled') console_.push({ t: Date.now() - t0, who, type: m.params.type, text: m.params.args.map((x) => x.value ?? x.description ?? '').join(' ').slice(0, 400) });
    else if (m.method === 'Runtime.exceptionThrown') console_.push({ t: Date.now() - t0, who, type: 'exception', text: JSON.stringify(m.params.exceptionDetails).slice(0, 800) });
    else if (m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') {
      cdp.send('Runtime.enable', {}, m.params.sessionId).catch(() => {});
      cdp.send('Runtime.runIfWaitingForDebugger', {}, m.params.sessionId).catch(() => {});
    }
  });
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const S = (method, params) => cdp.send(method, params, sessionId);
  await S('Runtime.enable');
  await S('Page.enable');
  await S('Network.enable');
  await S('Network.setCacheDisabled', { cacheDisabled: true });
  await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
  const base = `http://127.0.0.1:${port}`;
  const evalJSON = async (expr) => {
    const r = await S('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error('evaluate: ' + JSON.stringify(r.exceptionDetails).slice(0, 500));
    return r.result.value;
  };
  let logSeen = 0;
  const waitDone = async (secs = timeoutS) => {
    const deadline = Date.now() + secs * 1000;
    while (Date.now() < deadline) {
      await sleep(200);
      const v = await evalJSON(`JSON.stringify({done: window.__done === true, err: window.__error || null, log: (window.__log || []).slice(${logSeen})})`);
      const o = JSON.parse(v);
      for (const l of o.log) console.error(`[${((Date.now() - t0) / 1000).toFixed(1)}s] ${l}`);
      logSeen += o.log.length;
      if (o.err) throw new Error('page error: ' + o.err);
      if (o.done) return JSON.parse(await evalJSON('JSON.stringify(window.__result)'));
    }
    throw new Error('page did not finish in ' + secs + ' s; console: ' + JSON.stringify(console_.slice(-5)));
  };
  const out = { name, page, started: new Date().toISOString(), loadavg_before: fs.readFileSync('/proc/loadavg', 'utf8').trim() };
  let result;
  let error = null;
  try {
    const url = `${base}${page}`;
    out.url = url;
    await S('Page.navigate', { url });
    result = await fn({ S, evalJSON, waitDone, cdp, url, base, console: console_, chrome, sessionId, out });
  } catch (e) { error = String(e && e.stack || e); }
  out.loadavg_after = fs.readFileSync('/proc/loadavg', 'utf8').trim();
  out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'], args: chrome.args };
  out.console = console_.slice(0, 40);
  out.error = error;
  out.result = result;
  const pids = descendants(chrome.proc.pid);
  await shutdown(chrome, cdp);
  server.close();
  const left = pids.filter((pid) => { try { process.kill(pid, 0); return true; } catch { return false; } });
  out.chrome_pids_left = left.length;
  if (!keepProfile) fs.rmSync(profile, { recursive: true, force: true });
  fs.mkdirSync(RESULTS, { recursive: true });
  fs.writeFileSync(path.join(RESULTS, `${name}.json`), JSON.stringify(out, null, 1));
  return out;
}

export function pct(arr, qs = [50, 95, 99]) {
  const a = Float64Array.from(arr).sort();
  const o = { n: a.length, min: a[0], max: a[a.length - 1], mean: a.reduce((s, x) => s + x, 0) / (a.length || 1) };
  for (const q of qs) o['p' + q] = a[Math.min(a.length - 1, Math.floor((q / 100) * a.length))];
  return o;
}

export const argv = Object.fromEntries(process.argv.slice(2).map((s) => { const i = s.indexOf('='); return i < 0 ? [s, '1'] : [s.slice(0, i), s.slice(i + 1)]; }));
