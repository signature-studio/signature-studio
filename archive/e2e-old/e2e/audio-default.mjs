// Decision 0211 (aud3): does the default entrance make sound? Opens the page (default: NO parameter, the entrance the owner opens; --query adds any, for example audio=0 or audio=1), without the autoplay-policy
// flag (the owner's browser starts an AudioContext suspended until a gesture), waits for the engine's first idle, closes the welcome wizard, optionally clicks (virtual screen coordinates), starts the transport
// with Space (a CDP key event = a user gesture) and records what the AudioWorklet received: the context state over time, the worklet's counters (quanta, underruns, non-silent quanta, peak/RMS), a raw capture of the
// engine's own output from its first non-zero sample (--capture N frames; onsets and intervals are computed from it), the page's audio status line and the beacon's audio block. It asserts nothing: it prints and stores.
// usage: FL_ASSETS_DIR=<dir with manifest.json> node test/e2e/audio-default.mjs [--out DIR] [--query 'audio=1'] [--click 'x,y;x,y'] [--capture 441000] [--play-s 20] [--settle-s 20] [--no-enter] [--autoplay-flag] [--shot]
// FL_ASSETS_DIR is only READ; FL_PERF_LOG (default: out/perf-log.jsonl) receives the beacons of the run.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { freePort, start } from '../helpers/proc.mjs';
import { APP } from '../helpers/wrangler.mjs';
import { openBrowser, sleep } from './browser.mjs';
import { descendants } from './cdp.mjs';
import { keyDown, keyUp, moveTo, pressAt, releaseAt } from './input-helpers.mjs';

// a key chord such as 'ctrl+m' through CDP Input.dispatchKeyEvent (modifiers: alt 1, ctrl 2, meta 4, shift 8); only letters, digits and the named keys the tests use
async function chord(page, spec) {
  const parts = spec.toLowerCase().split('+');
  const k = parts.pop();
  const mod = parts.reduce((m, p) => m | ({ alt: 1, ctrl: 2, meta: 4, shift: 8 }[p] ?? 0), 0);
  const vk = k.length === 1 ? k.toUpperCase().charCodeAt(0) : ({ space: 32, enter: 13, escape: 27, f5: 116 }[k] ?? 0);
  const key = k === 'space' ? ' ' : k;
  const code = k.length === 1 && /[a-z]/.test(k) ? 'Key' + k.toUpperCase() : k === 'space' ? 'Space' : k;
  if (mod & 2) await page.S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17, modifiers: 2 });
  await page.S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: mod });
  await sleep(40);
  await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: mod });
  if (mod & 2) await page.S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17, modifiers: 0 });
}

