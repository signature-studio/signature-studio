// Engine cost, stage 0 (the baseline for the lift work): one scene of the product in headless Chrome at a device pixel ratio, with the CPU profile of every thread of the page (its main thread, and each Worker attached with Target.setAutoAttach) over the scene's window. The product is not changed: nothing is enabled for the measurement (no flag, no URL parameter, no environment variable).
// Usage: node tools/engine-cost/measure.mjs --bundle <absolute path to raw/<name>> --names <absolute path to wasm-func-names.tsv> --scene i|ii|iii --dpr <number> --out <absolute path to the result JSON> [--window-s <seconds>] [--keep-profiles]
// Scenes (t0 is the first observation of engine-idle). i: nothing is sent; the window is t0+30 s to t0+90 s. ii: at t0 the audio output is started (a press on the status line; a Shift key press if that does not start it), the welcome window is closed as tools/p2-frames does for load b, Space starts the transport, and the window is the 60 s from the first sign of playback (quantaWhilePlaying rises). iii: the welcome window is closed as above; the window is t0+30 s to t0+90 s, and the pointer goes round an ellipse over the Playlist at 1200 Hz (no wheel, no audio).
// Classes of a profile's self time (the leaf frame of each sample, for its interval to the next sample; (idle) is not busy): A emscripten runtime and glue (the glue JS, emscripten names, and wasm functions with such names); B lifted C functions (wasm f_<hex>); C C++ drawing (wasm bl::); D runtime C (wasm fle_* and poll_waiters); E the app's JS (127.0.0.1 URLs) and browser APIs; F browser internals ((program), (garbage collector), (root)); unclassified the rest.
// stdout and --out: one JSON line. Exit 0 when error is null, 1 when the run failed (the JSON is still written), 2 when the arguments are wrong (usage on stderr; nothing is written).
// Raw profiles go to .tmp-engine-cost-<pid>/profiles and are removed at the end. --keep-profiles keeps them (and removes the Chrome profile and the asset links).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const USAGE = 'usage: node tools/engine-cost/measure.mjs --bundle <absolute path to raw/<name>> --names <absolute path to wasm-func-names.tsv> --scene i|ii|iii --dpr <number> --out <absolute path to the result JSON> [--window-s <seconds, default 60>] [--keep-profiles]\n';
const VALUE_FLAGS = ['--bundle', '--names', '--scene', '--dpr', '--out', '--window-s', '--app'];
const SCENES = ['i', 'ii', 'iii'];
const argv = process.argv.slice(2);
const valueOf = (flag) => { const i = argv.indexOf(flag); return i < 0 ? undefined : argv[i + 1]; };
const shapeOk = argv.every((a, i) => {
  if (a === '--keep-profiles') return true;
  if (VALUE_FLAGS.includes(a)) return i + 1 < argv.length && !argv[i + 1].startsWith('--');
  return i > 0 && VALUE_FLAGS.includes(argv[i - 1]);
});
const bundleArg = valueOf('--bundle');
const namesArg = valueOf('--names');
const SCENE = valueOf('--scene');
const dprArg = valueOf('--dpr');
const OUT = valueOf('--out');
const DPR = dprArg === undefined ? NaN : Number(dprArg);
const windowArg = valueOf('--window-s');
const WINDOW_S = windowArg === undefined ? 60 : Number(windowArg);
const KEEP_PROFILES = argv.includes('--keep-profiles');
const argsOk = shapeOk
  && [bundleArg, namesArg, OUT].every((p) => p !== undefined && path.isAbsolute(p))
  && SCENES.includes(SCENE) && Number.isFinite(DPR) && DPR > 0
  && Number.isFinite(WINDOW_S) && WINDOW_S > 0;
