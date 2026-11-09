// Copied from lift/harness/web/cdp.mjs (Chrome launcher, CDP client, shutdown - without the RssSampler); the harness is not imported. Only this line is new.
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