const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf('--' + n); return i < 0 ? d : argv[i + 1]; };
if (!process.env.FL_ASSETS_DIR) { console.error('usage: FL_ASSETS_DIR=<dir> node test/e2e/audio-default.mjs [--out DIR] [--query q]'); process.exit(2); }
const out = path.resolve(opt('out', path.join(os.tmpdir(), 'opencode', 'aud3-run')));
fs.mkdirSync(out, { recursive: true });
const query = opt('query', '');
const playS = Number(opt('play-s', '20')), settleS = Number(opt('settle-s', '20')), captureFrames = Number(opt('capture', '0'));
const clicks = opt('click', '').split(';').filter(Boolean).map((p) => p.split(',').map(Number));
const keysBefore = opt('keys', '').split(',').filter(Boolean);
const noEnter = argv.includes('--no-enter'), shot = argv.includes('--shot'), noPlay = argv.includes('--no-play');
const distDir = opt('dist', '');
const say = (s) => console.log(`${new Date().toISOString()} ${s}`);
const port = await freePort();
const server = start(path.join(APP, 'node_modules/.bin/vite'), ['preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort', ...(distDir ? ['--outDir', distDir] : [])], { cwd: APP, env: { ...process.env, FL_PERF_LOG: process.env.FL_PERF_LOG ?? path.join(out, 'perf-log.jsonl') }, ready: /Local:\s+http/, label: 'vite preview' });
await server.ready;
const browser = await openBrowser(out, 'aud3');
// --hold-resume: AudioContext.resume() never settles (a browser that keeps the context suspended even after a gesture): what does the engine do when the worklet never runs?
const holdResume = argv.includes('--hold-resume');
const page = await browser.newPage({ width: 2400, height: 1300, initScript: holdResume ? 'AudioContext.prototype.resume = function () { return new Promise(() => {}); };' : '' });
// --throttle K: CDP Emulation.setCPUThrottlingRate (K times slower); --pulses: keep the engine's audio-pulse messages (virtual clock, frames consumed and written every 500 ms)
const throttle = Number(opt('throttle', '1'));
const result = { url: `http://127.0.0.1:${port}/${query ? '?' + query : ''}`, query, throttle, holdResume, chrome: browser.version, loadAtStart: os.loadavg(), startedAt: new Date().toISOString(), series: [], steps: [] };
const step = (name, extra = {}) => { const r = { name, t: Date.now() - t0, ...extra }; result.steps.push(r); say(`${name} ${JSON.stringify(extra).slice(0, 300)}`); };
const t0 = Date.now();
let exitCode = 0;
// CPU seconds per thread class (utime+stime of /proc/<pid>/task/<tid>/stat) of the browser's process tree: the engine Worker, the renderer main thread, the audio threads, the rest
const HZ = 100;
function threadTicks() {
  const m = new Map();
  for (const pid of descendants(browser.pid)) {
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
const classOf = (c) => (/DedicatedWorker/i.test(c) ? 'engineWorker' : /Audio|WebAudio/i.test(c) ? 'audio' : /CrRendererMain/i.test(c) ? 'rendererMain' : 'other');
function cpuBetween(a, b) {
  const o = { engineWorker: 0, audio: 0, rendererMain: 0, other: 0 };
  for (const [k, v] of b) o[classOf(v.comm)] += (v.ticks - (a.get(k)?.ticks ?? 0)) / HZ;
  return o;
}
result.cpu = {};
const AU = `(() => { const a = window.__app?.audio; if (!a) return null; const c = a.counters(); const u = a.underruns(); return { t: performance.now(), ctx: a.ctx ? { state: a.ctx.state, sampleRate: a.ctx.sampleRate, base: a.ctx.baseLatency, out: a.ctx.outputLatency, time: a.ctx.currentTime } : null, pa: a.state, node: !!a.node, err: a.error, status: document.getElementById('audio-status')?.textContent ?? null, c, peak: u.level.peak, rms: u.level.rms, fill: a.fill(), sinkFrames: a.sink.frames, idles: window.__app.driverStats?.idleCount ?? null, upd: window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total }; })()`;
const au = async () => JSON.parse(await page.ev(`JSON.stringify(${AU})`));
const row = (tag, r) => { if (!r) return `${tag} (no audio object)`; const c = r.c; return `${tag} ctx=${r.ctx?.state} t=${r.ctx?.time?.toFixed(1)} dev=${c.state} q=${c.quanta} play=${c.playing} ur=${c.urQuanta} nonSilent=${c.nonSilent} peak=${r.peak.toFixed(4)} rms=${r.rms.toFixed(5)} fill=${r.fill} sink=${r.sinkFrames ?? '-'} status='${r.status}'`; };
try {
  if (throttle > 1) { await page.S('Emulation.setCPUThrottlingRate', { rate: throttle }); step('cpu-throttle', { rate: throttle }); }
  await page.goto(result.url);
  await page.waitFor('window.__app', 60000);
  let idleAt = null;
  while ((Date.now() - t0) / 1000 < 400) {
    await sleep(500);
    const j = JSON.parse(await page.ev('JSON.stringify({ state: window.__app?.state, kind: window.__app?.engineKind, tl: window.__app?.timeline, err: window.__app?.error })').catch(() => '{}'));
    if (j.state === 'error' || j.state === 'exited') { step('page-ended-early', j); break; }
    if (j.tl && typeof j.tl.idle === 'number') { idleAt = j.tl.idle; result.engineKind = j.kind; break; }
  }
  result.firstIdleS = idleAt === null ? null : idleAt / 1000;
  step('first-idle', { s: result.firstIdleS, engineKind: result.engineKind });
  if (idleAt === null) throw new Error('no first idle');
  result.audioAtIdle = await au();
  step('audio-at-first-idle (before any gesture)', { row: row('idle', result.audioAtIdle) });
  const tick = async (tag, seconds) => {
    const end = Date.now() + seconds * 1000;
    const c0 = threadTicks(), w0 = Date.now(), l0 = os.loadavg()[0];
    while (Date.now() < end) {
      await sleep(Math.min(1000, Math.max(50, end - Date.now())));
      const r = await au();
      if (r) { r.tag = tag; r.load = os.loadavg()[0]; result.series.push(r); }
      if (result.series.length % 5 === 0) say(row(tag, r));
    }
    const wall = (Date.now() - w0) / 1000;
    result.cpu[tag] = { wallS: wall, cpuS: cpuBetween(c0, threadTicks()), loadStart: l0, loadEnd: os.loadavg()[0] };
    say(`cpu ${tag}: ${JSON.stringify(result.cpu[tag])}`);
  };
  await sleep(2000);
  result.audioBeforeGesture = await au();
  step('before-gesture', { row: row('pre', result.audioBeforeGesture) });
  if (argv.includes('--first-click')) {   // the first gesture is a mouse click on an empty part of the desktop (not a key)
    await moveTo(page, 1800, 1000); await sleep(200);
    await pressAt(page, 1800, 1000); await sleep(80); await releaseAt(page, 1800, 1000);
    await sleep(1500);
    result.audioAfterClick = await au();
    step('after-first-click (a mouse gesture, no key yet)', { row: row('click', result.audioAfterClick) });
  }
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
    step('enter-sent', { tries: result.enterTries });
    result.audioAfterGesture = await au();
    step('after-enter (a keydown gesture)', { row: row('post', result.audioAfterGesture) });
  }
  step('settle', { s: settleS });
  await tick('settle', settleS);
  if (shot) fs.writeFileSync(path.join(out, 'before-clicks.png'), await page.shot(0, 0, 2400, 1300));
  for (const [cx, cy] of clicks) {
    await moveTo(page, cx, cy); await sleep(200);
    await pressAt(page, cx, cy); await sleep(80); await releaseAt(page, cx, cy);
    step('clicked', { cx, cy });
    await sleep(1500);
  }
  for (const spec of keysBefore) { await moveTo(page, 960, 700); await sleep(200); await chord(page, spec); step('key', { spec }); await sleep(1500); }
  if (shot) fs.writeFileSync(path.join(out, 'after-clicks.png'), await page.shot(0, 0, 2400, 1300));
  const updTotal = async () => Number(await page.ev('window.__app.perf.recorder.snapshot(performance.timeOrigin + performance.now()).updates.total'));
  const updRate = async () => { const u0 = await updTotal(), t = Date.now(); await sleep(2000); return ((await updTotal()) - u0) / ((Date.now() - t) / 1000); };
  if (!noPlay) {
    result.updRateBeforePlay = await updRate();
    step('window-update-rate-before-play', { perSecond: result.updRateBeforePlay });
    if (captureFrames > 0) await page.ev(`window.__app.audio?.armCapture(${captureFrames}, ${argv.includes('--timeline')})`);
    result.beforePlay = await au();
    await moveTo(page, 960, 700); await sleep(300);
    await keyDown(page, 'Space'); await sleep(40); await keyUp(page, 'Space');
    step('space-sent');
    result.playStartPage = Number(await page.ev('performance.now()'));
    result.countersAtPlayStart = await au();
    result.updRateDuringPlay = await updRate();
    step('window-update-rate-during-play', { perSecond: result.updRateDuringPlay });
    await tick('play', playS);
    if (shot) fs.writeFileSync(path.join(out, 'during-play.png'), await page.shot(0, 0, 2400, 1300));
    result.playEndPage = Number(await page.ev('performance.now()'));
    result.countersAtPlayEnd = await au();
    await keyDown(page, 'Space'); await sleep(40); await keyUp(page, 'Space');
    step('space-stopped');
    await sleep(1500);
  }
  result.final = await au();
  step('final', { row: row('final', result.final) });
  if (captureFrames > 0) {
    const caps = await page.ev(`JSON.stringify((window.__app.audio?.captures ?? []).map((c) => ({ valid: c.valid, firstFrame: c.firstFrame, validFrames: c.validFrames, n: c.frames.length / 2 })))`).catch(() => '[]');
    result.captureInfo = JSON.parse(caps);
    const full = await page.ev(`(async () => { const c = (window.__app.audio?.captures ?? [])[0]; if (!c) return null; const b = new Uint8Array(c.frames.buffer, c.frames.byteOffset, c.frames.byteLength); let s = ''; for (let i = 0; i < b.length; i += 32768) s += String.fromCharCode(...b.subarray(i, i + 32768)); return btoa(s); })()`);
    if (full) fs.writeFileSync(path.join(out, 'capture.f32'), Buffer.from(full, 'base64'));
  }
  result.audioDump = JSON.parse(await page.ev(`JSON.stringify((() => { const a = window.__app.audio; return a ? { opts: a.opts, latency: a.latency, state: a.state, error: a.error, ctxState: a.ctx?.state, sampleRate: a.ctx?.sampleRate, baseLatency: a.ctx?.baseLatency, outputLatency: a.ctx?.outputLatency, detail: a.detail(), samples: a.samples.slice(-4000), pulses: a.pulses.map((p) => ({ page: p.page, real: p.real, virt: p.virt, consumed: p.consumed, written: p.written, mixer: p.mixer, uc: p.unlockCompute })), reports: a.reports.map((r) => ({ page: r.page, waits: r.waits, publishes: r.publishes, skipped: r.skipped, gapFrames: r.gapFrames, pace: r.pace })), history: a.history.filter((h, i) => i % 20 === 0) } : null; })())`));
  result.idleCount = Number(await page.ev('window.__app.driverStats?.idleCount ?? 0').catch(() => 0));
  result.beaconAudio = JSON.parse(await page.ev('JSON.stringify(window.__app.perf.snapshot("periodic").audio ?? null)').catch(() => 'null'));
  result.statusLine = await page.ev("document.getElementById('audio-status')?.innerText ?? null").catch(() => null);
  result.errors = JSON.parse(await page.ev('JSON.stringify({ err: window.__app.error, state: window.__app.state, exit: window.__app.exit })'));
  result.engineLogTail = await page.ev('JSON.stringify(window.__app.engineLog().slice(-30))').then(JSON.parse).catch(() => []);
  result.console = page.consoleLog.slice(-40);
} catch (e) {
  say('FAILED: ' + String(e?.stack ?? e));
  result.failure = String(e?.stack ?? e);
  result.console = page.consoleLog.slice(-40);
  exitCode = 1;
} finally {
  result.loadAtEnd = os.loadavg();
  fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(result));
  say(`wrote ${path.join(out, 'result.json')}`);
  await page.close().catch(() => {});
  await browser.close().catch(() => {});
  await server.stop();
}
process.exit(exitCode);
