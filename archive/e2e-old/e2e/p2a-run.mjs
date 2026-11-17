// Decision 0170: does the engine's virtual clock follow the wall clock? Opens the page against `vite preview` (the REAL engine of a bundle, default configuration D), waits for the first idle and measures, per phase, with the
// engine and the page both running in the same isolated headless Chrome as the other e2e tests:
//   virtual/wall   (virtual ms the engine's clock advanced) / (wall ms), from the last idle of the Worker at the start and the end of the phase
//   idle/s, chained/bounce/timer deliveries per second      the IdleDriver's own counters
//   updates/s      presents of the desktop picture that changed a pixel (display method D: the Worker's 'desk' reports; decisions 0223 / desk2)
//   CPU            thread CPU time of the engine Worker thread and of the page's main thread (/proc/<pid>/task/<tid>/stat, 100 Hz ticks) over the phase / the phase's wall time (1.0 = one full core)
//   input -> next update   p50 / p95 of the page's latency records created in the phase
// usage: FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/p2a-run.mjs [--out DIR] [--wizard-s 20] [--idle-s 60] [--mouse-s 20] [--play-s 30] [--port 4200] [--params 'a=b&c=d']
// FL_ASSETS_DIR is only READ. The measurement beacon log goes to <out>/perf-log.jsonl (FL_PERF_LOG), never into the assets directory's parent. It prints what was measured; it asserts nothing about speed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';
import { openBrowser, sleep } from './browser.mjs';
import { descendants } from './cdp.mjs';
import { keyDown, keyUp, moveTo, pressAt, releaseAt } from './input-helpers.mjs';
import net from 'node:net';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> node test/e2e/p2a-run.mjs [--out DIR]'); process.exit(2); }
const out = path.resolve(opt('out', path.join(os.tmpdir(), 'opencode', 'clk1-run')));
const secs = { wizard: Number(opt('wizard-s', '20')), idle: Number(opt('idle-s', '60')), mouse: Number(opt('mouse-s', '20')), play: Number(opt('play-s', '30')), taps: Number(opt('taps', '30')), clicks: Number(opt('clicks', '0')), raf: Number(opt('raf-s', '0')), freeze: Number(opt('freeze-s', '0')), hide: Number(opt('hide-s', '0')), soak: Number(opt('soak-s', '0')), profile: Number(opt('profile-s', '0')), after: Number(opt('after-s', '15')) };
const extra = opt('params', '');
const basePort = Number(opt('port', '4200'));
const dist = opt('dist', '');
fs.mkdirSync(out, { recursive: true });
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);

const portFree = (p) => new Promise((res) => { const s = net.createServer(); s.once('error', () => res(false)); s.listen(p, '127.0.0.1', () => s.close(() => res(true))); });
let port = basePort;
while (!(await portFree(port))) port++;

const env = { ...process.env, FL_PERF_LOG: path.join(out, 'perf-log.jsonl') };
const server = start(path.join(APP, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort', ...(dist ? ['--outDir', dist] : [])], { cwd: APP, env, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
say(`vite preview on ${port}`);

// ---- thread CPU from /proc ----
function threads(root) {
  const m = new Map();
  const pids = descendants(root);
  const renderers = new Set();
  for (const pid of pids) { try { if (fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes('--type=renderer')) renderers.add(pid); } catch { /* gone */ } }
  for (const pid of pids) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
        const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]), main: tid === String(pid) && renderers.has(pid) });
      } catch { /* gone */ }
    }
  }
  return m;
}
const HZ = 100;

const browser = await openBrowser(out, 'clk');
const page = await browser.newPage({ width: 2400, height: 1300 });
const result = { port, load: [], phases: {}, chrome: browser.version, startedAt: new Date().toISOString() };