if (!argsOk) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(bundleArg);
const BUNDLE = path.basename(bundleDir);
const NAMES = path.resolve(namesArg);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = valueOf('--app') ?? path.join(repo, 'app');   // --app: the app served (an absolute path with a built dist); default: this repository's app
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 50;
const CDP_TIMEOUT_MS = 20000;
const PROFILE_INTERVAL_US = 250;
const PROFILE_STOP_MS = 300000;
// the room of tools/p3-audio-probe (BOOT_VIEWPORT): the stage is 1836 x 1056 CSS px, with the 340 px panel and the 36 px header
const VIEWPORT = { width: 1836 + 340, height: 1056 + 36 };
// the window of a scene is WINDOW_S long from WINDOW_START_MS (60 s by default; run.sh --quick uses 30 s)
const WINDOW_START_MS = 30000;
const WINDOW_END_MS = WINDOW_START_MS + WINDOW_S * 1000;
const PLAY_WINDOW_MS = WINDOW_S * 1000;
const PLAY_WAIT_MS = 15000;
const AUDIO_WAIT_MS = 5000;
// the pointer of tools/p2-frames load a: 1200 Hz (1000 is the least the input must reach), two turns a second, over the Playlist
const SEND_HZ = 1200;
const REV_PER_S = 2;
const PLAYLIST = { cx: 0.78, cy: 0.55, rx: 0.05, ry: 0.05 };
const WIZARD_CLASS = 'TWelcomeWizard';
const WIZARD_LIMIT_MS = 15000;
const IDLE_LIMIT_MS = 30000;
const CLASSES = ['A', 'B', 'C', 'D', 'E', 'F', 'unclassified'];

// the isolation flags of tools/boot-check/boot-bundle.mjs (copied: that file does not export them)
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

const STATE = 'window.__app ? window.__app.state : null';
const AUDIO = 'window.__app && window.__app.audio ? window.__app.audio.detail() : null';
const AUDIO_AT = "(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()";
const GEOMETRY = "(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()";
const GUEST = "(() => { const g = window.__app.screen.guest; return g.width + 'x' + g.height; })()";
const WINDOWS = '(async () => (await window.__app.deskDumpBase64(false)).windows)()';
const FOCUS_DESKTOP = "document.getElementById('desktop').focus({ preventScroll: true }); true";

// the class of a frame. The names of the lifted C functions are the table's (tools/p3-audio-probe/profile-names.mjs reads the same table; it runs on import, so its reading is copied here)
const GLUE_URL = /engine-glue\.js|flengine6\.js/;
const APP_URL = /^(?:blob:)?https?:\/\/127\.0\.0\.1[:/]/;
const EMS_NAME = /^_?emscripten_|^__syscall_|^invoke_|^HEAP|^updateMemoryViews$|^Atomics\.wait$|^fle_js_wait$/;
const EMS_WASM = /^__wasm_|^_?pthread_|^__pthread_|^__futexwait$|^_emscripten_/;
const WAIT_NAME = /^(?:Atomics\.wait|_?emscripten_futex_wait)$/;
const BROWSER_API = new Set(['postMessage', 'decode', 'setTimeout', 'clearTimeout', 'setInterval', 'clearInterval', 'structuredClone', 'requestAnimationFrame', 'cancelAnimationFrame', 'queueMicrotask', 'fetch']);
const BROWSER_INTERNAL = new Set(['(program)', '(garbage collector)', '(root)']);

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const r1 = (x) => Math.round(x * 10) / 10;
const r3 = (x) => Math.round(x * 1000) / 1000;
const errText = (e) => String(e?.message ?? e).slice(0, 300);
const sleepUntil = async (at) => { const d = at - Date.now(); if (d > 0) await sleep(d); };

// the names table (lift/out-app/gen/wasm-func-names.tsv: index, kind, va, name; a header line). kind f and ia are lifted code (f_<hex>, ia_<hex>); kind c is a C function by its symbol, or unnamed_<index> for a function that has no name (Issue #1396). A missing file or an unnamed row leaves the wasm frame unknown (unclassified).
function loadNames(file) {
  const names = new Map();
  if (!fs.existsSync(file)) return { names, found: false, count: 0 };
  for (const line of fs.readFileSync(file, 'utf8').split('\n').slice(1)) {
    const cols = line.replace(/\r$/, '').split('\t');
    if (cols.length >= 4 && /^\d+$/.test(cols[0]) && cols[3] !== '') names.set(Number(cols[0]), { name: cols[3], kind: cols[1] });
  }
  return { names, found: true, count: names.size };
}

