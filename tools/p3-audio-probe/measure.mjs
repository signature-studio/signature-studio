// P3 audio probe (Issue #1303): starts the page's AudioContext with CDP Input events (a press and a release on the page's status line, which the browser counts as a user gesture), starts the transport with a Space key, and reads the audio output's numbers (window.__app.audio.detail(): decision 0175 / 0211) over about 10 s. The product is not changed: every input is sent from here, and the start-up consent box is closed with "No thanks" from here too.
// Usage: node tools/p3-audio-probe/measure.mjs <absolute path to the bundle directory raw/<name>> [<absolute path to a .flp project>] [--load none|ui] > probe.json
// --load ui: during the 10 s run the pointer moves over the guest's main window every 50 ms (CDP Input.dispatchMouseEvent), so the engine has UI work to do (#1304). The default is none.
// --boot (#1334): the start-up window with nothing sent to the guest. The room is 1836x1056 CSS px at DPR 1.7647 (the guest screen is 3240x1863 device px). t0 is the first observation of engine-idle; the one press on the status line (it starts the AudioContext, no guest event) comes before the window; the numbers are read at t0+10 s and t0+70 s. maxRefillStallMs is the engine's own longest interval between two refills of the output ring (fle_audio_transport.c, the Worker's fleAudioWorkerState).
// --boot options (#1334, only with --boot): --press-at S: the press that starts the AudioContext comes S seconds (0 to 60) after engine-idle (default 0). --trace FILE: every 50 ms in the window, the page's underrun quanta and playing quanta and the engine's WR, RD (frames), publishes and maxPubGapMs, as TSV (tools/p3-audio-probe/trace-stats.mjs reads it). --profile FILE: the CDP CPU profile of the engine's Worker over the window (tools/p3-audio-probe/profile-names.mjs maps its wasm-function[N] to names). Both are measurements: they add CDP calls, so their runs are not the acceptance runs.
// stdout: one JSON line. stderr: the log. Exit 1 when "error" is set, 2 when the arguments are wrong. The temporary directory is inside the repository (.tmp-p3-probe-*) and is removed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const USAGE = 'usage: node tools/p3-audio-probe/measure.mjs <absolute path to the bundle directory raw/<name>> [<absolute path to a .flp project>] [--load none|ui] [--boot [--press-at S] [--trace FILE] [--profile FILE]]\n';
const argv = process.argv.slice(2);
const boot = argv.includes('--boot');
const li = argv.indexOf('--load');
const load = li < 0 ? 'none' : argv[li + 1];
const BOOT_VALUE_OPTS = ['--press-at', '--trace', '--profile'];
const valueOf = (name) => { const i = argv.indexOf(name); return i < 0 ? undefined : argv[i + 1]; };
const bootOptUsed = BOOT_VALUE_OPTS.some((o) => argv.includes(o));
const pressRaw = valueOf('--press-at');
const pressAt = pressRaw === undefined ? 0 : Number(pressRaw);
const tracePath = valueOf('--trace') ?? null;
const profilePath = valueOf('--profile') ?? null;
const skipped = new Set();
argv.forEach((a, i) => {
  if (a === '--boot') skipped.add(i);
  if (a === '--load' || BOOT_VALUE_OPTS.includes(a)) { skipped.add(i); skipped.add(i + 1); }
});
const positional = argv.filter((a, i) => !skipped.has(i));
const pressOk = pressRaw === undefined || (Number.isFinite(pressAt) && pressAt >= 0 && pressAt <= 60);
const bootOk = !bootOptUsed || boot;
const valuesOk = BOOT_VALUE_OPTS.every((o) => !argv.includes(o) || (valueOf(o) !== undefined && !valueOf(o).startsWith('--')));
if (positional.length < 1 || positional.length > 2 || (load !== 'none' && load !== 'ui') || (boot && (li >= 0 || positional.length !== 1)) || !pressOk || !bootOk || !valuesOk) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(positional[0]);
const projectFile = positional[1] ? path.resolve(positional[1]) : null;
if (projectFile !== null && !fs.existsSync(projectFile)) {
  process.stderr.write(`no project file ${projectFile}\n`);
  process.exit(2);
}
const BUNDLE = path.basename(bundleDir);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 500;
const CDP_TIMEOUT_MS = 20000;
const RUN_MS = 10000;
const UI_STEP_MS = 50;
const AUDIO_WAIT_MS = 5000;
// the start-up consent box ("Help us improve FL Studio by sending usage data") is drawn in the guest picture (not a window). Its "No thanks" button is at this guest pixel on the 780x480 screen that the default window gives (measured with a screenshot: the box is gone after the press, the status line reads "last input: button L up at screen (513, 308)")
const NO_THANKS = { gw: 780, gh: 480, x: 513, y: 308 };
// --boot (#1334): the room (the stage: the viewport less the 340 px monitor panel and the 36 px header of style.css, screen-fit) is 1836x1056 CSS px at the device pixel ratio of the user's machine (1.7647 = 30/17: the guest screen is then 3240x1863 device px, floor of the room times the ratio); the poll of engine-idle; the three read times after engine-idle (+10, +70, +90 s; bootLast60sAt90s is the one the acceptance reads)
const BOOT_VIEWPORT = { width: 1836 + 340, height: 1056 + 36, dpr: 30 / 17 };
const BOOT_POLL_MS = 50;
const BOOT_READ_MS = { first: 10000, last: 70000, final: 90000 };
const TRACE_MS = 50;
const PROFILE_INTERVAL_US = 250;

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

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const pickAudio = (d) => ({ status: d.status, ctxState: d.ctxState, nodeReady: d.nodeReady, deviceState: d.deviceState, gestures: d.gestures, quantaWhilePlaying: d.quantaWhilePlaying, underrunQuanta: d.underrunQuanta, engineGapFrames: d.engineGapFrames, lastResume: d.lastResume });