async function sample() {
  const a = JSON.parse(await page.ev(`JSON.stringify((() => {
    const a = window.__app; const ids = a.idles; const last = ids[ids.length - 1];
    const ps = a.perf.recorder.snapshot(performance.timeOrigin + performance.now());
    return { abs: performance.timeOrigin + performance.now(), last: last ? { at: last.at, now: last.now, next: last.next } : null, stats: a.driverStats, wasmBytes: a.engineMemoryBytes ?? null, prof: a.details.prof ?? null, updates: ps.updates.total, idles: ps.idles.total, lat: a.latency.records.length, state: a.state };
  })())`));
  return { t: Date.now(), cpu: threads(browser.pid), load: os.loadavg()[0], ...a };
}
function diffCpu(s0, s1, seconds) {
  const rows = [];
  for (const [k, v1] of s1.cpu) {
    const v0 = s0.cpu.get(k);
    const d = (v1.ticks - (v0 ? v0.ticks : 0)) / HZ;
    if (d > 0) rows.push({ k, comm: v1.comm, cpuS: d, core: d / seconds, main: v1.main });
  }
  rows.sort((a, b) => b.cpuS - a.cpuS);
  const worker = rows.filter((r) => /Worker/i.test(r.comm));
  const main = rows.filter((r) => r.main);
  const sum = (l) => l.reduce((x, r) => x + r.cpuS, 0);
  return { workerCore: sum(worker) / seconds, mainCore: sum(main) / seconds, allCore: sum(rows) / seconds, workerCpuS: sum(worker), top: rows.slice(0, 6).map((r) => `${r.comm} ${r.cpuS.toFixed(2)}s`) };
}
function report(name, s0, s1) {
  const wall = (s1.t - s0.t) / 1000;
  const dv = s0.last && s1.last ? (s1.last.now - s0.last.now) / 10000 : null; // virtual ms
  const dw = s0.last && s1.last ? s1.last.at - s0.last.at : null;              // wall ms between those two idles (the Worker's clock)
  const st0 = s0.stats ?? {}, st1 = s1.stats ?? {};
  const rate = (k) => (st1[k] ?? 0) - (st0[k] ?? 0);
  const cpu = diffCpu(s0, s1, wall);
  let prof = null;
  if (s0.prof && s1.prof && s1.prof.n > s0.prof.n) { const n = s1.prof.n - s0.prof.n; const us = (k) => +((s1.prof[k] - s0.prof[k]) / n * 1000).toFixed(1); prof = { cycles: n, flushUs: us('flush'), pumpUs: us('pump'), routingUs: us('routing'), appviewUs: us('appview'), driverUs: us('driver'), postUs: us('post'), totalUs: us('total'), betweenUs: us('gapSum') }; }
  const r = {
    wallS: +wall.toFixed(2), virtualMs: dv, virtualPerWall: dv !== null && dw ? +(dv / dw).toFixed(4) : null,
    idlePerS: +(rate('idleCount') / wall).toFixed(1), deadlinePerS: +(rate('deadlineWakes') / wall).toFixed(1), chainedPerS: +(rate('chained') / wall).toFixed(1), bouncesPerS: +(rate('bounces') / wall).toFixed(1), timerPerS: +(rate('timerWakes') / wall).toFixed(1),
    lagCuts: rate('lagCuts'), maxLagMs: st1.maxLagTicks !== undefined ? +(st1.maxLagTicks / 10000).toFixed(1) : null,
    updatesPerS: +((s1.updates - s0.updates) / wall).toFixed(2), pageIdleMsgPerS: +((s1.idles - s0.idles) / wall).toFixed(1),
    workerCore: +cpu.workerCore.toFixed(3), mainCore: +cpu.mainCore.toFixed(3), allCore: +cpu.allCore.toFixed(3), workerCpuPerVirtualS: dv ? +(cpu.workerCpuS / (dv / 1000)).toFixed(3) : null,
    wasmMBStart: s0.wasmBytes ? +(s0.wasmBytes / 1048576).toFixed(1) : null, wasmMBEnd: s1.wasmBytes ? +(s1.wasmBytes / 1048576).toFixed(1) : null,
    loadStart: +s0.load.toFixed(1), loadEnd: +s1.load.toFixed(1), top: cpu.top, ...(prof ? { prof } : {}),
  };
  result.phases[name] = r;
  say(`PHASE ${name}: ${JSON.stringify(r)}`);
  return r;
}
async function latencyOf(sinceAbs) {
  // the records of the inputs the page sent at or after `sinceAbs` (the page keeps the last 400 only, so the phase is picked by time, not by index)
  return JSON.parse(await page.ev(`JSON.stringify((() => {
    const recs = window.__app.latency.records.filter((r) => r.tPage >= ${sinceAbs});
    const q = (v, p) => { v = v.slice().sort((a, b) => a - b); return v.length ? v[Math.min(v.length - 1, Math.max(0, Math.ceil(p / 100 * v.length) - 1))] : null; };
    const ok = recs.filter((r) => r.dropped === null && r.tIdle !== null);
    const first = (r) => { const f = Object.values(r.frames); return f.length ? Math.min(...f) : null; };
    const total = ok.map((r) => first(r) !== null ? first(r) - r.tPage : null).filter((x) => x !== null);
    const toInject = ok.map((r) => r.tInject - r.tPage), engine = ok.map((r) => r.tIdle - r.tInject), toFrame = ok.map((r) => first(r) !== null ? first(r) - r.tIdle : null).filter((x) => x !== null);
    // only in the instrumented build (tools/prof-patch2.py): the Worker logs "WF <seq> <abs ms>" when it first draws a rectangle after the engine took input <seq>
    const wf = new Map(); for (const m of (document.getElementById('log')?.textContent ?? '').matchAll(/WF (\\d+) ([\\d.]+)/g)) wf.set(Number(m[1]), Number(m[2]));
    const pageToWorkerFrame = ok.filter((r) => wf.has(r.seq)).map((r) => wf.get(r.seq) - r.tPage);
    const workerFrameToPage = ok.filter((r) => wf.has(r.seq) && first(r) !== null).map((r) => first(r) - wf.get(r.seq));
    return { wfN: pageToWorkerFrame.length, pageToWorkerFrameP50: q(pageToWorkerFrame, 50), pageToWorkerFrameP95: q(pageToWorkerFrame, 95), workerFrameToPageP50: q(workerFrameToPage, 50), workerFrameToPageP95: q(workerFrameToPage, 95), n: total.length, of: recs.length, p50: q(total, 50), p95: q(total, 95), max: total.length ? Math.max(...total) : null,
      pageToInjectP50: q(toInject, 50), pageToInjectP95: q(toInject, 95), engineP50: q(engine, 50), engineP95: q(engine, 95), idleToFrameP50: q(toFrame, 50), idleToFrameP95: q(toFrame, 95) };
  })())`));
}
// the SHA-256 of the desktop picture and of the main window's part of it (decisions 0223 / desk2: the page has one canvas and knows no window, so the compositor's picture is read: window.__app.deskDumpBase64(true), the picture as it is,
// and the main window is the largest window of the compositor's list), with the engine's virtual clock at that moment: compared with the baseline SHAs of decision 0147 (native, 16 snapshots; those baselines do not hold on today's engine)
async function shas(tag) {
  const o = JSON.parse(await page.ev(`(async () => {
    const d = await window.__app.deskDumpBase64(true);
    const bin = atob(d.base64); const px = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) px[i] = bin.charCodeAt(i);
    const hex = async (u8) => [...new Uint8Array(await crypto.subtle.digest('SHA-256', u8))].map((b) => b.toString(16).padStart(2, '0')).join('');
    const wins = (d.windows ?? '').split('\\n').map((l) => /^lid (\\d+) at \\((-?\\d+), (-?\\d+)\\) (\\d+)x(\\d+)/.exec(l)).filter(Boolean).map((m) => ({ lid: +m[1], x: +m[2], y: +m[3], w: +m[4], h: +m[5] }));
    const main = wins.slice().sort((a, b) => b.w * b.h - a.w * a.h)[0] ?? null;
    let mainSha = null;
    if (main) {
      const x0 = Math.max(0, main.x), y0 = Math.max(0, main.y), x1 = Math.min(d.width, main.x + main.w), y1 = Math.min(d.height, main.y + main.h);
      const reg = new Uint8Array((x1 - x0) * (y1 - y0) * 4);
      for (let y = y0; y < y1; y++) reg.set(px.subarray((y * d.width + x0) * 4, (y * d.width + x1) * 4), (y - y0) * (x1 - x0) * 4);
      mainSha = await hex(reg);
    }
    const ids = window.__app.idles; const last = ids[ids.length - 1];
    return JSON.stringify({ virtualMs: last ? last.now / 10000 : null, idleN: window.__app.driverStats?.idleCount ?? null, main, windows: wins.length, picture: { w: d.width, h: d.height, sha: await hex(px) }, mainSha });
  })()`));
  result.shas = result.shas ?? {};
  result.shas[tag] = o;
  say(`SHAS ${tag}: virtual ${o.virtualMs} ms; main ${JSON.stringify(o.main)} sha ${o.mainSha?.slice(0, 12)}; picture ${o.picture.w}x${o.picture.h} sha ${o.picture.sha.slice(0, 12)}; ${o.windows} windows`);
}
// Space toggles playback, and whether a press started it cannot be told from the key: playback is told by its effect (the mixer's deadlines make the idle rate > 1,500/s under the new pacing; the window changes
// >= 3 times a second under the former one, whose idle rate does not rise) and Space is pressed again only when it is not in the wanted state after a long wait
const updTotal = async () => Number(await page.ev('window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total'));
const idleTotal = async () => Number(await page.ev('window.__app.driverStats?.idleCount ?? 0'));
async function playingNow() {
  const n0 = await idleTotal(), u0 = await updTotal(), t0 = Date.now();
  await sleep(1500);
  const dt = (Date.now() - t0) / 1000;
  const idleRate = ((await idleTotal()) - n0) / dt, updRate = ((await updTotal()) - u0) / dt;
  return idleRate > 1500;
}
async function setPlaying(want) {
  for (let attempt = 0; attempt < 3; attempt++) {
    if ((await playingNow()) === want) return true;
    await moveTo(page, 960, 700); await sleep(300);
    await keyDown(page, 'Space'); await sleep(40); await keyUp(page, 'Space');
    for (let k = 0; k < 8; k++) { await sleep(500); if ((await playingNow()) === want) return true; }
  }
  return (await playingNow()) === want;
}
// the engine Worker's CDP session: the page session auto-attaches to its dedicated workers (existing ones are reported at once)
let workerSess = null;
browser.cdp.on((m) => { if (m.method === 'Target.attachedToTarget' && m.params?.targetInfo?.type === 'worker') workerSess = m.params.sessionId; });
async function getWorkerSession() {
  if (!workerSess) {
    await page.S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
    for (let i = 0; i < 50 && !workerSess; i++) await sleep(100);
  }
  return workerSess;
}
let currentPhase = 'start';
const phase = async (name, seconds, during) => {
  currentPhase = name;
  const s0 = await sample();
  if (during) await during(seconds); else await sleep(seconds * 1000);
  const s1 = await sample();
  const r = report(name, s0, s1);
  if (name === 'mouse' || name === 'play' || name === 'taps' || name === 'clicks') { r.latency = await latencyOf(s0.abs); say(`  latency ${name}: ${JSON.stringify(r.latency)}`); }
  return r;
};

