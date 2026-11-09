// Decision 0175: G7 retry. Opens the page against `vite preview` (the REAL engine of the bundle whose manifest.json is in FL_ASSETS_DIR), waits for the first idle, dismisses the welcome wizard with Enter
// and records, for `--quiet-s` seconds with the transport stopped and `--play-s` seconds with the transport playing (Space), what the AudioWorklet received:
//   the worklet's own counters every second (quanta, underrun quanta and frames, episodes, longest run, non-silent quanta, peak, RMS), the 50 ms samples of the worklet, the engine's own pulses (audio-pulse: virtual clock, frames
//   consumed, per-thread slices), the Worker's reports (Unlock times), and every idle of the engine (the virtual clock and the time since the last idle) from a collector that wraps window.__app.idles.push.
// usage: FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/audio-g7.mjs [--out DIR] [--quiet-s 60] [--play-s 60] [--port 4220] [--no-enter] [--chrome-args '...']
// FL_ASSETS_DIR is only READ. Nothing here asserts a result: it prints and stores what was measured.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';
import { openBrowser, sleep } from './browser.mjs';
import { descendants } from './cdp.mjs';
import { keyDown, keyUp, moveTo, pressAt, releaseAt } from './input-helpers.mjs';

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> node test/e2e/audio-g7.mjs [--out DIR] [--quiet-s 60] [--play-s 60]'); process.exit(2); }
const out = path.resolve(opt('out', path.join(os.tmpdir(), 'opencode', 'aud2-run')));
const quietS = Number(opt('quiet-s', '60')), playS = Number(opt('play-s', '60')), settleMaxS = Number(opt('settle-max-s', '150'));
const basePort = Number(opt('port', '4220'));
const noEnter = argv.includes('--no-enter');
const postEnterS = Number(opt('post-enter-s', '30'));
const keepIdles = opt('idles', '1') === '1';
// --click 'x,y;x,y' : clicks (virtual screen coordinates) made after the quiet phase and before the transport starts (for example the metronome button); --shot : a screenshot of the page after them
const clicks = opt('click', '').split(';').filter(Boolean).map((p) => p.split(',').map(Number));
const shot = argv.includes('--shot');
// --capture N : the worklet copies the first N frames from the first non-zero output sample after the transport starts; their SHA-256 and level are stored (the engine's own sound, nothing replayed)
const captureFrames = Number(opt('capture', '0'));
const hideS = Number(opt('hide-s', '0'));
// --chrome-nice N (PM-approved for the real-time runs, decision 0175): Chrome alone runs at nice N; the driver and vite preview are set to nice 15. Start the driver WITHOUT a nice prefix for this (a process cannot lower its own nice value).
const chromeNice = Number(opt('chrome-nice', '0'));
// cdp.mjs reads CHROME_BIN when it is imported, which is before this line: the wrapper must be named in the environment of the shell that starts the driver:
//   CHROME_BIN=/tmp/opencode/aud2/chrome-nice.sh CHROME_NICE=5 node test/e2e/audio-g7.mjs --chrome-nice 5 ...   (the wrapper is `exec nice -n $CHROME_NICE /usr/bin/google-chrome "$@"`)
// The driver checks the nice value of the browser it got and stops when it is not the one asked for.
const dist = opt('dist', '');   // another build directory for vite preview (--outDir)
const waitLoad = Number(opt('wait-load', '0')), waitLoadMaxS = Number(opt('wait-load-max-s', '900'));
fs.mkdirSync(out, { recursive: true });
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);
const portFree = (p) => new Promise((res) => { const s = net.createServer(); s.once('error', () => res(false)); s.listen(p, '127.0.0.1', () => s.close(() => res(true))); });
let port = basePort;
while (!(await portFree(port))) port++;

