// What the screen meter and the beacon cost the page (decision 0192): the CPU of the page's main thread (the renderer's tid == pid thread), of the engine's Worker and of the whole renderer / GPU / browser processes of an isolated headless Chrome,
// while the REAL engine idles at the default entrance, with and without the measurement. It sets nothing and sends no input: it opens the page, waits for the first idle, lets the engine settle, and reads /proc twice.
// usage: FL_ASSETS_DIR=<dir whose manifest.json is the real engine's> node test/e2e/display-cost.mjs --query "beacon=0" [--out DIR] [--port N] [--settle-s 15] [--measure-s 20] [--tag name]
// --query is appended to the default entrance ("" = the entrance the owner opens). FL_PERF_LOG is set to a file in --out so that the run never writes to the production log. Numbers are CPU seconds per second on THIS machine (16 cores, load differs).
import fs from 'node:fs';
import path from 'node:path';
import { start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';
import { openBrowser, sleep } from './browser.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> node test/e2e/display-cost.mjs --query "beacon=0"'); process.exit(2); }
const query = opt('query', '');
const tag = opt('tag', query || 'default');
const out = path.resolve(opt('out', '/tmp/opencode/cbeacon1/cost'));
fs.mkdirSync(out, { recursive: true });
const settleS = Number(opt('settle-s', '15')), measureS = Number(opt('measure-s', '20'));
const port = Number(opt('port', '4300'));
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);
const HZ = 100; // CLK_TCK
const ticksOf = (stat) => { const f = stat.slice(stat.lastIndexOf(')') + 2).split(' '); return Number(f[11]) + Number(f[12]); }; // utime + stime