async function main() {
  const t0 = Date.now();
  const out = {
    startedAt: new Date(t0).toISOString(), bundle: BUNDLE, mode: boot ? 'boot' : 'run', load, project: projectFile === null ? null : path.basename(projectFile),
    state: null, standIn: null, error: null, gesture: null, consent: null, keyAtGuest: null, before: null,
    ctxState: null, status: null, quantaWhilePlaying: null, underrunQuanta: null, engineGapFrames: null, playingSeconds: null, deviceState: null,
    presentsDelta: null, measuredMs: null, elapsedMs: null, chromeVersion: null, nproc: os.availableParallelism?.() ?? os.cpus().length, loadavg: null,
    screenDevicePx: null, dpr: null, quantaDelta: null, bootLast60sAt10s: null, bootLast60sAt70s: null, bootLast60sAt90s: null, maxRefillStallMs: null, readAtMs: null,
  };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-p3-probe-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
  let probeError = null;
  const fromAudio = (d) => Object.assign(out, {
    ctxState: d.ctxState, status: d.status, quantaWhilePlaying: d.quantaWhilePlaying, underrunQuanta: d.underrunQuanta, engineGapFrames: d.engineGapFrames,
    playingSeconds: d.playingSeconds, deviceState: d.deviceState, underrunQuantaSinceFirstIdle: d.underrunQuantaSinceFirstIdle, underrunEpisodes: d.episodes,
    longestRunQuanta: d.longestRunQuanta, nonSilentQuanta: d.nonSilentQuanta, engineGapFramesLast60s: d.engineGapFramesLast60s,
  });
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });

    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    out.chromeVersion = chrome.version.Browser;
    cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const chooser = [];
    const workers = [];
    cdp.on((m) => {
      if (m.sessionId === sessionId && m.method === 'Page.fileChooserOpened') chooser.push(m.params);
      if (m.sessionId === sessionId && m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId);
    });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    const audioNow = () => ev('window.__app && window.__app.audio ? window.__app.audio.detail() : null');
    const presentsNow = () => ev('window.__app && window.__app.deskStats ? window.__app.deskStats.presents : null');
    await S('Page.enable');
    if (boot) {
      await S('Emulation.setDeviceMetricsOverride', { width: BOOT_VIEWPORT.width, height: BOOT_VIEWPORT.height, deviceScaleFactor: BOOT_VIEWPORT.dpr, mobile: false });
      await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });
    }

    navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let tIdle = 0;
    for (;;) {
      const st = await ev('window.__app ? window.__app.state : null').catch(() => null);
      if (st === 'engine-idle') { tIdle = Date.now(); break; }
      if (st === 'error' || st === 'exited') throw new Error(`state ${st} before the engine was idle`);
      if (Date.now() - navAt >= WAIT_LIMIT_MS) throw new Error(`no engine-idle within ${WAIT_LIMIT_MS} ms (state ${st ?? 'window.__app not set'})`);
      await sleep(boot ? BOOT_POLL_MS : POLL_MS);
    }
    out.state = 'engine-idle';
    out.standIn = await ev("document.body.innerText.includes('STAND-IN')");
    log('engine-idle', { ms: Date.now() - navAt });
    if (boot) {
      await bootWindow(S, ev, audioNow, fromAudio, cdp, workers, out, tIdle);
      return;
    }
    const before = await audioNow();
    if (before === null) throw new Error('the page has no audio output (window.__app.audio is null)');
    out.before = pickAudio(before);
    log('before the gesture', out.before);

    out.gesture = await startAudio(S, ev);
    log('AudioContext', { gesture: out.gesture, ctxState: (await audioNow()).ctxState });
    if (out.gesture === null) {
      const d = await audioNow();
      fromAudio(d);
      probeError = `AudioContext did not start: ctxState ${d.ctxState} after a press and a release on the status line and after a Shift key press`;
      return;
    }

    out.consent = await dismissStartupBox(S, ev);
    log('consent box', out.consent);

    if (projectFile !== null) await openProject(S, ev, chooser, projectFile);
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await tapKey(S, 'Space', ' ', 32);
    out.keyAtGuest = await keyShown(ev, 'vk 0x20', 2000);
    log('Space sent', { reachedGuest: out.keyAtGuest });

    const d0 = await audioNow();
    const p0 = await presentsNow();
    const stop = { done: false };
    const ui = load === 'ui' ? uiLoop(S, ev, stop) : null;
    const m0 = Date.now();
    let next = m0 + 1000;
    while (Date.now() - m0 < RUN_MS) {
      await sleep(Math.max(0, next - Date.now()));
      next += 1000;
      const d = await audioNow();
      log('t', Date.now() - m0, { ctxState: d.ctxState, quanta: d.quantaWhilePlaying, ur: d.underrunQuanta, gap: d.engineGapFrames, dev: d.deviceState });
    }
    stop.done = true;
    if (ui) await ui;
    out.measuredMs = Date.now() - m0;
    const d1 = await audioNow();
    const p1 = await presentsNow();
    log('from', pickAudio(d0), 'to', pickAudio(d1));
    fromAudio(d1);
    Object.assign(out, { presentsDelta: p0 === null || p1 === null ? null : p1 - p0, quantaDelta: d1.quantaWhilePlaying - d0.quantaWhilePlaying, underrunDelta: d1.underrunQuanta - d0.underrunQuanta });
    if (d1.error) probeError = d1.error;
  } catch (e) {
    probeError = String(e?.message ?? e).slice(0, 500);
  } finally {
    out.error = probeError;
    if (navAt !== null) out.elapsedMs = Date.now() - navAt;
    out.loadavg = os.loadavg();
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
    process.stdout.write(JSON.stringify(out) + '\n');
    process.exitCode = out.error ? 1 : 0;
  }
}