if (waitLoad > 0) {   // start only when the machine is quiet enough (the 1-minute load average is at most waitLoad), or after waitLoadMaxS
  const t0w = Date.now();
  while (os.loadavg()[0] > waitLoad && (Date.now() - t0w) / 1000 < waitLoadMaxS) await sleep(5000);
  say(`start: load ${os.loadavg()[0].toFixed(1)} after waiting ${((Date.now() - t0w) / 1000).toFixed(0)} s (wanted <= ${waitLoad})`);
}
const server = start(path.join(APP, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort', ...(dist ? ['--outDir', dist] : [])], { cwd: APP, env: { ...process.env, FL_PERF_LOG: path.join(out, 'perf-log.jsonl') }, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
if (chromeNice > 0) for (const pid of descendants(server.child.pid)) { try { os.setPriority(pid, 15); } catch { /* gone */ } }
say(`vite preview on ${port}`);

const HZ = 100;
function threads(root) {
  const m = new Map();
  const pids = descendants(root);
  for (const pid of pids) {
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      try {
        const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, 'utf8');
        const comm = st.slice(st.indexOf('(') + 1, st.lastIndexOf(')'));
        const f = st.slice(st.lastIndexOf(')') + 2).split(' ');
        m.set(`${pid}/${tid}`, { comm, ticks: Number(f[11]) + Number(f[12]) });
      } catch { /* gone */ }
    }
  }
  return m;
}
const sumCpu = (a, b, re) => { let s = 0; for (const [k, v] of b) { if (!re.test(v.comm)) continue; const o = a.get(k); s += (v.ticks - (o ? o.ticks : 0)) / HZ; } return s; };

