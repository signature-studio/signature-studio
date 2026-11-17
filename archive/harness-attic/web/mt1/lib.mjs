// mt1 (decision 0173) helpers: a localhost-only static server with COOP/COEP (the same three headers as app/shared/headers.ts ISOLATION_HEADERS when coi=1), a throw-away headless Chrome,
// CDP over node's WebSocket (../cdp.mjs, read-only), and per-thread CPU time of the Chrome process tree read from /proc (utime+stime in clock ticks; the tick is 10 ms).
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { Cdp, descendants, freePort, sleep, shutdown } from '../cdp.mjs';

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO = path.resolve(HERE, '../../../..');
export const OUT = process.env.MT1_OUT || path.join(REPO, 'lift/out-mt1');
const CHROME = process.env.CHROME_BIN || '/usr/bin/google-chrome';
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json', '.bin': 'application/octet-stream' };
export { sleep };

// /coi/<p> and /plain/<p>. A path starting with /data/ is served from MT1_DATA (default lift/out-mt1/data); everything else from this directory.
export function startServer() {
  const data = process.env.MT1_DATA || path.join(OUT, 'data');
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
    const file = p.startsWith('/data/') ? path.join(data, p.slice(6)) : path.join(HERE, p);
    fs.stat(file, (err, st) => {
      if (err || !st.isFile()) { res.writeHead(404).end('not found: ' + p); return; }
      headers['Content-Type'] = TYPES[path.extname(file)] || 'application/octet-stream';
      headers['Content-Length'] = st.size;
      res.writeHead(200, headers);
      fs.createReadStream(file).pipe(res);
    });
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({ server, port: server.address().port })));
}

export async function launch({ profile, args = [] }) {
  const port = await freePort();
  fs.mkdirSync(profile, { recursive: true });
  const a = ['--headless=new', '--no-sandbox', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-dev-shm-usage', '--disable-gpu', '--mute-audio', ...args, 'about:blank'];
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

// CPU ticks (utime+stime) of every thread of the Chrome process tree: Map "pid/tid" -> {name, ticks}. Thread names: DedicatedWorker -> "DedicatedWorker thread"; AudioWorklet -> "AudioWorklet"... (comm, 15 chars)
export function threadCpu(rootPid) {
  const m = new Map();
  for (const pid of descendants(rootPid)) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const name = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
        const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { name, ticks: +f[11] + +f[12] });
      } catch { /* gone */ }
    }
  }
  return m;
}

export function cpuDiff(before, after) {
  const out = [];
  for (const [k, a] of after) {
    const b = before.get(k);
    const d = a.ticks - (b ? b.ticks : 0);
    if (d > 0) out.push({ key: k, name: a.name, ms: d * 10 });
  }
  return out.sort((x, y) => y.ms - x.ms);
}

export const clkTck = 100;

// fn({S, evalJSON, waitDone, cdp, chrome, base, console, out}); page = path under /coi/ or /plain/ (with query)
export async function withBrowser({ name, page, args = [], timeoutS = 300 }, fn) {
  const { server, port } = await startServer();
  const profile = path.join(OUT, 'chrome-profile', name);
  fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launch({ profile, args });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const cons = [];
  const t0 = Date.now();
  cdp.on((m) => {
    const who = m.sessionId ? 'w' + m.sessionId.slice(0, 4) : 'page';
    if (m.method === 'Runtime.consoleAPICalled') cons.push({ t: Date.now() - t0, who, type: m.params.type, text: m.params.args.map((x) => x.value ?? x.description ?? '').join(' ').slice(0, 400) });
    else if (m.method === 'Runtime.exceptionThrown') cons.push({ t: Date.now() - t0, who, type: 'exception', text: JSON.stringify(m.params.exceptionDetails).slice(0, 800) });
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
      await sleep(250);
      const o = JSON.parse(await evalJSON(`JSON.stringify({done: window.__done === true, err: window.__error || null, log: (window.__log || []).slice(${logSeen})})`));
      for (const l of o.log) console.error(`[${((Date.now() - t0) / 1000).toFixed(1)}s] ${l}`);
      logSeen += o.log.length;
      if (o.err) throw new Error('page error: ' + o.err);
      if (o.done) return JSON.parse(await evalJSON('JSON.stringify(window.__result)'));
    }
    throw new Error('page did not finish in ' + secs + ' s; console: ' + JSON.stringify(cons.slice(-5)));
  };
  const out = { name, page, started: new Date().toISOString(), loadavg_before: fs.readFileSync('/proc/loadavg', 'utf8').trim(), hardwareConcurrencyOfHost: (await import('node:os')).cpus().length };
  let result, error = null;
  try {
    out.url = `${base}${page}`;
    await S('Page.navigate', { url: out.url });
    result = await fn({ S, evalJSON, waitDone, cdp, chrome, base, console: cons, out });
  } catch (e) { error = String(e && e.stack || e); }
  out.loadavg_after = fs.readFileSync('/proc/loadavg', 'utf8').trim();
  out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], userAgent: chrome.version['User-Agent'], args: chrome.args };
  out.console = cons.slice(0, 40);
  out.error = error;
  out.result = result;
  const pids = descendants(chrome.proc.pid);
  await shutdown(chrome, cdp);
  server.close();
  out.chrome_pids_left = pids.filter((pid) => { try { process.kill(pid, 0); return true; } catch { return false; } }).length;
  fs.rmSync(profile, { recursive: true, force: true });
  const rdir = process.env.MT1_RESULTS || path.join(OUT, 'results');
  fs.mkdirSync(rdir, { recursive: true });
  fs.writeFileSync(path.join(rdir, `${name}.json`), JSON.stringify(out, null, 1));
  return out;
}

export function pct(arr, qs = [50, 90, 99, 99.9]) {
  const a = Float64Array.from(arr).sort();
  const o = { n: a.length, min: a[0], max: a[a.length - 1], mean: a.reduce((s, x) => s + x, 0) / (a.length || 1) };
  for (const q of qs) o['p' + q] = a[Math.min(a.length - 1, Math.floor((q / 100) * a.length))];
  return o;
}

export const argv = Object.fromEntries(process.argv.slice(2).map((s) => { const i = s.indexOf('='); return i < 0 ? [s, '1'] : [s.slice(0, i), s.slice(i + 1)]; }));