// --boot (#1334): the one press that starts the AudioContext comes --press-at seconds after engine-idle (default 0; the status line, as startAudio does; no guest event). The window is t0+10 s to t0+70 s: nothing is sent to the guest in it, and the reads are taken at its two ends. The read at t0+90 s (bootLast60sAt90s: the last 60 s are t0+30 s to t0+90 s, past the start-up peak) comes after the one at +70 s.
async function bootWindow(S, ev, audioNow, fromAudio, cdp, workers, out, tIdle) {
  const engine = await engineWorker(cdp, workers);
  if ((tracePath !== null || profilePath !== null) && engine === null) throw new Error('no engine Worker with fleAudioWorkerState (--trace and --profile need it)');
  const stop = { done: false };
  const trace = tracePath !== null ? traceLoop(audioNow, cdp, engine, tIdle, stop) : null;
  try {
    if (profilePath !== null) await profileStart(cdp, engine);
    out.pressAtMs = Math.round(pressAt * 1000);
    const press = async () => {
      await sleepUntil(tIdle + out.pressAtMs);
      out.gesture = await startAudio(S, ev);
      out.pressedAtMs = Date.now() - tIdle;
      log('AudioContext (boot)', { gesture: out.gesture, pressedAtMs: out.pressedAtMs });
      if (out.gesture === null) throw new Error(`AudioContext did not start after the press and the Shift key press (ctxState ${(await audioNow())?.ctxState ?? 'no audio'})`);
      const s = await ev('(() => { const g = window.__app.screen.guest; return { w: g.width, h: g.height, dpr: window.devicePixelRatio }; })()');
      out.screenDevicePx = `${s.w}x${s.h}`;
      out.dpr = s.dpr;
    };
    let d10 = null;
    let at10 = null;
    const read10 = async () => {
      await sleepUntil(tIdle + BOOT_READ_MS.first);
      d10 = await audioNow();
      if (d10 === null) throw new Error('the page has no audio output (window.__app.audio is null)');
      at10 = Date.now() - tIdle;
    };
    // the read at +10 s comes before the press when the press is later than that (the reads are at their own times, in time order)
    if (out.pressAtMs <= BOOT_READ_MS.first) { await press(); await read10(); } else { await read10(); await press(); }
    await sleepUntil(tIdle + BOOT_READ_MS.last);
    const d70 = await audioNow();
    const at70 = Date.now() - tIdle;
    await sleepUntil(tIdle + BOOT_READ_MS.final);
    const d90 = await audioNow();
    const at90 = Date.now() - tIdle;
    fromAudio(d90);
    out.readAtMs = [at10, at70, at90];
    out.bootLast60sAt10s = d10.underrunQuantaLast60s;
    out.bootLast60sAt70s = d70.underrunQuantaLast60s;
    out.bootLast60sAt90s = d90.underrunQuantaLast60s;
    out.quantaDelta = d70.quantaWhilePlaying - d10.quantaWhilePlaying;
    out.maxRefillStallMs = await enginePublishGap(cdp, workers);
    log('boot window', { at10, at70, at90, last60sAt10s: out.bootLast60sAt10s, last60sAt70s: out.bootLast60sAt70s, last60sAt90s: out.bootLast60sAt90s, quantaDelta: out.quantaDelta, maxRefillStallMs: out.maxRefillStallMs, ctxState: d90.ctxState });
    if (profilePath !== null) await profileStop(cdp, engine, profilePath);
    out.profile = profilePath;
  } finally {
    stop.done = true;
    if (trace) await trace;
    out.trace = tracePath;
  }
}