// B: lifted code (f_<hex>, ia_<hex>). C: Blend2D (bl::, and the bl<Name> C API). A: emscripten and pthread. D: the runtime's fle_* and poll_waiters, and the other C functions of the wasm (the C runtime and libc: notes say so).
function wasmClass(name, kind) {
  if (/^(?:f|ia)_[0-9a-f]+$/.test(name)) return 'B';
  if (/^bl(?:::|[A-Z])/.test(name)) return 'C';
  if (EMS_NAME.test(name) || EMS_WASM.test(name)) return 'A';
  if (/^fle_/.test(name) || name === 'poll_waiters') return 'D';
  if (kind === 'c') return 'D';
  return 'unclassified';
}

function classify(frame, names) {
  const fn = frame.functionName || '(anonymous)';
  const url = frame.url || '';
  if (fn === '(idle)') return { k: 'idle', wait: false, unknown: false };
  if (BROWSER_INTERNAL.has(fn)) return { k: 'F', wait: false, unknown: false };
  const w = /^wasm-function\[(\d+)\]$/.exec(fn);
  if (w) {
    const entry = names.get(Number(w[1]));
    if (entry === undefined) return { k: 'unclassified', wait: false, unknown: true };
    return { k: wasmClass(entry.name, entry.kind), wait: WAIT_NAME.test(entry.name), unknown: false };
  }
  let k = 'unclassified';
  if (GLUE_URL.test(url) || EMS_NAME.test(fn)) k = 'A';
  else if (APP_URL.test(url) || BROWSER_API.has(fn) || url === '') k = 'E';
  return { k, wait: WAIT_NAME.test(fn), unknown: false };
}

// one profile: the self time of each class (the sample's interval to the next sample), the idle time, the Atomics.wait time, the wasm time with no name
function analyze(prof, names) {
  if (!Array.isArray(prof.nodes) || !Array.isArray(prof.samples) || !Array.isArray(prof.timeDeltas)) throw new Error('the profile has no nodes, samples or timeDeltas');
  const byId = new Map(prof.nodes.map((n) => [n.id, n]));
  const cache = new Map();
  const sums = Object.fromEntries(CLASSES.map((k) => [k, 0]));
  let idle = 0, wait = 0, unknownWasm = 0;
  for (let i = 0; i < prof.samples.length; i++) {
    const id = prof.samples[i];
    if (!cache.has(id)) cache.set(id, classify(byId.get(id)?.callFrame ?? {}, names));
    const c = cache.get(id);
    const dt = (prof.timeDeltas[i + 1] ?? 0) / 1000;
    if (c.k === 'idle') { idle += dt; continue; }
    sums[c.k] += dt;
    if (c.wait) wait += dt;
    if (c.unknown) unknownWasm += dt;
  }
  const busy = CLASSES.reduce((a, k) => a + sums[k], 0);
  return { busy, idle, sums, wait, unknownWasm };
}

// the page's main thread and each Worker attached so far (Target.setAutoAttach, flatten)
function profiledThreads(cx) {
  return [{ label: 'main', url: 'main', sid: cx.pageSid }, ...cx.workers.map((w, k) => ({ label: `worker-${k}`, url: w.url, sid: w.sid }))];
}

async function profileStart(cx) {
  const started = [];
  for (const t of profiledThreads(cx)) {
    try {
      await cx.T('Profiler.enable', {}, t.sid);
      await cx.T('Profiler.setSamplingInterval', { interval: PROFILE_INTERVAL_US }, t.sid);
      await cx.T('Profiler.start', {}, t.sid);
      started.push(t);
    } catch (e) {
      cx.failures.push(`${t.label} start: ${errText(e)}`);
    }
  }
  log('profiling', { threads: started.map((t) => t.label) });
  return started;
}

