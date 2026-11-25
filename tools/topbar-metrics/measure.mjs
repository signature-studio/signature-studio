// topbar-metrics (#1391): the top bar's Memory Usage, CPU Load and Frame Rate against what the engine really does.
// Usage: node tools/topbar-metrics/measure.mjs <absolute path to the bundle directory raw/<name>> [--dpr <value>] [--shots <directory>] [--dump <file>] > metrics.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does).
// --dump writes the whole engine log to <file> when the run fails (the trap's frames are there; the stdout JSON keeps only the first lines).
// The default project is opened (the welcome window is closed with its X; the "Problems loading the project" box, which the default project shows because its samples and plug-ins
// are not in this environment, is answered with Enter). Then, on the same page:
//   memory   from the start of the project: MEMORY_SAMPLES readings of the guest's memory (the "<n> MB" runs, guestText), then the label is found (findLabel); the K32 values are the engine log's
//            "memory-usage: K32GetProcessMemoryInfo WorkingSetSize <bytes>" lines (the engine writes one for each call of K32GetProcessMemoryInfo, #1391).
//   idle     no input for IDLE_MS, then a window of MEASURE_MS;
//   playing  the audio output is started by a gesture of the page's own, Space starts the transport and the metronome button of the guest's top bar is pressed (the default project makes sound);
//            then a window of MEASURE_MS;
//   fps      at DPR 1 and at a random DPR in 1.01..3.00 (the playing state): a window of MEASURE_MS at each.
// Per window: rafPerSec (the page's requestAnimationFrame callbacks per second; a loop of this tool, not the product's burst), presentsPerSec (window.__app.deskStats.presents per second),
// busyPct (the Worker's busy fraction, the page's own engineRunBeacon measure: the sum of the busyMs of the idles over the window divided by the wall time; the idles' times are absolute, as the window is).
// The top bar's Memory Usage is drawn by the guest with no window of its own (the desktop's window table has no text for it), so it is read from the guest's memory: the "<n> MB" runs (one space).
// memory (#1391, stage 1): MEMORY_SAMPLES samples, MEMORY_EVERY_MS apart, from the close of the welcome window on. Each sample: the K32 answers so far (the engine log), the "<n> MB" runs of the guest's memory
//   (guestText, every place, ASCII and UTF-16LE), and the hash of the top bar's band (a screenshot of its BAND_H rows; sha256 of the PNG, first 16 hex digits).
//   candidates  the integers of all those runs (the samples and a last reading after the K32 answers).
//   zeroMbNew   the "0 MB" runs seen in a sample taken after the first K32 answer (the engine's count) and not seen before it.
//   shownMb     the MiB floor of the last K32 answer when a candidate equals it; else the one candidate within +-1; else null (nothing is guessed).
//   hash        method "band" (no rectangle of the label is taken): series and floors are reported; the check of the change timing is not judged for "band".
// memory.guess (findLabel: the run that changed last) is kept for reference only; it is not the displayed value (pm-work/findings/topbar-metrics-1391.md).
// Not done in this version (#1402): the top bar's CPU Load is the top-left digits of the CPU panel (no unit); its Frame Rate is the bottom-right digits, which show the voice count until the CPU
// panel's right-click menu selects "View FPS" (then "60 FPS" and the like). Both are plain digits in the guest's memory and are not read here: cpu.idlePct, cpu.playingPct and fps.samples[].shown
// stay null (pm-work/findings/topbar-cpu-fps-1402.md §1 and §6). The engine's own values (cpu.busyPct, presentsPerSec) are measured.
// #1402 (measured, not judged): clockRate per window = the virtual clock's speed against the wall clock (the idles' ticks over their absolute ms; 1 = the same speed);
//   desk = the render Worker's counts per second over the window (ticksPerSec: the frames it asked the engine to compose; drawsPerSec: the pictures that changed (presents);
//   composeMsPerSec: the engine's compose time); windows: the titles of the window table (the top bar has no window of its own, so none is named here).
//   dwmPerSec per window = the engine's DwmFlush calls per second (the count reported with each idle, read at the window's ends).
//   hz.steps (judgement ②): native, then slow (the page's rAF made about 30 Hz by this tool only: each callback one frame later), then back. Each step is read HZ_SETTLE_MS after the change over
//   HZ_READ_MS: rafPerSec, envHz (window.__app.hz.hz), dwmPerSec. The page measures its rAF every 2 s and the engine takes a changed rate at its next idle (product: app/src/main.ts trackHz).
// stdout: ONE JSON line. Exit 1 when error is set, 2 when the arguments are wrong.
import fs from 'node:fs';
import os from 'node:os';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const argv = process.argv.slice(2);
const valueOf = (flag) => { const i = argv.indexOf(flag); return i < 0 ? null : argv[i + 1] ?? null; };
const SHOTS = valueOf('--shots');
const DPR_FIXED = valueOf('--dpr');
const DUMP = valueOf('--dump');
const positional = argv.filter((a, i) => !a.startsWith('--') && !['--dpr', '--shots', '--dump'].includes(argv[i - 1]));
if (positional.length !== 1) { process.stderr.write('usage: node tools/topbar-metrics/measure.mjs <absolute path to the bundle directory raw/<name>> [--dpr <value>] [--shots <directory>]\n'); process.exit(2); }
const bundleDir = path.resolve(positional[0]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');

const ROOM_W = 1836, ROOM_H = 1056;
const VIEW_START = { width: 2176, height: 1092 };
const READY_LIMIT_MS = 180000, IDLE_LIMIT_MS = 30000, CDP_TIMEOUT_MS = 20000, POLL_MS = 250;
const AUDIO_WAIT_MS = 5000, PLAY_WAIT_MS = 8000;
const IDLE_MS = 6000, MEASURE_MS = 8000;
// #1402: each Hz step is read HZ_SETTLE_MS after its change (the page measures every 2 s, the engine takes the rate at its next idle), over HZ_READ_MS
const HZ_SETTLE_MS = 10000, HZ_READ_MS = 5000;
const MEMORY_SAMPLES = 40, MEMORY_EVERY_MS = 1000, BAND_H = 96;
const METRONOME = { x: 750, y: 19 };
const MIB = 1048576;
// the top bar's Memory Usage is "<n> MB" with one space (its run is the whole text); the settings window's list is "<n>MB" (no space) and is not taken
const MB_RUN = /^(\d+) MB$/;
const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection,PrivacySandboxSettings4,Variations,SafeBrowsing,SafeBrowsingHashPrefixRealTimeLookups,HashPrefixRealTimeLookups,SafeBrowsingOnUIThread,SafeBrowsingNewGmsApiForBrowseUrlDatabaseCheck',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text',
];

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, error: a.error ? JSON.stringify(a.error).slice(0, 900) : null, title: document.title }; })()`;
const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const AUDIO = `(() => { const a = window.__app; return a && a.audio ? a.audio.detail() : null; })()`;
const AUDIO_AT = `(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const RAF_START = `(() => { window.__raf = { n: 0, on: true }; const f = () => { if (!window.__raf.on) return; window.__raf.n++; requestAnimationFrame(f); }; requestAnimationFrame(f); return true; })()`;
// the Worker's idles carry the absolute time (app.idles[].at = performance.timeOrigin + now), so the window is in that time too
const RAF_COUNT = `(() => ({ n: window.__raf.n, presents: window.__app.deskStats ? window.__app.deskStats.presents : null, t: performance.now(), abs: performance.timeOrigin + performance.now(), idles: window.__app.idles.map((x) => [x.at, x.busyMs, x.now, x.dwm]), desk: window.__app.deskStats ? { ticks: window.__app.deskStats.ticks, idleTicks: window.__app.deskStats.idleTicks, draws: window.__app.deskStats.draws, composeMs: window.__app.deskStats.composeMs } : null }))()`;
const RAF_STOP = `(() => { window.__raf.on = false; return true; })()`;
// #1402 (Hz follow, the stage 2 judgement): the page's requestAnimationFrame is made slower by this tool only (not the product): while `slow` is on, each callback runs one frame later, so a
// callback comes every second frame (about 30 Hz on a 60 Hz screen). The callbacks of every rAF of the page go through it (the product's too), and the tool's own counting loop too.
const HZ_INSTALL = `(() => { if (window.__hzWrap) return true; const raw = window.requestAnimationFrame.bind(window); window.__hzWrap = { slow: false }; window.requestAnimationFrame = (cb) => raw((t) => { if (window.__hzWrap.slow) raw(() => cb(t)); else cb(t); }); return true; })()`;
const HZ_SLOW = (on) => `(() => { window.__hzWrap.slow = ${on ? 'true' : 'false'}; return true; })()`;
const HZ_ENV = `window.__app.hz.hz`;
const K32_VALUES = `window.__app.engineLog().filter((l) => /memory-usage: K32GetProcessMemoryInfo/.test(l)).map((l) => l.trim())`;
const PAGE_MEASURE = `(async () => { if (!crossOriginIsolated || typeof performance.measureUserAgentSpecificMemory !== 'function') return null; return (await performance.measureUserAgentSpecificMemory()).bytes; })()`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const r3 = (x) => (x === null || x === undefined ? null : Math.round(x * 1000) / 1000);

function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { cls: names ? names[1] : '', title: names ? names[2] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  });
}