const children = (root) => {
  const all = new Map();
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    try { const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8'); const ppid = Number(st.slice(st.lastIndexOf(')') + 2).split(' ')[1]); all.set(Number(d), ppid); } catch { /* gone */ }
  }
  const mine = new Set([root]);
  for (let grew = true; grew;) { grew = false; for (const [pid, ppid] of all) if (!mine.has(pid) && mine.has(ppid)) { mine.add(pid); grew = true; } }
  return [...mine];
};
function sample(root) {
  const row = { t: Date.now(), procs: {}, threads: {} };
  for (const pid of children(root)) {
    let cmd = '';
    try { cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8'); } catch { continue; }
    const type = /--type=([a-z-]+)/.exec(cmd)?.[1] ?? 'browser';
    try { row.procs[pid] = { type, ticks: ticksOf(fs.readFileSync(`/proc/${pid}/stat`, 'utf8')) }; } catch { continue; }
    if (type !== 'renderer') continue;
    for (const tid of fs.readdirSync(`/proc/${pid}/task`)) {
      try {
        const comm = fs.readFileSync(`/proc/${pid}/task/${tid}/comm`, 'utf8').trim();
        // this Chrome (156, headless=new) names the renderer's main thread after the process ("chrome", tid == pid), not CrRendererMain; the Worker is "DedicatedWorker", the compositor thread "Compositor"
        const isMain = Number(tid) === pid;
        if (isMain || /DedicatedWorker|Compositor/.test(comm)) row.threads[`${pid}/${tid}`] = { comm: isMain ? 'main' : comm, ticks: ticksOf(fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8')) };
      } catch { /* gone */ }
    }
  }
  return row;
}

const server = start(path.join(APP, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: APP, env: { ...process.env, FL_PERF_LOG: path.join(out, `perf-log-${tag.replace(/\W+/g, '_')}.jsonl`) }, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
const browser = await openBrowser(out, 'cost-' + tag.replace(/\W+/g, '_'));
const page = await browser.newPage({ width: 2400, height: 1300 });
const result = { tag, query, browser: browser.version, userAgent: browser.userAgent, settleS, measureS };
try {
  const t0 = Date.now();
  await page.goto(`http://127.0.0.1:${port}/${query ? '?' + query : ''}`);
  await page.waitFor("window.__app && window.__app.timeline && typeof window.__app.timeline.idle === 'number'", 400000, 'first idle');
  result.firstIdleSeconds = (await page.ev('window.__app.timeline.idle')) / 1000;
  result.kind = await page.ev('window.__app.engineKind');
  say(`${tag}: first idle ${result.firstIdleSeconds.toFixed(1)} s, engine ${result.kind}; settling ${settleS} s`);
  await sleep(settleS * 1000);
  const upd = async () => page.ev('JSON.stringify({ total: window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total, idles: window.__app.idles.length, sent: window.__app.perf.sent, meter: window.__app.perf.display.isRunning })').then(JSON.parse);
  const a = sample(browser.pid), u0 = await upd();
  await sleep(measureS * 1000);
  const b = sample(browser.pid), u1 = await upd();
  const dt = (b.t - a.t) / 1000;
  const main = Object.entries(b.threads).filter(([, v]) => v.comm === 'main').sort((x, y) => (b.threads[y[0]].ticks - (a.threads[y[0]]?.ticks ?? 0)) - (b.threads[x[0]].ticks - (a.threads[x[0]]?.ticks ?? 0)))[0];
  const per = (k, v) => ({ comm: v.comm, cores: (v.ticks - (a.threads[k]?.ticks ?? 0)) / HZ / dt });
  const sumType = (type) => Object.entries(b.procs).filter(([, v]) => v.type === type).reduce((s, [k, v]) => s + (v.ticks - (a.procs[k]?.ticks ?? 0)) / HZ / dt, 0);
  const pidOf = main ? main[0].split('/')[0] : null;
  result.windowSeconds = dt;
  result.pageMainThreadCores = main ? per(main[0], main[1]).cores : null;
  result.workerCores = Object.entries(b.threads).filter(([k, v]) => k.startsWith(pidOf + '/') && /DedicatedWorker/.test(v.comm)).reduce((s, [k, v]) => s + per(k, v).cores, 0);
  result.compositorCores = Object.entries(b.threads).filter(([k, v]) => k.startsWith(pidOf + '/') && /Compositor/.test(v.comm)).reduce((s, [k, v]) => s + per(k, v).cores, 0);
  result.rendererCores = sumType('renderer');
  result.gpuCores = sumType('gpu-process');
  result.browserCores = sumType('browser');
  result.totalCores = Object.entries(b.procs).reduce((s, [k, v]) => s + (v.ticks - (a.procs[k]?.ticks ?? 0)) / HZ / dt, 0);
  result.windowUpdatesPerSecond = (u1.total - u0.total) / dt;
  result.idlesPerSecond = (u1.idles - u0.idles) / dt;
  result.beaconsSent = u1.sent;
  result.meterRunning = u1.meter;
  if (u1.meter) {
    const d = JSON.parse(await page.ev('JSON.stringify(window.__app.perf.displaySnapshot())'));
    result.rafHzSinceIdle = d.raf.sinceIdle?.hz ?? null;
    result.rafPerSecondRecent = d.raf.recent10s.perSecond;
    result.freshPerSecondRecent = d.canvas?.recent10s.freshPerSecond ?? null;
    result.longTasksSinceIdle = d.longTasks.sinceIdle;
  }
  result.load = fs.readFileSync('/proc/loadavg', 'utf8').trim();
  say(`${tag}: page main thread ${result.pageMainThreadCores?.toFixed(4)} cores, Worker ${result.workerCores.toFixed(3)}, compositor ${result.compositorCores.toFixed(4)}, renderer ${result.rendererCores.toFixed(3)}, gpu ${result.gpuCores.toFixed(3)}, browser ${result.browserCores.toFixed(3)}, total ${result.totalCores.toFixed(3)}; updates ${result.windowUpdatesPerSecond.toFixed(1)}/s; meter ${u1.meter}; load ${result.load}`);
} catch (e) {
  result.error = String(e.stack ?? e);
  say('FAILED: ' + result.error);
}
fs.writeFileSync(path.join(out, `cost-${tag.replace(/\W+/g, '_')}-${Date.now()}.json`), JSON.stringify(result, null, 1));
await page.close().catch(() => {});
await browser.close().catch(() => {});
await server.stop();