async function profileStop(cx, started) {
  const done = [];
  for (const t of started) {
    try {
      const r = await cx.T('Profiler.stop', {}, t.sid, PROFILE_STOP_MS);
      fs.writeFileSync(path.join(cx.profilesDir, `${t.label}.json`), JSON.stringify(r.profile));
      done.push({ label: t.label, url: t.url === 'main' ? 'main' : (t.url || null), analysis: analyze(r.profile, cx.names.names) });
    } catch (e) {
      cx.failures.push(`${t.label} stop: ${errText(e)}`);
    }
  }
  return done;
}

// the engine-idle state, polled; returns the time of the first observation
async function waitIdle(cx, limitMs) {
  const t0 = Date.now();
  for (;;) {
    const st = await cx.ev(STATE).catch(() => null);
    if (st === 'engine-idle') return Date.now();
    if (st === 'error' || st === 'exited') throw new Error(`state ${st} before the engine was idle`);
    if (Date.now() - t0 >= limitMs) throw new Error(`no engine-idle within ${limitMs} ms (state ${st ?? 'window.__app not set'})`);
    await sleep(POLL_MS);
  }
}

async function audioNow(cx) { return cx.ev(AUDIO); }

async function underrunNow(cx) { return (await audioNow(cx))?.underrunQuanta ?? null; }

async function tapKey(cx, code, key, vk) {
  const text = code === 'Space' ? ' ' : undefined;
  await cx.S('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, ...(text ? { text } : {}) });
  await sleep(40);
  await cx.S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}

async function audioRunning(cx, ms) {
  const t0 = Date.now();
  for (;;) {
    const d = await audioNow(cx);
    if (d && d.ctxState === 'running' && d.nodeReady) return true;
    if (Date.now() - t0 >= ms) return false;
    await sleep(POLL_MS);
  }
}

// the page's own start (tools/p3-audio-probe startAudio): a press and a release on the status line, which is outside the desktop (the guest gets no event); a Shift key press if that does not start it
async function startAudio(cx) {
  const at = await cx.ev(AUDIO_AT);
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await cx.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  if (await audioRunning(cx, AUDIO_WAIT_MS)) return 'mouse';
  await tapKey(cx, 'ShiftLeft', 'Shift', 16);
  if (await audioRunning(cx, AUDIO_WAIT_MS)) return 'key';
  return null;
}

async function clickGuest(cx, gx, gy, geo) {
  const c = { x: geo.x + (gx + 0.5) * (geo.w / geo.gw), y: geo.y + (gy + 0.5) * (geo.h / geo.gh) };
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(120);
  await cx.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(70);
  await cx.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(60);
}

/** the guest's windows (the page's format: lid N at (x, y) WxH ... class "..." title "..."), as tools/p2-frames reads them */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { cls: names ? names[1] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  });
}

// the page refuses the window list while the picture is being drawn ("ask again"): it is asked again
async function windowsNow(cx) {
  const t0 = Date.now();
  for (;;) {
    try {
      return parseWindows(await cx.ev(WINDOWS));
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
      await sleep(100);
    }
  }
}

// the welcome window (and its usage-data question), if the page shows it: its X, as tools/p2-frames does for load b; then the engine is idle again
async function closeWelcome(cx) {
  const wiz = (await windowsNow(cx)).find((w) => w.cls === WIZARD_CLASS);
  if (!wiz) return false;
  await clickGuest(cx, wiz.x + wiz.w - 18, wiz.y + 18, await cx.ev(GEOMETRY));
  const t0 = Date.now();
  while ((await windowsNow(cx)).some((w) => w.cls === WIZARD_CLASS)) {
    if (Date.now() - t0 >= WIZARD_LIMIT_MS) throw new Error('the welcome window did not close');
    await sleep(POLL_MS);
  }
  await sleep(1500);
  await waitIdle(cx, IDLE_LIMIT_MS);
  return true;
}

function pointerAt(geo, i) {
  const th = 2 * Math.PI * REV_PER_S * (i / SEND_HZ);
  return { x: geo.x + (PLAYLIST.cx + PLAYLIST.rx * Math.cos(th)) * geo.w, y: geo.y + (PLAYLIST.cy + PLAYLIST.ry * Math.sin(th)) * geo.h };
}