/** the "<n> MB" runs of a guestText answer: every run near a place of "MB" that reads as a number of megabytes (ASCII and UTF-16LE). key = the encoding and the place's offset */
function memoryRuns(text) {
  const out = [];
  for (const n of text.near) for (const run of n.runs) {
    const m = MB_RUN.exec(run.trim());
    if (m) out.push({ key: `${n.enc}@${n.offset}`, enc: n.enc, offset: n.offset, mb: Number(m[1]), run: run.trim() });
  }
  return out;
}

/** one reading of the guest's memory: the runs and the size of the wasm memory (guestText's memBytes) */
async function readRuns(ev) {
  // the marker has the space: the "MB" places are capped at 64 (TEXT_HITS, the lowest addresses first), the " MB" places are fewer
  const t = await ev(`window.__app.guestText([" MB"], 40)`, 60000);
  return { runs: memoryRuns(t), bytes: t.memBytes };
}

/**
 * The label among the samples ({ at, runs } in time order). A run whose value changes between two samples is redrawn: a copy that stays is never changed, a run that is
 * written again is the label (it may be redrawn in place, or written as a new copy, which is a run that did not exist before: both are a change). The change of each run is
 * the sample time at which its value first differs (or it first appears, after the first sample). The label is the run(s) that changed last: they must agree on the value,
 * and must still be in the last sample. Returns { label, changedAt, runsChanged, why }: label null (and why says what) when there is no label.
 */
