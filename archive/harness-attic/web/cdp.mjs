// Dependency-free Chrome DevTools Protocol helper: launch a headless Chrome with a throw-away profile, talk CDP over node's built-in
// WebSocket, sample the memory of the Chrome process tree from /proc, shut everything down. Only the processes that descend from the
// Chrome started here are looked at or signalled; other browsers on the machine are not touched.
import fs from 'node:fs';
import net from 'node:net';
import { spawn } from 'node:child_process';

const CHROME = process.env.CHROME_BIN || '/usr/bin/google-chrome';
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export function freePort() {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); });
    s.on('error', reject);
  });
}

function procTable() {
  const out = new Map();
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    try {
      const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8');
      const ppid = +st.slice(st.lastIndexOf(')') + 2).split(' ')[1];
      out.set(+d, ppid);
    } catch { /* process gone */ }
  }
  return out;
}

export function descendants(root) {
  const t = procTable();
  const kids = new Map();
  for (const [pid, ppid] of t) { if (!kids.has(ppid)) kids.set(ppid, []); kids.get(ppid).push(pid); }
  const out = [root];
  for (let i = 0; i < out.length; i++) for (const k of kids.get(out[i]) || []) out.push(k);
  return out;
}

function procInfo(pid, withPss) {
  try {
    const status = fs.readFileSync(`/proc/${pid}/status`, 'utf8');
    const num = (k) => { const m = status.match(new RegExp('^' + k + ':\\s+(\\d+) kB', 'm')); return m ? +m[1] : 0; };
    const cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8');
    const m = cmd.match(/--type=([A-Za-z-]+)/);
    const type = m ? (m[1] === 'utility' && /NetworkService/.test(cmd) ? 'network' : m[1]) : 'browser';
    let pss = null;
    if (withPss) try {
      const sm = fs.readFileSync(`/proc/${pid}/smaps_rollup`, 'utf8');
      const g = (k) => { const mm = sm.match(new RegExp('^' + k + ':\\s+(\\d+) kB', 'm')); return mm ? +mm[1] : 0; };
      pss = { pss: g('Pss'), anon: g('Pss_Anon'), file: g('Pss_File'), shmem: g('Pss_Shmem') };
    } catch { /* not readable */ }
    let cpuMs = null;
    try {
      const st = fs.readFileSync(`/proc/${pid}/stat`, 'utf8');
      const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
      cpuMs = Math.round((+f[11] + +f[12]) * 10);
    } catch { /* gone */ }
    return { pid, type, cpuMs, threads: (+((status.match(/^Threads:\s+(\d+)/m) || [0, 0])[1])), rssKB: num('VmRSS'), hwmKB: num('VmHWM'), vmSizeKB: num('VmSize'), swapKB: num('VmSwap'), pss };
  } catch { return null; }
}