// the pointer is sent on the clock (catch-up, not a timer per event), as tools/p2-frames does; st.sent counts the events
async function pointerLoop(cx, geo, st) {
  const t0 = performance.now();
  while (!st.stop) {
    const due = Math.floor(((performance.now() - t0) * SEND_HZ) / 1000);
    while (st.sent < due) {
      const p = pointerAt(geo, st.sent);
      cx.cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y, button: 'none', buttons: 0, pointerType: 'mouse' }, cx.pageSid).catch(() => {});
      st.sent++;
    }
    await sleep(2);
  }
}

// scene i: nothing is sent
async function sceneI(cx, tIdle) {
  await sleepUntil(tIdle + WINDOW_START_MS);
  const u0 = await underrunNow(cx);
  const tA = Date.now();
  const started = await profileStart(cx);
  await sleepUntil(tIdle + WINDOW_END_MS);
  const u1 = await underrunNow(cx);
  const tB = Date.now();
  const profiles = await profileStop(cx, started);
  return {
    windowMs: tB - tA, u0, u1, profiles,
    text: `Scene i: nothing is sent; the window is t0+${tA - tIdle} ms to t0+${tB - tIdle} ms. The audio output is not started in this scene, so underrunQuanta is the page's count with no sound playing.`,
  };
}

// scene ii: the audio output at t0, Space, and the 60 s from the start of playback
async function sceneII(cx, tIdle) {
  const gesture = await startAudio(cx);
  const tGesture = Date.now() - tIdle;
  log('AudioContext (scene ii)', { gesture, atMs: tGesture });
  if (gesture === null) {
    const d = await audioNow(cx);
    throw new Error(`the AudioContext did not start after a press and a release on the status line and after a Shift key press (ctxState ${d?.ctxState ?? 'no audio'})`);
  }
  const welcome = await closeWelcome(cx);
  await cx.ev(FOCUS_DESKTOP);
  const q0 = (await audioNow(cx)).quantaWhilePlaying;
  const ns0 = (await audioNow(cx)).nonSilentQuanta;
  await tapKey(cx, 'Space', ' ', 32);
  const tSpace = Date.now();
  let d = await audioNow(cx);
  while (!(d.quantaWhilePlaying > q0)) {
    if (Date.now() - tSpace >= PLAY_WAIT_MS) throw new Error(`the transport did not play: quantaWhilePlaying stayed at ${q0} for ${PLAY_WAIT_MS} ms after Space`);
    await sleep(POLL_MS);
    d = await audioNow(cx);
  }
  const tPlay = Date.now();
  const u0 = d.underrunQuanta;
  log('playing (scene ii)', { atMs: tPlay - tIdle, quanta: d.quantaWhilePlaying });
  const started = await profileStart(cx);
  await sleepUntil(tPlay + PLAY_WINDOW_MS);
  const d1 = await audioNow(cx);
  const tB = Date.now();
  const profiles = await profileStop(cx, started);
  return {
    windowMs: tB - tPlay, u0, u1: d1.underrunQuanta, profiles,
    text: `Scene ii: the audio output was started at t0+${tGesture} ms (${gesture}); Space started the transport at t0+${tPlay - tIdle} ms (quantaWhilePlaying ${q0} -> ${d.quantaWhilePlaying}); the window is the 60 s from there, to t0+${tB - tIdle} ms. The welcome window ${welcome ? 'was closed' : 'was not listed'}. The metronome was not pressed (the scene is Space only), so the Default project plays with no sound: nonSilentQuanta ${ns0} -> ${d1.nonSilentQuanta}.`,
  };
}