export function findLabel(samples) {
  const changed = new Map();
  let prev = null;
  for (const s of samples) {
    const now = new Map(s.runs.map((r) => [r.key, r]));
    if (prev !== null) for (const [key, r] of now) {
      const before = prev.get(key);
      if (before === undefined || before.mb !== r.mb) changed.set(key, { at: s.at, ...r });
    }
    prev = now;
  }
  if (changed.size === 0) return { label: null, changedAt: null, runsChanged: 0, why: 'no run changed value in the sampled time (the value did not move)' };
  const last = Math.max(...[...changed.values()].map((c) => c.at));
  const newest = [...changed.values()].filter((c) => c.at === last);
  const values = [...new Set(newest.map((c) => c.mb))];
  if (values.length !== 1) return { label: null, changedAt: last, runsChanged: changed.size, why: `the newest changes disagree: ${values.join(', ')}` };
  const final = new Map(samples[samples.length - 1].runs.map((r) => [r.key, r]));
  const pick = newest.find((c) => final.has(c.key) && final.get(c.key).mb === values[0]);
  if (!pick) return { label: null, changedAt: last, runsChanged: changed.size, why: 'the newest changed run is not in the last sample' };
  return { label: { key: pick.key, enc: pick.enc, offset: pick.offset, mb: pick.mb, run: pick.run }, changedAt: last, runsChanged: changed.size, why: null };
}

/** the value of the label's run in one reading: null when the run is not there */
const valueAt = (runs, key) => { const r = runs.find((x) => x.key === key); return r ? r.mb : null; };