// the engine's Worker: the one whose global fleAudioWorkerState exists (fle_audio_transport.c); null when none has it
async function engineWorker(cdp, workers) {
  const expr = "typeof globalThis.fleAudioWorkerState === 'function'";
  for (const sid of workers) {
    const r = await timed(cdp.send('Runtime.evaluate', { expression: expr, returnByValue: true }, sid), CDP_TIMEOUT_MS, 'the engine worker').catch(() => null);
    if (r?.result?.value === true) return sid;
  }
  return null;
}

// --trace (#1334): one row every TRACE_MS from engine-idle until the window ends: the page's underrun quanta and playing quanta (detail()), and the engine's WR and RD (frames; RD = rdHi + the low word, as fle_js_publish reads it), its publish count and maxPubGapMs. A failed sample ends the trace (the error goes to the log; the rows so far are written).
async function traceLoop(audioNow, cdp, engine, tIdle, stop) {
  const rows = ['tMs\turQuanta\tplayingQuanta\tctxState\tpublishes\twr\trd\tmaxPubGapMs\twaits\tgapFrames\tpaceWaits\tpaceWaitMs'];
  const expr = '(() => { const A = globalThis.__fleAud; if (!A) return null; const f = globalThis.fleAudioWorkerState(); const p = globalThis.fleAudioPace ? globalThis.fleAudioPace() : {}; return { wr: A.wr, rd: A.rdHi + (Atomics.load(A.h, 0) >>> 0), publishes: f.publishes, maxPubGapMs: f.maxPubGapMs, waits: f.waits, gapFrames: f.gapFrames, paceWaits: p.waits, paceWaitMs: p.waitMs }; })()';
  try {
    for (let k = 0; !stop.done; k++) {
      await sleepUntil(tIdle + k * TRACE_MS);
      if (stop.done) break;
      const t = Date.now() - tIdle;
      const d = (await audioNow()) ?? {};
      const w = (await timed(cdp.send('Runtime.evaluate', { expression: expr, returnByValue: true }, engine), CDP_TIMEOUT_MS, 'the trace of the engine'))?.result?.value ?? {};
      rows.push([t, d.underrunQuanta, d.quantaWhilePlaying, d.ctxState, w.publishes, w.wr, w.rd, w.maxPubGapMs, w.waits, w.gapFrames, w.paceWaits, w.paceWaitMs].join('\t'));
    }
  } catch (e) {
    log('trace stopped', { error: String(e?.message ?? e).slice(0, 200) });
  }
  fs.writeFileSync(tracePath, rows.join('\n') + '\n');
  log('trace', { rows: rows.length - 1, file: tracePath });
}

