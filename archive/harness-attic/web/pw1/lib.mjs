// pw1 (decision 0186): helpers for the real-time Serum 2 / OTT Worker runs. From srm1/mt1 lib.mjs (decisions 0177/0173), changed for this use:
//  - the server serves lift/harness/web/ as /coi/ (pw1/, mt1/, srm1/ files are importable from each other) and $PW1_DATA as /coi/data/ (COOP/COEP/CORP headers, localhost only);
//  - Chrome can be started under a prefix command (CHROME_PREFIX="taskset -c 0-3 nice -n 5"): the whole browser then runs on the given cores;
//  - CPU is read per thread from /proc/<pid>/task/<tid>/schedstat (nanoseconds on the CPU) for every process of the Chrome tree, and the machine's per-CPU busy time from /proc/stat.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { Cdp, descendants, freePort, sleep, shutdown } from '../cdp.mjs';

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const WEB = path.resolve(HERE, '..');
export const REPO = path.resolve(HERE, '../../../..');
export const OUT = process.env.PW1_OUT || path.join(REPO, 'lift/out-pw1');
const CHROME = process.env.CHROME_BIN || '/usr/bin/google-chrome';
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.wasm': 'application/wasm', '.json': 'application/json', '.bin': 'application/octet-stream' };
export { sleep, descendants };