/** the bytes of K32GetProcessMemoryInfo's answers in the engine log, in the order they were given */
const k32Values = (ev) => ev(`window.__app.engineLog().filter((l) => /memory-usage: K32GetProcessMemoryInfo/.test(l)).map((l) => Number(/WorkingSetSize (\\d+)/.exec(l)[1]))`, 60000);
/** the K32 answers so far: how many, and the last one's bytes (null before the first) */
const K32_LAST = `(() => { const v = window.__app.engineLog().filter((l) => /memory-usage: K32GetProcessMemoryInfo/.test(l)); return { n: v.length, last: v.length ? Number(/WorkingSetSize (\\d+)/.exec(v[v.length - 1])[1]) : null }; })()`;

/** one sample of the top bar (#1391): the K32 answers so far, the "<n> MB" runs of the guest's memory, and the hash of the band of the top bar (a screenshot of its rows) */
async function memorySample(ev, S, band, t0) {
  const at = Date.now() - t0;
  const k = await ev(K32_LAST, 60000);
  const r = await readRuns(ev);
  const shot = await S('Page.captureScreenshot', { format: 'png', clip: { x: band.x, y: band.y, width: band.w, height: Math.min(BAND_H, band.h), scale: 1 } });
  return { at, k32: k.n, last: k.last, runs: r.runs, bytes: r.bytes, hash: createHash('sha256').update(shot.data).digest('hex').slice(0, 16) };
}

/** a measured window: the rAF callbacks, the presents and the Worker's busy fraction over `ms` */
async function window_(ev, ms) {
  await ev(RAF_START);
  const a = await ev(RAF_COUNT);
  await sleep(ms);
  const b = await ev(RAF_COUNT);
  await ev(RAF_STOP);
  const secs = (b.t - a.t) / 1000;
  const busyMs = b.idles.filter(([at]) => at > a.abs && at <= b.abs).reduce((s, [, busy]) => s + (typeof busy === 'number' ? busy : 0), 0);
  const first = b.idles.filter(([at]) => at > a.abs && at <= b.abs);
  // the engine's virtual clock against the wall clock over the window (the idles carry both: ticks of 100 ns and the absolute ms); 1.000 = the clock runs at the wall's speed
  const clockRate = first.length >= 2 ? r3(((first[first.length - 1][2] - first[0][2]) / 10000) / (first[first.length - 1][0] - first[0][0])) : null;
  // the render Worker's counts (deskStats, the beacon's desk): ticks = the frames it asked the engine to compose, presents = the changed pictures, draws = the engine's draws, composeMs = the engine's compose time
  const d = a.desk && b.desk ? { ticksPerSec: r3((b.desk.ticks - a.desk.ticks) / secs), idleTicksPerSec: r3((b.desk.idleTicks - a.desk.idleTicks) / secs), drawsPerSec: r3((b.desk.draws - a.desk.draws) / secs), composeMsPerSec: r3((b.desk.composeMs - a.desk.composeMs) / secs) } : null;
  // #1402: DwmFlush calls per second (the engine's count, reported with its idles: the last report before each end of the window)
  const dwmAt = (idles) => idles.reduce((v, x) => (typeof x[3] === 'number' ? x[3] : v), null);
  const dwmA = dwmAt(a.idles), dwmB = dwmAt(b.idles);
  return {
    seconds: r3(secs),
    rafPerSec: r3((b.n - a.n) / secs),
    dwmPerSec: dwmA === null || dwmB === null ? null : r3((dwmB - dwmA) / secs),
    presentsPerSec: a.presents === null || b.presents === null ? null : r3((b.presents - a.presents) / secs),
    busyPct: first.length >= 2 ? r3((100 * busyMs) / (b.abs - a.abs)) : null,
    clockRate,
    desk: d,
    idles: first.length,
  };
}

/** #1402 (judgement ②): native -> about 30 Hz -> native. Each step is changed, read HZ_SETTLE_MS later over HZ_READ_MS: rafPerSec (the page's rAF), envHz (window.__app.hz.hz: the rate the engine is given), dwmPerSec (the engine's DwmFlush calls) */
async function hzSteps(ev, shot) {
  await ev(HZ_INSTALL);
  const steps = [];
  for (const [name, slow] of [['native', false], ['slow', true], ['back', false]]) {
    await ev(HZ_SLOW(slow));
    await sleep(HZ_SETTLE_MS);
    const w = await window_(ev, HZ_READ_MS);
    const envHz = await ev(HZ_ENV);
    await shot(`hz-${name}`);
    steps.push({ name, slow, envHz, rafPerSec: w.rafPerSec, dwmPerSec: w.dwmPerSec, seconds: w.seconds, idles: w.idles });
    log('hz', steps[steps.length - 1]);
  }
  await ev(HZ_SLOW(false));
  return steps;
}