// scene iii: the pointer on the Playlist for the window, no wheel and no audio
async function sceneIII(cx, tIdle) {
  const welcome = await closeWelcome(cx);
  await sleepUntil(tIdle + WINDOW_START_MS);
  const geo = await cx.ev(GEOMETRY);
  const u0 = await underrunNow(cx);
  const tA = Date.now();
  const started = await profileStart(cx);
  const pointer = { stop: false, sent: 0 };
  const pointerP = pointerLoop(cx, geo, pointer);
  await sleepUntil(tIdle + WINDOW_END_MS);
  pointer.stop = true;
  await pointerP;
  const u1 = await underrunNow(cx);
  const tB = Date.now();
  const profiles = await profileStop(cx, started);
  const sec = (tB - tA) / 1000;
  return {
    windowMs: tB - tA, u0, u1, profiles,
    text: `Scene iii: the window is t0+${tA - tIdle} ms to t0+${tB - tIdle} ms; the pointer sent ${pointer.sent} events (${(pointer.sent / sec).toFixed(0)}/s, ${SEND_HZ}/s nominal) on the Playlist ellipse, ${REV_PER_S} turns a second; no wheel. The welcome window ${welcome ? 'was closed before the window' : 'was not listed'}. The audio output is not started in this scene, so underrunQuanta is the page's count with no sound playing.`,
  };
}

// the numbers over every profiled thread: the classes, busyMs and idleMs are sums (so they can be more than the window)
function fill(out, profiles) {
  const sums = Object.fromEntries(CLASSES.map((k) => [k, 0]));
  let busy = 0, idle = 0, wait = 0, unknownWasm = 0;
  for (const p of profiles) {
    for (const k of CLASSES) sums[k] += p.analysis.sums[k];
    busy += p.analysis.busy;
    idle += p.analysis.idle;
    wait += p.analysis.wait;
    unknownWasm += p.analysis.unknownWasm;
  }
  out.busyMs = r1(busy);
  out.idleMs = r1(idle);
  out.classesMs = Object.fromEntries(CLASSES.map((k) => [k, r1(sums[k])]));
  out.classifiedShare = busy > 0 ? r3((busy - sums.unclassified) / busy) : 0;
  out.atomicsWaitMs = r1(wait);
  out.workers = profiles.map((p) => ({ url: p.url, busyMs: r1(p.analysis.busy) }));
  return { unknownWasm };
}