// --profile (#1334): the CPU profile of the engine's Worker from engine-idle to the end of the window, at PROFILE_INTERVAL_US; written as the CDP profile JSON (the nodes and the samples; the names are wasm-function[N])
async function profileStart(cdp, engine) {
  await timed(cdp.send('Profiler.enable', {}, engine), CDP_TIMEOUT_MS, 'Profiler.enable');
  await timed(cdp.send('Profiler.setSamplingInterval', { interval: PROFILE_INTERVAL_US }, engine), CDP_TIMEOUT_MS, 'Profiler.setSamplingInterval');
  await timed(cdp.send('Profiler.start', {}, engine), CDP_TIMEOUT_MS, 'Profiler.start');
}

async function profileStop(cdp, engine, file) {
  const r = await timed(cdp.send('Profiler.stop', {}, engine), 300000, 'Profiler.stop');
  fs.writeFileSync(file, JSON.stringify(r.profile));
  log('profile', { file, samples: r.profile.samples?.length ?? null });
}

async function sleepUntil(at) {
  const d = at - Date.now();
  if (d > 0) await sleep(d);
}

// the engine's own count (fle_audio_transport.c: the longest interval between two refills of the output ring since its first one). The Worker's global is read through CDP; the other Workers (the renderer) have no fleAudioWorkerState and answer null.
async function enginePublishGap(cdp, workers) {
  const expr = "(() => { const f = globalThis.fleAudioWorkerState; return typeof f === 'function' ? f().maxPubGapMs : null; })()";
  for (const sid of workers) {
    const r = await timed(cdp.send('Runtime.evaluate', { expression: expr, returnByValue: true }, sid), CDP_TIMEOUT_MS, 'the engine worker').catch(() => null);
    const v = r?.result?.value;
    if (typeof v === 'number') return Math.round(v * 10) / 10;
  }
  return null;
}

// the page's own start: a press and a release on the status line (outside the desktop, so the guest gets no pointer event). The browser counts them as a user gesture, and the page's listeners call resume(). A Shift key press is a second gesture, if the first one did not start the context.
async function startAudio(S, ev) {
  const at = await ev("(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()");
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  if (await audioRunning(ev, AUDIO_WAIT_MS)) return 'mouse';
  await tapKey(S, 'ShiftLeft', 'Shift', 16);
  if (await audioRunning(ev, AUDIO_WAIT_MS)) return 'key';
  return null;
}