export function startServer() {
  const data = process.env.PW1_DATA || path.join(OUT, 'data');
  const server = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    let p = decodeURIComponent(u.pathname);
    const headers = { 'Cache-Control': 'no-store' };
    if (!p.startsWith('/coi/')) { res.writeHead(404).end('use /coi/'); return; }
    p = p.slice(4);
    headers['Cross-Origin-Opener-Policy'] = 'same-origin';
    headers['Cross-Origin-Embedder-Policy'] = 'require-corp';
    headers['Cross-Origin-Resource-Policy'] = 'same-origin';
    if (p.includes('..')) { res.writeHead(403).end('forbidden'); return; }
    const file = p.startsWith('/data/') ? path.join(data, p.slice(6)) : path.join(WEB, p);
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

export async function launch({ profile, args = [], prefix = [] }) {
  const port = await freePort();
  fs.mkdirSync(profile, { recursive: true });
  const a = ['--headless=new', '--no-sandbox', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-dev-shm-usage', '--disable-gpu', '--mute-audio',
    '--disable-renderer-backgrounding', '--disable-background-timer-throttling', ...args, 'about:blank'];
  const cmd = prefix.length ? prefix[0] : CHROME, cargs = prefix.length ? [...prefix.slice(1), CHROME, ...a] : a;
  const proc = spawn(cmd, cargs, { stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => { if (stderr.length < 100000) stderr += d; });
  let ver = null;
  for (let i = 0; i < 150 && !ver; i++) {
    try { ver = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); } catch { await sleep(100); }
  }
  if (!ver) { proc.kill('SIGKILL'); throw new Error('Chrome did not start: ' + stderr.slice(0, 500)); }
  return { proc, port, profile, version: ver, args: a.filter((x) => !x.startsWith('--remote-debugging-port')), prefix, getStderr: () => stderr };
}

const typeOf = (pid) => { try { const c = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8'); const m = /--type=([a-z-]+)/.exec(c); return m ? m[1] : 'browser'; } catch { return '?'; } };

// ns on the CPU of every thread of the Chrome process tree: Map "pid/tid" -> {name, proc, ns}
export function threadNs(rootPid) {
  const m = new Map();
  for (const pid of descendants(rootPid)) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    const proc = typeOf(pid);
    for (const tid of tids) {
      try {
        const name = fs.readFileSync(`/proc/${pid}/task/${tid}/comm`, 'utf8').trim();
        const ss = fs.readFileSync(`/proc/${pid}/task/${tid}/schedstat`, 'utf8').split(' ');
        m.set(`${pid}/${tid}`, { name, proc, ns: +ss[0], rd: +ss[1] });
      } catch { /* gone */ }
    }
  }
  return m;
}

export function nsDiff(before, after) {
  const out = [];
  for (const [k, a] of after) {
    const b = before.get(k);
    const d = a.ns - (b ? b.ns : 0);
    out.push({ key: k, name: a.name, proc: a.proc, ms: d / 1e6, delayMs: (a.rd - (b ? b.rd : 0)) / 1e6, new: !b });
  }
  return out.sort((x, y) => y.ms - x.ms);
}

// /proc/stat: per-CPU busy jiffies (all but idle and iowait), total jiffies. 1 jiffy = 10 ms.
export function cpuStat() {
  const o = {};
  for (const l of fs.readFileSync('/proc/stat', 'utf8').split('\n')) {
    const m = /^(cpu\d*)\s+(.*)$/.exec(l); if (!m) continue;
    const f = m[2].split(/\s+/).map(Number);
    const idle = f[3] + (f[4] || 0), tot = f.slice(0, 8).reduce((s, x) => s + x, 0);
    o[m[1]] = { busy: tot - idle, tot, nice: f[1] };
  }
  return o;
}

export function pssMB(root) { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/smaps_rollup`, 'utf8').match(/^Pss:\s+(\d+)/m)[1]; } catch {} } return r / 1024; }
export function rssMB(root) { let r = 0; for (const p of descendants(root)) { try { r += +fs.readFileSync(`/proc/${p}/status`, 'utf8').match(/VmRSS:\s+(\d+)/)[1]; } catch {} } return r / 1024; }
export function topOthers(root, n = 6) {
  const mine = new Set(descendants(root));
  const rows = [];
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d) || mine.has(+d)) continue;
    try {
      const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8');
      const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
      const cpu = (+f[11] + +f[12]) * 10;
      const nm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
      if (f[0] === 'R') rows.push({ pid: +d, name: nm, state: f[0], nice: +f[16] });
    } catch { /* gone */ }
  }
  return rows.slice(0, 40);
}

// fn({S, evalJSON, waitFor, cdp, chrome, base, out}); page = path under /coi/ (with query)
export async function withBrowser({ name, page, args = [], prefix = [], timeoutS = 600 }, fn) {
  const { server, port } = await startServer();
  const profile = path.join(OUT, 'chrome-profile', name);
  fs.rmSync(profile, { recursive: true, force: true });
  const chrome = await launch({ profile, args, prefix });
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
  const pollLog = async () => {
    const o = JSON.parse(await evalJSON(`JSON.stringify({err: window.__error || null, log: (window.__log || []).slice(${logSeen})})`));
    for (const l of o.log) console.error(`[${((Date.now() - t0) / 1000).toFixed(1)}s] ${l}`);
    logSeen += o.log.length;
    if (o.err) throw new Error('page error: ' + o.err);
  };
  const waitFor = async (flag, secs = timeoutS, every = 100) => {
    const deadline = Date.now() + secs * 1000;
    while (Date.now() < deadline) {
      if (await evalJSON(`window.${flag} === true`)) { await pollLog(); return true; }
      await pollLog();
      await sleep(every);
    }
    throw new Error(`page did not set ${flag} in ${secs} s; console: ` + JSON.stringify(cons.slice(-5)));
  };
  const out = { name, page, prefix, started: new Date().toISOString(), loadavg_before: fs.readFileSync('/proc/loadavg', 'utf8').trim() };
  let result, error = null;
  try {
    out.url = `${base}${page}`;
    await S('Page.navigate', { url: out.url });
    result = await fn({ S, evalJSON, waitFor, cdp, chrome, base, console: cons, out });
  } catch (e) { error = String(e && e.stack || e); }
  out.loadavg_after = fs.readFileSync('/proc/loadavg', 'utf8').trim();
  out.chrome = { product: chrome.version.Browser, v8: chrome.version['V8-Version'], args: chrome.args, prefix };
  out.console = cons.slice(0, 40);
  out.error = error;
  const pids = descendants(chrome.proc.pid);
  await shutdown(chrome, cdp);
  server.close();
  out.chrome_pids_left = pids.filter((pid) => { try { process.kill(pid, 0); return true; } catch { return false; } }).length;
  fs.rmSync(profile, { recursive: true, force: true });
  return { out, result, error };
}

export function pct(arr, qs = [50, 95, 99]) {
  const a = Float64Array.from(arr).sort();
  const o = { n: a.length, min: a[0], max: a[a.length - 1], mean: a.reduce((s, x) => s + x, 0) / (a.length || 1) };
  for (const q of qs) o['p' + q] = a[Math.min(a.length - 1, Math.floor((q / 100) * a.length))];
  return o;
}

export const argv = Object.fromEntries(process.argv.slice(2).map((s) => { const i = s.indexOf('='); return i < 0 ? [s, '1'] : [s.slice(0, i), s.slice(i + 1)]; }));