try {
  const t0 = Date.now();
  const url = `http://127.0.0.1:${port}/${extra ? '?' + extra : ''}`;
  await page.goto(url);
  let idleAt = null;
  while ((Date.now() - t0) / 1000 < 400) {
    await sleep(1000);
    const j = JSON.parse(await page.ev('JSON.stringify({ state: window.__app?.state, kind: window.__app?.engineKind, tl: window.__app?.timeline, err: window.__app?.error })').catch(() => '{}'));
    if (j.state === 'error' || j.state === 'exited') { say(`page ended early: ${JSON.stringify(j).slice(0, 600)}`); break; }
    if (j.tl && typeof j.tl.idle === 'number') { idleAt = j.tl.idle; result.engineKind = j.kind; break; }
  }
  result.firstIdleS = idleAt === null ? null : idleAt / 1000;
  say(`first idle after ${result.firstIdleS} s (engineKind ${result.engineKind})`);
  if (idleAt === null) throw new Error('no first idle');
  result.memSeries = [];
  const memT0 = Date.now();
  const memTimer = setInterval(async () => { try { const v = Number(await page.ev('window.__app.engineMemoryBytes ?? 0')); const ids = await page.ev('(() => { const a = window.__app.idles; const l = a[a.length - 1]; return l ? l.now / 10000 : 0; })()'); result.memSeries.push({ tS: +((Date.now() - memT0) / 1000).toFixed(1), wasmMB: +(v / 1048576).toFixed(1), virtualMs: Math.round(Number(ids)), phase: currentPhase }); } catch { /* the page is busy or gone */ } }, 15000);
  await sleep(3000);
  await phase('wizard', secs.wizard);
  await shas('wizard-end').catch((e) => say('shas failed: ' + e));
  // Enter dismisses the welcome wizard; the main thread then has a long stretch of work (decision 0147: about 4 CPU seconds)
  result.enterIdleN = Number(await page.ev('window.__app.driverStats?.idleCount ?? 0'));
  const sEnter0 = await sample();
  // Enter dismisses the welcome wizard; it is checked in the engine's window list and sent again (up to 4 times) if the wizard is still there: a run in which Enter had no effect measured a modal wizard, not the main window
  // decisions 0223 / desk2: the title of the engine's foreground window (document.title) says whether the welcome wizard is still up (the page holds no window list)
  const wizardUp = async () => page.ev("/^Welcome to FL Studio/.test(window.__app.pageTitle().title)");
  let enterTries = 0;
  for (; enterTries < 4; enterTries++) {
    await keyDown(page, 'Enter'); await sleep(40); await keyUp(page, 'Enter');
    let gone = false;
    for (let k = 0; k < 24 && !gone; k++) { await sleep(500); gone = !(await wizardUp()); }
    if (gone) break;
  }
  result.enterTries = enterTries + 1;
  say(`Enter sent ${result.enterTries} time(s); waiting until the engine is quiet again`);
  {
    let lastN = -1, still = 0;
    const tw = Date.now();
    while (Date.now() - tw < 180000 && still < 6) {
      await sleep(1000);
      const n = Number(await page.ev('window.__app.driverStats?.idleCount ?? 0'));
      const quiet = n !== lastN;
      // quiet = the idle counter moves again after the long stretch: count seconds in which it moved a lot (> 20 idles)
      if (lastN >= 0 && n - lastN > 20) still++; else still = 0;
      lastN = n;
      void quiet;
    }
    const sEnter1 = await sample();
    report('enter-settle', sEnter0, sEnter1);
  }
  await sleep(5000);
  await shas('after-enter').catch((e) => say('shas failed: ' + e));
  await phase('main-idle', secs.idle);
  await phase('mouse', secs.mouse, async (s) => {
    const t1 = Date.now();
    let i = 0;
    while (Date.now() - t1 < s * 1000) {
      const x = 400 + 700 * (0.5 + 0.5 * Math.sin(i / 40)), y = 300 + 400 * (0.5 + 0.5 * Math.sin(i / 23));
      await moveTo(page, Math.round(x), Math.round(y));
      i++;
      await sleep(25);
    }
    result.mouseMoves = i;
  });
  // isolated pointer moves, 700 ms apart, at positions that differ every time (a seeded sequence): the input -> next update time of one event, not of a stream that the engine merges
  await phase('taps', secs.taps * 0.7 + 1, async () => {
    let seed = 12345;
    const rnd = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; };
    for (let i = 0; i < secs.taps; i++) {
      await moveTo(page, Math.round(200 + rnd() * 1500), Math.round(120 + rnd() * 800));
      await sleep(700);
    }
  });
  if (secs.clicks > 0) {
    // button down / up (two input records each) on places that do not stop the engine at a wall (decision 0171: the top toolbar does): mixer Insert-3 and Insert-4 names, the channel rack, the playlist grid
    await phase('clicks', secs.clicks * 1.1 + 1, async () => {
      const pts = [[595, 678], [657, 678], [380, 300], [1100, 500]];
      for (let i = 0; i < secs.clicks; i++) {
        const [x, y] = pts[i % pts.length];
        await moveTo(page, x, y); await sleep(150);
        await pressAt(page, x, y); await sleep(60); await releaseAt(page, x, y);
        await sleep(800);
      }
    });
  }
  await phase('play', secs.play, async (s) => {
    result.playSeen = await setPlaying(true);
    result.playAttempts = 1;
    await sleep(s * 1000);
    result.playStopped = await setPlaying(false);
  });
  // decision 0182: CPU profiles (CDP Profiler, 200 us sampling) of the engine Worker AND the page's main thread over the same stretch, once with the main window idle and, with --profile-play 1, once while the project plays.
  // Self time by function; every wasm frame is one bucket '[wasm]'. usPerIdle = busy self time of the thread / idle cycles of the engine in the stretch.
  const summarize = (profile, cycles) => {
    const byId = new Map(profile.nodes.map((n) => [n.id, n]));
    const self = new Map();
    for (let i = 0; i < profile.samples.length; i++) self.set(profile.samples[i], (self.get(profile.samples[i]) ?? 0) + (profile.timeDeltas[i] ?? 0));
    const agg = new Map();
    let total = 0, idle = 0;
    for (const [id, us] of self) {
      const n = byId.get(id); const cf = n.callFrame;
      const fn = cf.functionName || '(anonymous)';
      const key = /^wasm-function|^\d+$/.test(fn) || cf.url.startsWith('wasm') ? '[wasm]' : `${fn} ${cf.url.split('/').pop()}:${cf.lineNumber}`;
      total += us;
      if (fn === '(idle)') idle += us;
      agg.set(key, (agg.get(key) ?? 0) + us);
    }
    const busy = total - idle;
    const top = [...agg.entries()].filter(([k]) => !k.startsWith('(idle)')).sort((a, b) => b[1] - a[1]).slice(0, 24).map(([k, us]) => ({ fn: k, ms: +(us / 1000).toFixed(1), pctBusy: +(100 * us / busy).toFixed(1), usPerIdle: cycles ? +(us / cycles).toFixed(2) : null }));
    const jsUs = [...agg.entries()].filter(([k]) => !k.startsWith('(idle)') && k !== '[wasm]' && !k.startsWith('(program)') && !k.startsWith('(garbage')).reduce((x, [, us]) => x + us, 0);
    return { totalMs: +(total / 1000).toFixed(0), idleMs: +(idle / 1000).toFixed(0), busyMs: +(busy / 1000).toFixed(0), busyUsPerIdle: cycles ? +(busy / cycles).toFixed(1) : null, wasmUsPerIdle: cycles ? +((agg.get('[wasm]') ?? 0) / cycles).toFixed(1) : null, jsOnlyUsPerIdle: cycles ? +(jsUs / cycles).toFixed(2) : null, programUsPerIdle: cycles ? +((agg.get('(program) :-1') ?? 0) / cycles).toFixed(2) : null, gcUsPerIdle: cycles ? +((agg.get('(garbage collector) :-1') ?? 0) / cycles).toFixed(2) : null, top };
  };
  const profileBoth = async (label, seconds) => {
    const sessionId = await getWorkerSession();
    if (!sessionId) { result['profile-' + label] = { error: 'no worker target' }; return; }
    const W = (m, p) => browser.cdp.send(m, p, sessionId);
    await W('Profiler.enable'); await W('Profiler.setSamplingInterval', { interval: 200 });
    await page.S('Profiler.enable'); await page.S('Profiler.setSamplingInterval', { interval: 200 });
    const sBefore = await sample();
    await W('Profiler.start'); await page.S('Profiler.start');
    await sleep(seconds * 1000);
    const wp = (await W('Profiler.stop')).profile; const pp = (await page.S('Profiler.stop')).profile;
    const sAfter = await sample();
    await W('Profiler.disable'); await page.S('Profiler.disable');
    const cycles = (sAfter.stats?.idleCount ?? 0) - (sBefore.stats?.idleCount ?? 0);
    const wallS = (sAfter.t - sBefore.t) / 1000;
    const out = { seconds, idleCycles: cycles, idlePerS: +(cycles / wallS).toFixed(1), updatesPerS: +((sAfter.updates - sBefore.updates) / wallS).toFixed(2), loadStart: +sBefore.load.toFixed(1), worker: summarize(wp, cycles), page: summarize(pp, cycles) };
    result['profile-' + label] = out;
    say(`PROFILE ${label}: ${cycles} idle cycles, ${out.idlePerS}/s; Worker busy ${out.worker.busyUsPerIdle} us/idle (wasm ${out.worker.wasmUsPerIdle}, js-only ${out.worker.jsOnlyUsPerIdle}, program ${out.worker.programUsPerIdle}, gc ${out.worker.gcUsPerIdle}); page main busy ${out.page.busyUsPerIdle} us/idle (${out.page.busyMs} ms of ${out.page.totalMs})`);
    for (const t of out.worker.top.slice(0, 14)) say(`  W ${t.ms} ms  ${t.pctBusy}%  ${t.usPerIdle} us/idle  ${t.fn}`);
    for (const t of out.page.top.slice(0, 14)) say(`  P ${t.ms} ms  ${t.pctBusy}%  ${t.usPerIdle} us/idle  ${t.fn}`);
  };
  if (secs.raf > 0) {
    // the page's own frame callbacks (requestAnimationFrame) over a stretch of idle and of play: how often the browser lets the page draw, and how even the intervals are. Installed only here (a callback every frame makes the page produce frames it would not).
    const rafStretch = async (label, seconds) => {
      await page.ev(`(() => { const a = []; window.__raf = a; const f = (t) => { a.push(t); if (window.__raf === a) requestAnimationFrame(f); }; requestAnimationFrame(f); })()`);
      const s0 = await sample();
      await sleep(seconds * 1000);
      const s1 = await sample();
      const ts = JSON.parse(await page.ev('JSON.stringify(window.__raf)'));
      await page.ev('window.__raf = null');
      const iv = []; for (let i = 1; i < ts.length; i++) iv.push(ts[i] - ts[i - 1]);
      iv.sort((x, y) => x - y);
      const q = (p) => (iv.length ? iv[Math.min(iv.length - 1, Math.max(0, Math.ceil(p / 100 * iv.length) - 1))] : null);
      const wall = (s1.t - s0.t) / 1000;
      const r = { seconds: +wall.toFixed(1), frames: ts.length, rafPerS: +(ts.length / wall).toFixed(1), intervalP50: q(50), intervalP95: q(95), intervalP99: q(99), intervalMax: iv.length ? iv[iv.length - 1] : null, over20ms: iv.filter((x) => x > 20).length, updatesPerS: +((s1.updates - s0.updates) / wall).toFixed(2), loadStart: +s0.load.toFixed(1) };
      result['raf-' + label] = r;
      say(`RAF ${label}: ${JSON.stringify(r)}`);
    };
    await rafStretch('idle', secs.raf);
    result.rafPlaySeen = await setPlaying(true);
    await rafStretch('play', secs.raf);
    result.rafPlayStopped = await setPlaying(false);
  }
  if (secs.profile > 0) {
    await profileBoth('idle', secs.profile);
    if (opt('profile-play', '0') === '1') {
      result.profilePlaySeen = await setPlaying(true);
      await profileBoth('play', secs.profile);
      result.profilePlayStopped = await setPlaying(false);
    }
  }
  if (secs.soak > 0) {
    // a long stretch of the idle main window: every 30 s the rates over the last 30 s and the memory (resident set of the whole Chrome tree, the page's JS heap, the Worker's JS heap through CDP Runtime.getHeapUsage)
    result.soak = [];
    // resident set (MB) of the whole Chrome tree, and by process type (the engine Worker and its wasm memory live in a renderer process)
    const rssBy = () => { const by = {}; let kb = 0; for (const pid of descendants(browser.pid)) { try { const st = fs.readFileSync(`/proc/${pid}/status`, 'utf8'); const k = Number(/VmRSS:\s+(\d+)/.exec(st)?.[1] ?? 0); const t = /--type=([a-z-]+)/.exec(fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').replace(/\0/g, ' '))?.[1] ?? 'browser'; by[t] = (by[t] ?? 0) + k; kb += k; } catch { /* gone */ } } return { rssMB: Math.round(kb / 1024), byType: Object.fromEntries(Object.entries(by).map(([t, k]) => [t, Math.round(k / 1024)])) }; };
    // the biggest resident mappings of the renderer processes (/proc/<pid>/smaps): which memory grows - the wasm linear memory (one big anonymous mapping), the code V8 compiles (executable mappings), or something else
    const mappings = () => {
      const out = [];
      let execKb = 0;
      for (const pid of descendants(browser.pid)) {
        let cmd = '';
        try { cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8'); } catch { continue; }
        if (!cmd.includes('--type=renderer')) continue;
        let txt = '';
        try { txt = fs.readFileSync(`/proc/${pid}/smaps`, 'utf8'); } catch { continue; }
        let cur = null;
        for (const line of txt.split('\n')) {
          const h = /^([0-9a-f]+)-([0-9a-f]+) (\S{4}) \S+ \S+ \S+\s*(.*)$/.exec(line);
          if (h) { cur = { pid, perms: h[3], name: h[4] || '[anon]', sizeKb: (parseInt(h[2], 16) - parseInt(h[1], 16)) / 1024, rssKb: 0 }; out.push(cur); continue; }
          const r = /^Rss:\s+(\d+) kB/.exec(line);
          if (r && cur) { cur.rssKb = Number(r[1]); if (cur.perms[2] === 'x') execKb += cur.rssKb; }
        }
      }
      out.sort((a, b) => b.rssKb - a.rssKb);
      return { execRssMB: Math.round(execKb / 1024), top: out.slice(0, 4).map((m) => `${m.perms} ${m.name === '[anon]' ? 'anon' : m.name.split('/').pop()} size ${Math.round(m.sizeKb / 1024)} MB rss ${Math.round(m.rssKb / 1024)} MB`) };
    };
    const heaps = async () => {
      const page_ = Number(await page.ev('performance.memory ? performance.memory.usedJSHeapSize : 0'));
      let worker = null;
      try {
        const sid = await getWorkerSession();
        if (sid) worker = (await browser.cdp.send('Runtime.getHeapUsage', {}, sid)).usedSize;
      } catch { /* not available */ }
      const wasm = Number(await page.ev('window.__app.engineMemoryBytes ?? 0'));
      return { wasmMB: wasm ? +(wasm / 1048576).toFixed(1) : null, pageJsHeapMB: +(page_ / 1048576).toFixed(1), workerJsHeapMB: worker === null ? null : +(worker / 1048576).toFixed(1), ...rssBy(), maps: mappings() };
    };
    if (opt('soak-play', '0') === '1') result.soakPlaySeen = await setPlaying(true);   // the soak runs while the project plays (the window changes ~60 times a second), so a falling update rate would show
    let prev = await sample();
    const t0 = Date.now();
    result.soak.push({ atS: 0, ...(await heaps()) });
    while ((Date.now() - t0) / 1000 < secs.soak) {
      await sleep(30000);
      const cur = await sample();
      const r = report(`soak-${result.soak.length}`, prev, cur);
      result.soak.push({ atS: Math.round((Date.now() - t0) / 1000), virtualPerWall: r.virtualPerWall, idlePerS: r.idlePerS, updatesPerS: r.updatesPerS, workerCore: r.workerCore, mainCore: r.mainCore, lagCuts: r.lagCuts, ...(await heaps()) });
      prev = cur;
    }
    if (opt('soak-play', '0') === '1') result.soakPlayStopped = await setPlaying(false);
  }
  if (secs.hide > 0) {
    // the page goes to the background (another page of the same browser context is brought to the front): does the Worker's clock keep pace? The page's own timers are throttled by the browser, the Worker's are not (hidden-timers.mjs)
    await page.activateOther();
    await sleep(500);
    result.visibilityWhileHidden = await page.ev('document.visibilityState');
    await phase('hidden', secs.hide);
    await page.activate();
    await sleep(500);
    result.visibilityAfter = await page.ev('document.visibilityState');
    await phase('after-hidden', Math.min(secs.hide, 15));
  }
  if (secs.freeze > 0) {
    // the page is frozen (CDP Page.setWebLifecycleState): its Worker does not run; the wall clock goes on. After it is active again the lag is what the driver has to deal with
    const sF0 = await sample();
    await page.S('Page.setWebLifecycleState', { state: 'frozen' });
    const tf = Date.now();
    await sleep(secs.freeze * 1000);
    await page.S('Page.setWebLifecycleState', { state: 'active' });
    result.frozenForS = (Date.now() - tf) / 1000;
    await phase('after-freeze', secs.after);
    const sF1 = await sample();
    report('freeze-whole', sF0, sF1);
  }
  clearInterval(memTimer);
  await sleep(3000);
  result.finalState = await page.ev('window.__app.state');
  // (result.checks, the per-window canvas checks of the former display paths, is gone with them: decisions 0223 / desk2)
  result.driverStats = await page.ev('JSON.stringify(window.__app.driverStats)').then(JSON.parse);
  result.idleTrace = await page.ev('JSON.stringify(window.__app.idleTrace ?? {})').then(JSON.parse);
  result.hzInfo = await page.ev('JSON.stringify(window.__app.hz ?? null)').then(JSON.parse);
  result.pageLog = await page.ev("document.getElementById('log').textContent").then((t) => t.split('\n').filter((l) => /hz|refresh rate|FLE_/i.test(l)).slice(0, 12));
  result.errors = page.consoleLog.filter((l) => l.startsWith('error') || l.startsWith('EXCEPTION')).slice(0, 10);
  result.logs = JSON.parse(await page.ev('JSON.stringify(window.__app.log ? window.__app.log.slice(-0) : null)').catch(() => 'null'));
} catch (e) {
  result.error = String(e.stack ?? e);
  say('FAILED: ' + result.error);
}
fs.writeFileSync(path.join(out, `result-${Date.now()}.json`), JSON.stringify(result, null, 1));
const net_ = await browser.close().catch((e) => ({ error: String(e) }));
result.net = net_;
await server.stop();
say('done');