// run-queue statistics (/proc/<pid>/task/<tid>/schedstat: ns on the CPU, ns waiting on a run queue) of the threads that matter, every 200 ms: how much of the engine's wall time was spent waiting for a CPU (the machine's load)
const sched = { names: {}, t: [], rows: [] };
let schedKeys = [], schedAt = 0;
function schedPick(root) {
  const keys = [];
  for (const pid of descendants(root)) {
    let cmd = '';
    try { cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8'); } catch { continue; }
    if (!cmd.includes('--type=renderer') && !cmd.includes('--type=gpu') ) continue;
    let tids = [];
    try { tids = fs.readdirSync(`/proc/${pid}/task`); } catch { continue; }
    for (const tid of tids) {
      let comm = '';
      try { comm = fs.readFileSync(`/proc/${pid}/task/${tid}/comm`, 'utf8').trim(); } catch { continue; }
      if (/DedicatedWorker|Audio|CrRendererMain|Compositor|VizCompositor|ThreadPool/i.test(comm) || tid === String(pid)) { keys.push({ k: `${pid}/${tid}`, comm }); sched.names[`${pid}/${tid}`] = comm; }
    }
  }
  return keys;
}
function schedSample(root) {
  if (Date.now() - schedAt > 5000) { schedKeys = schedPick(root); schedAt = Date.now(); }
  const row = {};
  for (const { k } of schedKeys) {
    try { const f = fs.readFileSync(`/proc/${k.split('/')[0]}/task/${k.split('/')[1]}/schedstat`, 'utf8').trim().split(' '); row[k] = [Number(f[0]), Number(f[1])]; } catch { /* gone */ }
  }
  sched.t.push(Date.now()); sched.rows.push(row);
}
const browser = await openBrowser(out, 'aud2');
if (chromeNice > 0) { try { os.setPriority(0, 15); } catch { /* ignore */ } }
const schedTimer = setInterval(() => { try { schedSample(browser.pid); } catch { /* ignore */ } }, 200);
// the engine Worker's thread alone every 5 ms: cumulative [on-CPU ns, run-queue wait ns] against Date.now(), to cut the CPU time out of every stretch of the engine's computing (load independent)
const fast = { key: null, t: [], run: [], wait: [] };
const fastTimer = setInterval(() => {
  try {
    if (!fast.key) { const k = schedKeys.find((x) => /DedicatedWorker/.test(x.comm)); if (!k) return; fast.key = k.k; }
    const [pid, tid] = fast.key.split('/');
    const f = fs.readFileSync(`/proc/${pid}/task/${tid}/schedstat`, 'utf8').trim().split(' ');
    fast.t.push(Date.now() + (performance.now() % 1)); fast.run.push(Number(f[0])); fast.wait.push(Number(f[1]));
  } catch { /* gone */ }
}, 5);
// every frame (a dirty rectangle of a window) the engine Worker posts to the page, with the page's clock: which windows the engine redrew at the end of a long stretch
const FRAMES_INIT = `(() => { window.__frames = []; const O = window.Worker; window.Worker = class extends O { constructor(u, o) { super(u, o); this.addEventListener('message', (e) => { const m = e.data; if (m && m.t === 'frame' && window.__frames.length < 400000) window.__frames.push([performance.timeOrigin + performance.now(), m.lid, m.rect.x0, m.rect.y0, m.rect.x1, m.rect.y1, m.postedAt || 0]); }); } }; })();`;
const page = await browser.newPage({ width: 2400, height: 1300, initScript: FRAMES_INIT });
if (chromeNice > 0) { const got = os.getPriority(browser.pid); if (got !== chromeNice) { console.error(`chrome-nice ${chromeNice} was asked for but the browser runs at nice ${got}: stopping (start the driver with CHROME_BIN=/tmp/opencode/aud2/chrome-nice.sh CHROME_NICE=${chromeNice})`); await browser.close().catch(() => {}); await server.stop(); process.exit(3); } }
const result = { chromeNice: (() => { try { return os.getPriority(browser.pid); } catch { return null; } })(), loadAtStart: os.loadavg(), port, chrome: browser.version, startedAt: new Date().toISOString(), loads: [], series: [], phases: {} };
let exitCode = 0;
let bootTimer;
try {
  const t0 = Date.now();
  const url = `http://127.0.0.1:${port}/`;
  await page.goto(url);
  // the idle collector: every idle the page is told about (at = Worker abs ms, n, virtual ticks, busyMs) in typed arrays; installed once the page has made window.__app
  await page.waitFor('window.__app && window.__app.idles', 60000);
  await page.ev(`(() => { const a = window.__app.idles; const cap = 600000; const at = new Float64Array(cap), now = new Float64Array(cap), busy = new Float32Array(cap), nn = new Int32Array(cap); let k = 0;
    const orig = a.push.bind(a); a.push = (x) => { if (k < cap) { at[k] = x.at; now[k] = x.now; busy[k] = x.busyMs === null ? -1 : x.busyMs; nn[k] = x.n; k++; } return orig(x); };
    window.__aud2 = { get k() { return k; }, at, now, busy, nn, rows(from, to) { return JSON.stringify({ at: Array.from(at.subarray(from, to)), now: Array.from(now.subarray(from, to)), busy: Array.from(busy.subarray(from, to)), n: Array.from(nn.subarray(from, to)) }); } }; })()`);
  let idleAt = null;
  while ((Date.now() - t0) / 1000 < 400) {
    await sleep(1000);
    const j = JSON.parse(await page.ev('JSON.stringify({ state: window.__app?.state, kind: window.__app?.engineKind, tl: window.__app?.timeline, err: window.__app?.error, au: window.__app?.audio ? { state: window.__app.audio.state, error: window.__app.audio.error, ctx: window.__app.audio.ctx?.state, sr: window.__app.audio.ctx?.sampleRate } : null })').catch(() => '{}'));
    if (j.state === 'error' || j.state === 'exited') { say(`page ended early: ${JSON.stringify(j).slice(0, 600)}`); break; }
    if (j.tl && typeof j.tl.idle === 'number') { idleAt = j.tl.idle; result.engineKind = j.kind; result.audioAtIdle = j.au; break; }
  }
  result.firstIdleS = idleAt === null ? null : idleAt / 1000;
  say(`first idle after ${result.firstIdleS} s; audio ${JSON.stringify(result.audioAtIdle)}`);
  if (idleAt === null) throw new Error('no first idle');

  const counters = async () => JSON.parse(await page.ev(`JSON.stringify((() => { const a = window.__app.audio; const u = a.underruns(); return { t: performance.now(), abs: performance.timeOrigin + performance.now(), ctxTime: a.ctx.currentTime, ctxState: a.ctx.state, c: u.c, peak: u.level.peak, rms: u.level.rms, fill: a.fill(), pulses: a.pulses.length, samples: a.samples.length, unlocks: a.workerUnlocks.length, idleK: window.__aud2.k, mem: window.__app.engineMemoryBytes, upd: window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total }; })())`));
  const showRow = (tag, r, prev) => {
    const c = r.c;
    say(`${tag} t=${(r.t / 1000).toFixed(1)}s ctx=${r.ctxTime.toFixed(2)} state=${c.state} q=${c.quanta} play=${c.playing} ur=${c.urQuanta} (steady ${c.urSteady}, startup ${c.urStartup}) runs=${c.urRuns} maxrun=${c.urMaxRun} nonSilent=${c.nonSilent} fill=${r.fill} peak=${r.peak.toFixed(4)} rms=${r.rms.toFixed(5)} idles=${r.idleK} upd=${r.upd}`);
  };
  const tick = async (tag, seconds) => {
    const end = Date.now() + seconds * 1000;
    let last = await counters();
    const cpu0 = threads(browser.pid);
    result.loads.push({ tag, load: os.loadavg()[0] });
    while (Date.now() < end) {
      await sleep(Math.min(1000, Math.max(50, end - Date.now())));
      const r = await counters();
      r.load = os.loadavg()[0];
      r.tag = tag;
      result.series.push(r);
      if (result.series.length % 5 === 0) showRow(tag, r, last);
      last = r;
    }
    const cpu1 = threads(browser.pid);
    result.phases[tag] = { seconds, workerCpuS: sumCpu(cpu0, cpu1, /Worker|DedicatedWorker/i), allCpuS: sumCpu(cpu0, cpu1, /./), last };
    return last;
  };

  // from the first idle until the quiet phase begins: the counters every second (the start-up and the stretch after Enter, where the engine computes for long)
  result.bootSeries = [];
  let bootBusy = false;
  bootTimer = setInterval(async () => { if (bootBusy) return; bootBusy = true; try { const r = await counters(); r.load = os.loadavg()[0]; result.bootSeries.push(r); } catch { /* page busy */ } bootBusy = false; }, 1000);
  await sleep(3000);
  if (!noEnter) {
    // decisions 0223 / desk2: the page holds no window list; the title of the engine's foreground window (the page's document.title, no window list needed) says whether the welcome wizard is still up
    const wizardUp = async () => page.ev("/^Welcome to FL Studio/.test(window.__app.pageTitle().title)");
    let tries = 0;
    for (; tries < 4; tries++) {
      await keyDown(page, 'Enter'); await sleep(40); await keyUp(page, 'Enter');
      let gone = false;
      for (let k = 0; k < 24 && !gone; k++) { await sleep(500); gone = !(await wizardUp()); }
      if (gone) break;
    }
    result.enterTries = tries + 1;
    say(`Enter sent ${result.enterTries} time(s)`);
    // wait until the engine's long stretch after Enter is over: the idle counter moves again by > 20 per second for 6 seconds in a row
    let lastN = -1, still = 0;
    const tw = Date.now();
    while (Date.now() - tw < settleMaxS * 1000 && still < 6) {
      await sleep(1000);
      const n = Number(await page.ev('window.__app.driverStats?.idleCount ?? 0'));
      if (lastN >= 0 && n - lastN > 20) still++; else still = 0;
      lastN = n;
    }
    result.settleS = (Date.now() - tw) / 1000;
    say(`settled after ${result.settleS.toFixed(1)} s`);
  }
  if (!noEnter && postEnterS > 0) { say(`waiting ${postEnterS} s after the settle (the engine's own animation after Enter)`); await sleep(postEnterS * 1000); }
  clearInterval(bootTimer);
  const resetFill = () => page.ev('window.__app.audio.resetFill()');
  await resetFill();
  const before = await counters();
  result.beforeQuiet = before;
  showRow('before', before);
  if (quietS > 0) await tick('quiet', quietS);

  const updTotal = async () => Number(await page.ev('window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total'));
  const updRate = async () => { const u0 = await updTotal(), t = Date.now(); await sleep(2000); return ((await updTotal()) - u0) / ((Date.now() - t) / 1000); };
  async function setPlaying(want) {
    for (let attempt = 0; attempt < 3; attempt++) {
      const playing = (await updRate()) >= 3;
      if (playing === want) return true;
      await moveTo(page, 960, 700); await sleep(300);
      (result.spaceAt ??= []).push({ want, page: Number(await page.ev('performance.now()')) });
      await keyDown(page, 'Space'); await sleep(40); await keyUp(page, 'Space');
      for (let k = 0; k < 6; k++) { await sleep(500); if (((await updRate()) >= 3) === want) return true; }
    }
    return (await updRate() >= 3) === want;
  }
  if (hideS > 0) {   // the tab goes to the background (another tab in front): does the audio keep its pace, are timers throttled?
    await page.activateOther();
    result.hiddenVisibility = await page.ev('document.visibilityState').catch(() => null);
    await tick('hidden', hideS);
    await page.activate(); await sleep(500);
    result.afterHidden = await counters();
    say(`hidden for ${hideS} s; visibility during: ${result.hiddenVisibility}`);
  }
  for (const [cx, cy] of clicks) {
    await moveTo(page, cx, cy); await sleep(200);
    await pressAt(page, cx, cy); await sleep(80); await releaseAt(page, cx, cy);
    say(`clicked (${cx},${cy})`);
    await sleep(1500);
  }
  if (shot) { fs.writeFileSync(path.join(out, 'after-clicks.png'), await page.shot(0, 0, 2400, 1300)); say('screenshot written'); }
  if (clicks.length) result.afterClicks = await counters();
  if (playS > 0) {
    if (captureFrames > 0) await page.ev(`window.__app.audio.armCapture(${captureFrames})`);
    result.playSeen = await setPlaying(true);
    say(`transport playing: ${result.playSeen}`);
    await resetFill();
    result.playStartCounters = await counters();
    await tick('play', playS);
    result.playStopped = await setPlaying(false);
    say(`transport stopped: ${result.playStopped}`);
  }
  if (captureFrames > 0) {
    result.captures = JSON.parse(await page.ev(`(async () => { const out = []; for (const c of window.__app.audio.captures) { const b = new Uint8Array(c.frames.buffer, c.frames.byteOffset, c.frames.byteLength); const h = await crypto.subtle.digest('SHA-256', b); let pk = 0, ss = 0, nz = 0; for (let i = 0; i < c.frames.length; i++) { const v = Math.abs(c.frames[i]); if (v > pk) pk = v; ss += v * v; if (v !== 0) nz++; } const prefixes = {}; for (const n of [1024, 2048, 4096, 8192, 16384, 32768, 65536]) { if (n <= c.validFrames) { const hp = await crypto.subtle.digest('SHA-256', new Uint8Array(c.frames.buffer, c.frames.byteOffset, n * 8)); prefixes[n] = [...new Uint8Array(hp)].map((x) => x.toString(16).padStart(2, '0')).join(''); } } out.push({ frames: c.frames.length / 2, valid: c.valid, validFrames: c.validFrames, prefixes, firstFrame: c.firstFrame, sha256: [...new Uint8Array(h)].map((x) => x.toString(16).padStart(2, '0')).join(''), peak: pk, rms: Math.sqrt(ss / c.frames.length), nonZero: nz, head: Array.from(c.frames.slice(0, 16)) }); } return JSON.stringify(out); })()`));
    say(`captures: ${JSON.stringify(result.captures.map((c) => ({ ...c, head: undefined })))}`);
  }
  result.final = await counters();
  showRow('final', result.final);

  // the engine side
  result.audioDump = JSON.parse(await page.ev(`JSON.stringify((() => { const a = window.__app.audio; return { opts: a.opts, latency: a.latency, startedAt: a.startedAt, samples: a.samples, pulses: a.pulses, reports: a.reports.map((r) => ({ ...r, unlocks: undefined })), unlocks: a.workerUnlocks, ctxSampleRate: a.ctx.sampleRate, baseLatency: a.ctx.baseLatency, outputLatency: a.ctx.outputLatency }; })())`));
  if (keepIdles) {
    const k = Number(await page.ev('window.__aud2.k'));
    const rows = { at: [], now: [], busy: [], n: [] };
    for (let from = 0; from < k; from += 50000) { const r = JSON.parse(await page.ev(`window.__aud2.rows(${from}, ${Math.min(k, from + 50000)})`)); for (const key of Object.keys(rows)) rows[key].push(...r[key]); }
    result.idles = rows;
    result.timeOrigin = Number(await page.ev('performance.timeOrigin'));
  }
  result.frames = JSON.parse(await page.ev('JSON.stringify(window.__frames)'));
  // what the page itself shows: the beacon's audio field and the P3 row of the performance panel
  result.beaconAudio = JSON.parse(await page.ev('JSON.stringify(window.__app.perf.snapshot("periodic").audio)'));
  await page.ev("document.getElementById('perf-panel').open = true"); await sleep(400);
  result.panelText = await page.ev("document.getElementById('perf-body').innerText");
  result.errors = JSON.parse(await page.ev('JSON.stringify({ err: window.__app.error, state: window.__app.state, exit: window.__app.exit })'));
  result.log = (await page.ev("document.getElementById('log')?.textContent ?? ''")).split('\n').slice(-60);
  result.engineLogTail = (await page.ev('JSON.stringify(window.__app.engineLog().slice(-30))').then(JSON.parse).catch(() => []));
  result.console = page.consoleLog.slice(-40);
} catch (e) {
  say('FAILED: ' + String(e?.stack ?? e));
  result.failure = String(e?.stack ?? e);
  result.console = page.consoleLog.slice(-40);
  exitCode = 1;
} finally {
  if (bootTimer) clearInterval(bootTimer);
  result.loadAtEnd = os.loadavg();
  clearInterval(schedTimer); clearInterval(fastTimer);
  result.sched = sched; result.fast = fast;
  fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result));
  say(`wrote ${path.join(out, 'result.json')}`);
  await page.close().catch(() => {});
  await browser.close().catch(() => {});
  await server.stop();
}
process.exit(exitCode);