async function main() {
  const out = {
    scene: SCENE, dpr: DPR, busyMs: null, idleMs: null, classesMs: null, classifiedShare: null,
    boundaryCallsPerSec: null, jspi: null, memoryGrow: null,
    atomicsWaitMs: null, underrunQuanta: null, elapsedMs: null, preBootBytes: null,
    workers: [], windowMs: null, notes: '', error: null,
  };
  const root = path.join(repo, `.tmp-engine-cost-${process.pid}`);
  const assets = path.join(root, 'assets');
  const chromeProfile = path.join(root, 'chrome-profile');
  const profilesDir = path.join(root, 'profiles');
  const failures = [];
  let server = null, chrome = null, cdp = null;
  try {
    fs.mkdirSync(profilesDir, { recursive: true });
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });

    chrome = await launchChrome({ profile: chromeProfile, extraArgs: ISOLATION });
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId: pageSid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const cx = {
      cdp, pageSid, workers: [], failures, profilesDir, names: loadNames(NAMES),
      T: (method, params, sid, ms = CDP_TIMEOUT_MS) => timed(cdp.send(method, params, sid), ms, method),
    };
    cx.S = (method, params = {}) => cx.T(method, params, pageSid);
    cx.ev = async (expression) => {
      const r = await cx.S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text ?? '').slice(0, 200)}`);
      return r.result.value;
    };
    // the preBoot bytes: the encoded length of each finished request (page and attached Workers), counted once per requestId
    const net = { ids: new Set(), bytes: 0 };
    cdp.on((m) => {
      if (m.sessionId === pageSid && m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') {
        cx.workers.push({ sid: m.params.sessionId, url: m.params.targetInfo.url || null });
        cdp.send('Network.enable', {}, m.params.sessionId).catch(() => {});
      }
      if (m.method === 'Network.loadingFinished' && !net.ids.has(m.params.requestId)) {
        net.ids.add(m.params.requestId);
        net.bytes += Math.max(0, m.params.encodedDataLength ?? 0);
      }
    });

    await cx.S('Page.enable');
    await cx.S('Emulation.setDeviceMetricsOverride', { width: VIEWPORT.width, height: VIEWPORT.height, deviceScaleFactor: DPR, mobile: false });
    await cx.S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
    await cx.S('Network.enable');
    const navAt = Date.now();
    const nav = await cx.S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const tIdle = await waitIdle(cx, WAIT_LIMIT_MS);
    out.elapsedMs = tIdle - navAt;
    out.preBootBytes = net.bytes;
    log('engine-idle', { ms: out.elapsedMs });
    const guest = await cx.ev(GUEST).catch(() => 'unknown');

    const win = SCENE === 'i' ? await sceneI(cx, tIdle) : SCENE === 'ii' ? await sceneII(cx, tIdle) : await sceneIII(cx, tIdle);
    if (win.profiles.length === 0) throw new Error('no thread was profiled (every Profiler.start failed)');
    out.underrunQuanta = win.u0 === null || win.u1 === null ? null : win.u1 - win.u0;
    out.windowMs = r1(win.windowMs);
    const { unknownWasm } = fill(out, win.profiles);
    const chromeVersion = chrome.version.Browser;
    out.notes = [
      win.text,
      `Chrome ${chromeVersion}, viewport ${VIEWPORT.width}x${VIEWPORT.height} CSS px at DPR ${DPR}, guest screen ${guest} device px; t0 is ${out.elapsedMs} ms after navigate.`,
      `Profiled threads: ${win.profiles.map((p) => (p.url === 'main' ? 'main' : `${p.url ?? 'no url'}`)).join(', ')}; ${cx.workers.length} Worker(s) were attached in all. Only the page's main thread and its Worker targets are profiled (no GPU process, no compositor). classesMs, busyMs, idleMs and atomicsWaitMs are sums over the profiled threads.`,
      `Classes: A emscripten runtime and glue (glue JS and emscripten names; wasm functions named emscripten_*, __wasm_*, pthread_*, __futexwait); B wasm f_<hex> and ia_<hex> (lifted code); C wasm bl::, bl<Name> (Blend2D); D wasm fle_* and poll_waiters, and the other C functions of the wasm (the C runtime and libc, e.g. memcpy, malloc); E the app's JS (127.0.0.1 URLs) and browser APIs; F (program), (garbage collector), (root); E also takes the native frames with no URL (postMessage, setTimeout, now, getElementById, and the V8 boundary frames wasm-to-js and js-to-wasm). unclassified the rest, including the wasm functions with no name in the table (${r1(unknownWasm)} ms). EM_JS fle_js_* functions are JS frames of the glue, so they are A.`,
      `atomicsWaitMs: the self time of leaves named Atomics.wait or emscripten_futex_wait. fle_js_wait (A) is not in it.`,
      cx.names.found ? `Names: ${cx.names.count} rows from ${NAMES}.` : `Names: ${NAMES} does not exist; every wasm frame is unclassified.`,
      `preBootBytes: encodedDataLength of each Network.loadingFinished (page and attached Workers, each requestId once) before the first engine-idle observation. A Worker's loads that finished before its attach are not seen.`,
      `boundaryCallsPerSec, jspi and memoryGrow are null: each needs a count taken inside the product (a wrapper on the wasm imports, a JSPI flag, a memory.grow hook), and the product is not changed for a measurement.`,
      failures.length > 0 ? `Failures: ${failures.join('; ')}.` : '',
      `Runner: nproc ${os.availableParallelism?.() ?? os.cpus().length}, loadavg ${os.loadavg().map((x) => x.toFixed(2)).join(' ')} at the end.`,
    ].filter(Boolean).join(' ');
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    if (KEEP_PROFILES) {
      fs.rmSync(assets, { recursive: true, force: true });
      fs.rmSync(chromeProfile, { recursive: true, force: true });
    } else {
      fs.rmSync(root, { recursive: true, force: true });
    }
  }
  const line = JSON.stringify(out) + '\n';
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, line);
  process.stdout.write(line);
  process.exitCode = out.error ? 1 : 0;
}

await main();