// rss samples of the tree of one Chrome: {t, byType:{renderer:kB,...}, totalKB}; peaks: the largest sample of each type and of the sum
export class RssSampler {
  constructor(root, everyMs = 250, withPss = false) { this.withPss = withPss; this.root = root; this.everyMs = everyMs; this.samples = []; this.hwm = new Map(); this.t0 = Date.now(); this.timer = null; }
  start() { this.timer = setInterval(() => this.sample(), this.everyMs); this.sample(); }
  sample() {
    const byType = {};
    const procs = [];
    let total = 0;
    let pssTotal = 0;
    for (const pid of descendants(this.root)) {
      const i = procInfo(pid, this.withPss);
      if (!i) continue;
      byType[i.type] = (byType[i.type] || 0) + i.rssKB;
      total += i.rssKB;
      pssTotal += i.pss ? i.pss.pss : 0;
      procs.push({ pid: i.pid, type: i.type, cpuMs: i.cpuMs, threads: i.threads, rssKB: i.rssKB, hwmKB: i.hwmKB, pss: i.pss });
      this.hwm.set(pid, i);
    }
    this.samples.push({ t: Date.now() - this.t0, byType, totalKB: total, pssTotalKB: pssTotal, procs });
  }
  stop() { clearInterval(this.timer); this.sample(); }
  summary() {
    // per process: the largest VmRSS (and PSS) seen over the samples; VmHWM of the kernel is not used for the peak (it was lower than a
    // VmRSS read earlier in one run, so it is only kept as a second figure)
    const per = new Map();
    for (const s of this.samples) for (const p of s.procs) {
      const e = per.get(p.pid) || { pid: p.pid, type: p.type, peakRssKB: 0, peakPssKB: 0, hwmKB: 0, peakAtMs: 0 };
      if (p.rssKB > e.peakRssKB) { e.peakRssKB = p.rssKB; e.peakAtMs = s.t; }
      e.cpuMs = Math.max(e.cpuMs || 0, p.cpuMs || 0);
      e.threadsMax = Math.max(e.threadsMax || 0, p.threads || 0);
      e.peakPssKB = Math.max(e.peakPssKB, p.pss ? p.pss.pss : 0);
      e.hwmKB = Math.max(e.hwmKB, p.hwmKB);
      per.set(p.pid, e);
    }
    const procs = [...per.values()];
    const renderers = procs.filter((p) => p.type === 'renderer').sort((x, y) => y.peakRssKB - x.peakRssKB);
    const busiest = renderers[0] || null;
    const series = busiest ? this.samples.map((s) => { const p = s.procs.find((q) => q.pid === busiest.pid); return p ? { t: s.t, rssMB: Math.round(p.rssKB / 1024), pssMB: p.pss ? Math.round(p.pss.pss / 1024) : null, anonMB: p.pss ? Math.round(p.pss.anon / 1024) : null } : null; }).filter(Boolean) : [];
    let peakTotal = 0; let peakPss = 0;
    for (const s of this.samples) { peakTotal = Math.max(peakTotal, s.totalKB); peakPss = Math.max(peakPss, s.pssTotalKB); }
    return { samples: this.samples.length, everyMs: this.everyMs, busiestRenderer: busiest, otherRenderersPeakRssKB: renderers.slice(1).map((r) => r.peakRssKB),
      peakTreeRssKB: peakTotal, peakTreePssKB: peakPss, processes: procs, busiestRendererSeries: series };
  }
}

export async function launchChrome({ profile, extraArgs = [], url = 'about:blank', stdoutFile = null, devShm = false }) {
  const port = await freePort();
  fs.mkdirSync(profile, { recursive: true });
  const args = [
    '--headless=new', '--no-sandbox', '--disable-gpu', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', ...(devShm ? [] : ['--disable-dev-shm-usage']), '--mute-audio', ...extraArgs, url,
  ];
  const proc = spawn(CHROME, args, { stdio: ['ignore', stdoutFile ? fs.openSync(stdoutFile, 'w') : 'ignore', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => { if (stderr.length < 200000) stderr += d; });
  let ws = null;
  for (let i = 0; i < 100 && !ws; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/version`);
      ws = (await r.json());
    } catch { await sleep(100); }
  }
  if (!ws) { proc.kill('SIGKILL'); throw new Error('Chrome did not start: ' + stderr.slice(0, 500)); }
  return { proc, port, profile, version: ws, args, getStderr: () => stderr };
}

export class Cdp {
  constructor(wsUrl) {
    this.nextId = 1; this.pending = new Map(); this.handlers = []; this.closed = false;
    this.ws = new WebSocket(wsUrl);
    this.opened = new Promise((res, rej) => { this.ws.onopen = res; this.ws.onerror = (e) => rej(new Error('ws error')); });
    this.ws.onclose = () => { this.closed = true; for (const [, p] of this.pending) p.reject(new Error('ws closed')); };
    this.ws.onmessage = (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id && this.pending.has(m.id)) {
        const p = this.pending.get(m.id); this.pending.delete(m.id);
        if (m.error) p.reject(new Error(m.error.message + ' (' + JSON.stringify(m.error.data || '') + ')')); else p.resolve(m.result);
      } else for (const h of this.handlers) h(m);
    };
  }
  on(h) { this.handlers.push(h); }
  send(method, params = {}, sessionId) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify(sessionId ? { id, method, params, sessionId } : { id, method, params }));
    });
  }
  close() { try { this.ws.close(); } catch { /* ignore */ } }
}

export async function shutdown(chrome, cdp) {
  const pids = descendants(chrome.proc.pid);
  const exited = new Promise((r) => chrome.proc.once('exit', r));
  try { await Promise.race([cdp.send('Browser.close'), sleep(5000)]); } catch { /* browser closed the socket */ }
  cdp.close();
  await Promise.race([exited, sleep(8000)]);
  await sleep(500);
  for (const pid of pids.reverse()) {
    try {
      if (!fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes(`--user-data-dir=${chrome.profile}`)
        && !fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes('--type=')) continue;
      process.kill(pid, 'SIGKILL');
    } catch { /* gone */ }
  }
}