async function audioRunning(ev, ms) {
  const t = Date.now();
  for (;;) {
    const d = await ev('window.__app.audio.detail()');
    if (d.ctxState === 'running' && d.nodeReady) return true;
    if (Date.now() - t >= ms) return false;
    await sleep(POLL_MS);
  }
}

async function tapKey(S, code, key, vk) {
  const text = code === 'Space' ? ' ' : undefined;
  await S('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, ...(text ? { text } : {}) });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}

// the engine's status line names the last key the guest was given ("last input: key up vk 0x20" for Space); it is written after the engine has the key, so it is read for a short while
async function keyShown(ev, text, ms) {
  const t = Date.now();
  for (;;) {
    const line = (await ev("(document.getElementById('engine-line')?.textContent ?? '')")) ?? '';
    if (line.includes(text)) return true;
    if (Date.now() - t >= ms) return false;
    await sleep(POLL_MS);
  }
}

// the start-up consent box is closed with "No thanks" by this probe only. It is pressed at its measured guest pixel and only on the screen it was measured on; on another screen the box is left as it is and the JSON says so.
async function dismissStartupBox(S, ev) {
  const s = await ev('(() => { const g = window.__app.screen.guest; return { gw: g.width, gh: g.height }; })()');
  if (s.gw !== NO_THANKS.gw || s.gh !== NO_THANKS.gh) return `not pressed: the guest screen is ${s.gw}x${s.gh}, the position is measured for ${NO_THANKS.gw}x${NO_THANKS.gh}`;
  await clickGuest(S, ev, NO_THANKS.x, NO_THANKS.y);
  await sleep(1500);
  return `"No thanks" at guest (${NO_THANKS.x}, ${NO_THANKS.y}) of ${NO_THANKS.gw}x${NO_THANKS.gh}`;
}

// the project file is opened with the engine's File > Open, the route of the other probes (tools/verify-kit/tools/sdriveg2-lat.mjs, openFile). Not run yet: no project file is in the repository.
async function openProject(S, ev, chooser, file) {
  const stem = path.basename(file, path.extname(file));
  await S('Page.setInterceptFileChooserDialog', { enabled: true });
  await clickGuest(S, ev, 18, 20);
  await sleep(1500);
  await clickGuest(S, ev, 48, 85);
  await until(() => ev('window.__app.fileDlg.open === true'), 120000, 'the File > Open dialog');
  await sleep(500);
  const box = await ev("document.getElementById('filedlg-input').getBoundingClientRect().toJSON()");
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: box.x + 10, y: box.y + box.height / 2, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: box.x + 10, y: box.y + box.height / 2, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await until(() => chooser.length > 0, 5000, 'the file chooser');
  await S('DOM.setFileInputFiles', { files: [file], backendNodeId: chooser[0].backendNodeId });
  await until(async () => ((await ev('window.__app.pageTitle().title')) ?? '').startsWith(stem), 180000, `the title of ${stem}`);
  await sleep(4000);
}

async function until(fn, ms, what) {
  const t = Date.now();
  for (;;) {
    if (await fn()) return;
    if (Date.now() - t >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
  }
}

// guest pixel to client pixel of the desktop element (the guest's screen is fitted into the element)
async function clickGuest(S, ev, gx, gy) {
  const g = await ev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const s = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: s.width, gh: s.height }; })()`);
  const x = g.x + (gx + 0.5) * (g.w / g.gw), y = g.y + (gy + 0.5) * (g.h / g.gh);
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(120);
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(70);
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(60);
}

// --load ui: a small circle of pointer moves over the centre of the guest's screen (the main window fills it), one move per 50 ms
async function uiLoop(S, ev, stop) {
  const c = await ev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
  let i = 0;
  while (!stop.done) {
    const a = i++ * 0.3;
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: Math.round(c.x + 30 * Math.cos(a)), y: Math.round(c.y + 30 * Math.sin(a)), button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(UI_STEP_MS);
  }
}

await main();