async function main() {
  const out = { error: null, memory: null, cpu: { idlePct: null, playingPct: null, busyPct: null, note: 'the top bar CPU Load is not identified in this version (plain digits, no unit): null' }, fps: { samples: [], note: 'the top bar Frame Rate is not identified in this version: shown is null' }, hz: { steps: [], note: 'Issue #1402: native, slow (about 30 Hz: the tool makes the rAF slower), back; rafPerSec is the page rAF, envHz the rate the engine is given, dwmPerSec the engine DwmFlush calls per second' }, phases: {}, notes: '' };
  let ev0 = null;
  let shotN = 0;
  let shot = async () => {};
  const root = fs.mkdtempSync(path.join(repo, '.tmp-topbar-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params, ms = CDP_TIMEOUT_MS) => timed(cdp.send(method, params, sessionId), ms, method);
    const ev = async (expression, ms = CDP_TIMEOUT_MS) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, ms);
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
      return r.result.value;
    };
    ev0 = ev;
    await S('Page.enable');
    shot = async (name) => { if (!SHOTS) return; const r = await S('Page.captureScreenshot', { format: 'png' }); fs.mkdirSync(SHOTS, { recursive: true }); fs.writeFileSync(path.join(SHOTS, `${String(++shotN).padStart(2, '0')}-${name}.png`), Buffer.from(r.data, 'base64')); };
    const dpr0 = DPR_FIXED ? Number(DPR_FIXED) : 1;
    // the size is set after the engine is idle: with the big viewport before the navigation the engine did not start (the state stayed running, no log line)
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const waitIdle = async (ms) => {
      const t0 = Date.now();
      for (;;) {
        const s = await ev(STATE).catch(() => null);
        if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} ${s.error ?? ''}`.trim());
        if (s && s.state === 'engine-idle') return s.state;
        if (Date.now() - t0 >= ms) return s ? s.state : null;
        await sleep(POLL_MS);
      }
    };
    if ((await waitIdle(READY_LIMIT_MS)) !== 'engine-idle') throw new Error('no engine-idle');
    await S('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: dpr0, mobile: false });
    await sleep(1500);
    let room = null;
    for (let i = 0; i < 4 && room === null; i++) {
      const r = await ev(ROOM);
      if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) room = r;
      else {
        await S('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: dpr0, mobile: false });
        await sleep(1500);
      }
    }
    if (room === null) throw new Error('the room is not 1836 x 1056 CSS px');
    const tap = async (code, key, vk) => {
      const text = code === 'Space' ? ' ' : undefined;
      await S('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, ...(text ? { text } : {}) });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    };
    const click = async (sx, sy) => {
      const g = await ev(GEOMETRY);
      const c = { x: g.x + (sx + 0.5) * (g.w / g.gw), y: g.y + (sy + 0.5) * (g.h / g.gh) };
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    };
    const windowsNow = async () => {
      const t0 = Date.now();
      for (;;) {
        try { return parseWindows(await ev(WINDOWS)); } catch (e) {
          if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
          await sleep(100);
        }
      }
    };
    const wiz = (await windowsNow()).find((w) => w.cls === 'TWelcomeWizard');
    if (wiz) {
      await click(wiz.x + wiz.w - 18, wiz.y + 18);
      const t0 = Date.now();
      while ((await windowsNow()).some((w) => w.cls === 'TWelcomeWizard')) {
        if (Date.now() - t0 >= 15000) throw new Error('the welcome wizard did not close');
        await sleep(POLL_MS);
      }
    }
    // the top bar's Memory Usage (#1391): MEMORY_SAMPLES samples, MEMORY_EVERY_MS apart, from the close of the welcome window on (the values move while the project opens: the K32 answers begin then)
    const samples = [];
    const seriesT0 = Date.now();
    const band = await ev(GEOMETRY);
    for (let i = 0; i < MEMORY_SAMPLES; i++) {
      samples.push(await memorySample(ev, S, band, seriesT0));
      await sleep(Math.max(0, seriesT0 + (i + 1) * MEMORY_EVERY_MS - Date.now()));
    }
    await sleep(1500);
    if ((await waitIdle(IDLE_LIMIT_MS)) !== 'engine-idle') throw new Error('the engine did not return to engine-idle after the wizard');
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    // the default project's "Problems loading the project" box is modal: Enter answers its default button
    for (let i = 0; i < 20; i++) {
      const titled = (await windowsNow()).filter((w) => w.title && /Problems loading/.test(w.title));
      if (titled.length === 0) break;
      await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
      await sleep(1000);
    }
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    log('project open', await ev(STATE));
    await shot('project-open');

    const found = findLabel(samples);
    const label = found.label;
    const pageBytes = samples[samples.length - 1].bytes;
    await shot('memory-label');
    out.memory = {
      shownMb: null,
      shownRun: null,
      shownWhy: null,
      candidates: [],
      zeroMbNew: null,
      hash: null,
      guess: label ? { mb: label.mb, run: label.run, key: label.key, changedAt: found.changedAt, runsChanged: found.runsChanged } : null,
      guessWhy: found.why,
      samples: samples.length,
      sampleSeconds: r3((samples[samples.length - 1].at - samples[0].at) / 1000),
      k32Bytes: [],
      pageBytes,
    };
    log('memory guess', { guess: out.memory.guess, why: out.memory.guessWhy });

    await sleep(IDLE_MS);
    out.phases.idle = await window_(ev, MEASURE_MS);
    if (label) {
      const r = await readRuns(ev);
      out.phases.idleMemory = { guessMb: valueAt(r.runs, label.key), runs: r.runs.length };
    }

    // playing: the audio output started by a gesture of the page's own, Space, the metronome
    const at = await ev(AUDIO_AT);
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
    await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    const running = async (ms) => {
      const t0 = Date.now();
      for (;;) {
        const d = await ev(AUDIO);
        if (d && d.ctxState === 'running' && d.nodeReady) return true;
        if (Date.now() - t0 >= ms) return false;
        await sleep(POLL_MS);
      }
    };
    if (!(await running(AUDIO_WAIT_MS))) { await tap('ShiftLeft', 'Shift', 16); if (!(await running(AUDIO_WAIT_MS))) throw new Error('the audio output did not start (no gesture took)'); }
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await tap('Space', ' ', 32);
    await sleep(500);
    await click(METRONOME.x, METRONOME.y);
    const n0 = (await ev(AUDIO)).nonSilentQuanta;
    const t0 = Date.now();
    for (;;) {
      const n1 = (await ev(AUDIO)).nonSilentQuanta;
      if (n1 > n0 + 20) break;
      if (Date.now() - t0 >= PLAY_WAIT_MS) throw new Error(`the playback is silent (nonSilentQuanta ${n0} -> ${n1})`);
      await sleep(POLL_MS);
    }
    await sleep(2000);
    out.phases.playing = await window_(ev, MEASURE_MS);
    if (label) {
      const r = await readRuns(ev);
      out.phases.playingMemory = { guessMb: valueAt(r.runs, label.key), runs: r.runs.length };
    }
    out.cpu.idlePct = null;
    out.cpu.playingPct = null;
    out.cpu.busyPct = out.phases.playing.busyPct;
    out.cpu.busyIdlePct = out.phases.idle.busyPct;

    // Frame Rate: DPR 1 and a random DPR in 1.01..3.00, in the playing state
    const dprs = DPR_FIXED ? [Number(DPR_FIXED)] : [1, Math.round((1.01 + Math.random() * 1.99) * 100) / 100];
    for (const dpr of dprs) {
      await S('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: dpr, mobile: false });
      await sleep(3000);
      const w = await window_(ev, MEASURE_MS);
      const room2 = await ev(ROOM);
      out.fps.samples.push({ dpr, shown: null, rafPerSec: w.rafPerSec, presentsPerSec: w.presentsPerSec, clockRate: w.clockRate, desk: w.desk, seconds: w.seconds, roomCss: [room2.sw, room2.sh] });
      log('dpr', dpr, w);
    }
    // #1402: the Hz follow (native, slow, back), in the playing state, at the DPR of the last Frame Rate sample
    out.hz.steps = await hzSteps(ev, shot);
    // the K32 answers (the engine log's lines, in order), then a last reading of the guest's memory (after the last answer)
    out.memory.k32Bytes = await k32Values(ev);
    const fin = await readRuns(ev);
    const last = out.memory.k32Bytes.length ? out.memory.k32Bytes[out.memory.k32Bytes.length - 1] : null;
    const all = [...samples, { at: null, k32: out.memory.k32Bytes.length, last, runs: fin.runs, bytes: fin.bytes, hash: null }];
    const candidates = [...new Set(all.flatMap((s) => s.runs.map((r) => r.mb)))].sort((a, b) => a - b);
    const zeroAfter = new Set(), zeroBefore = new Set();
    for (const s of all) for (const r of s.runs) if (r.mb === 0) (s.k32 >= 1 ? zeroAfter : zeroBefore).add(r.key);
    const floor = last === null ? null : Math.floor(last / MIB);
    const near = candidates.filter((q) => floor !== null && Math.abs(q - floor) <= 1);
    const shownMb = floor === null ? null : near.includes(floor) ? floor : near.length === 1 ? near[0] : null;
    out.memory.candidates = candidates;
    out.memory.zeroMbNew = [...zeroAfter].filter((k) => !zeroBefore.has(k)).length;
    out.memory.shownMb = shownMb;
    out.memory.shownRun = shownMb === null ? null : `${shownMb} MB`;
    out.memory.shownWhy = shownMb === null
      ? `no candidate is the K32 floor (${floor}) or the one within +-1 (near: ${near.join(',') || 'none'}); not guessed`
      : near.includes(floor) ? `the K32 floor ${floor} is a candidate (a copy of the text reads ${floor} MB)` : `the one candidate within +-1 of the K32 floor ${floor}: ${near[0]}`;
    out.memory.hash = {
      method: 'band',
      bandCss: { y: band.y, height: Math.min(BAND_H, band.h), width: band.w },
      series: samples.map((s) => s.hash),
      floors: samples.map((s) => (s.last === null ? null : Math.floor(s.last / MIB))),
      note: 'band: the top bar rows (not the label rectangle). The change timing against the floors is reported, not judged for band.',
    };
    const pageMeasure = await ev(PAGE_MEASURE, 30000).catch(() => null);
    out.memory.pageMeasure = pageMeasure !== null
      ? { method: 'performance.measureUserAgentSpecificMemory', bytes: pageMeasure }
      : { method: 'WebAssembly.Memory (the guest-text memBytes)', bytes: pageBytes };
    out.memory.ratio = out.memory.shownMb === null || !out.memory.pageMeasure.bytes ? null : r3((out.memory.shownMb * MIB) / out.memory.pageMeasure.bytes);
    out.notes = `nproc ${os.availableParallelism?.() ?? os.cpus().length}, loadavg ${os.loadavg().map((x) => x.toFixed(2)).join(' ')}`;
    out.windows = (await windowsNow()).filter((w) => w.title !== '' || w.cls !== '');
    if (DUMP) fs.writeFileSync(DUMP, String(await ev(`window.__app.engineLog().join('\\n')`, 60000).catch(() => '')));
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
    if (ev0) out.engineTail = await ev0(`window.__app.engineLog().filter((l) => /K32|RangeError|fle: |trap/.test(l)).slice(0, 120).concat(window.__app.engineLog().slice(-30))`, 20000).catch(() => null);
    if (ev0 && DUMP) fs.writeFileSync(DUMP, String(await ev0(`window.__app.engineLog().join('\\n')`, 60000).catch(() => '')));
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  if (out.memory === null) out.memory = { shownMb: null, shownRun: null, shownWhy: 'not read', candidates: [], zeroMbNew: null, hash: null, guess: null, guessWhy: 'not read', k32Bytes: [], pageMeasure: null, ratio: null };
  out.cpu.playingPct = null;
  out.cpu.idlePct = null;
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
