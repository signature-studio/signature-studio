// Live audio probe (Issue #1349): the page is opened in a room of 1836x1056 CSS px at the device pixel ratio --dpr (1.7647 by default: the guest screen is 3240x1863 device px), the engine's idle is waited for (t0 = the first observation of engine-idle), and the audio output's numbers are read at t0+10 s (reference), t0+70 s (start of the window) and t0+130 s (end of the window).
// In the window: --play 1 presses Space at its start; --hz H --mouse S move the pointer for S seconds on a circle (radius 400 CSS px, one turn per second) over the guest's desktop, sent from here with CDP Input.dispatchMouseEvent (no button, no reply awaited, batches on each wake-up). The nominal send rate is 1.2 x H (so the page receives at least H in every second).
// The product is not changed: every input is sent from here. The AudioContext is started before the window by a press and a release on the status line (the browser's gesture rule). The start-up consent box is not closed (its "No thanks" position is measured for another screen size).
// --project: the .flp to open (File > Open). Without it, inputs/p3-project.tsv (#1335) names it when that file exists: its first data row (not '#', not empty), first tab-separated column, a path relative to the repository or absolute. Without either, the start-up Default project is used.
// --profile FILE (#1349, a measurement: it adds CDP calls): the CDP CPU profile of the engine's Worker over the window (its interval is the same as measure.mjs's); tools/p3-audio-probe/profile-names.mjs reads it.
// --app <dir> (#1422): the page measured is the app of that directory (default: this repository's app). --sound (#1422): after the transport starts, the metronome button of the guest's top bar is pressed (the place and steps of tools/p2-frames/measure.mjs --load b), so the Default project makes sound; without a rise of 20 non-silent quanta within 5 s the run fails.
// PCM repeat (#1422): the output of the window's first 8 s is taken (the page computes its summary, the samples stay there): pcmRepeat8192 (the share of non-silent frames equal to the ones 8192 frames earlier; null below 1000 comparisons), pcmEligible, pcmNeighbors (the other lags, as controls), pcmCaptureFrames, pcmCaptureValid, pcmNonFiniteSamples. tools/p3-audio-probe/waveform.mjs.
// Usage: node tools/p3-audio-probe/measure-live.mjs <absolute path to the bundle directory raw/<name>> [--dpr D] [--play 0|1] [--hz H] [--mouse S] [--project <absolute path to a .flp>] [--profile FILE] [--app <dir>] [--sound] > live.json
// stdout: one JSON line. stderr: the log. Exit 1 when "error" is set, 2 when the arguments are wrong. The temporary directory is inside the repository (.tmp-live-*) and is removed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { armExpression, takeExpression, pcmReport } from './waveform.mjs';

const USAGE = 'usage: node tools/p3-audio-probe/measure-live.mjs <absolute path to the bundle directory raw/<name>> [--dpr D] [--play 0|1] [--hz H] [--mouse S] [--project <absolute path to a .flp>] [--app <dir>] [--sound]\n';
const argv = process.argv.slice(2);
const valueOf = (name) => { const i = argv.indexOf(name); return i < 0 ? undefined : argv[i + 1]; };
const VALUE_OPTS = ['--dpr', '--play', '--hz', '--mouse', '--project', '--profile', '--app'];
const FLAG_OPTS = ['--sound'];
const skipped = new Set();
argv.forEach((a, i) => {
  if (VALUE_OPTS.includes(a)) { skipped.add(i); skipped.add(i + 1); }
  if (FLAG_OPTS.includes(a)) skipped.add(i);
});
const positional = argv.filter((a, i) => !skipped.has(i));
const dpr = valueOf('--dpr') === undefined ? 1.7647 : Number(valueOf('--dpr'));
const play = valueOf('--play') === undefined ? 1 : Number(valueOf('--play'));
const hz = valueOf('--hz') === undefined ? 1000 : Number(valueOf('--hz'));
const mouseSec = valueOf('--mouse') === undefined ? 60 : Number(valueOf('--mouse'));
const projectRaw = valueOf('--project');
const profilePath = valueOf('--profile') === undefined ? null : path.resolve(valueOf('--profile'));
const soundOn = argv.includes('--sound');
const appRaw = valueOf('--app');
const argsOk = positional.length === 1 && Number.isFinite(dpr) && dpr > 0 && (play === 0 || play === 1) && Number.isFinite(hz) && hz >= 0 && Number.isFinite(mouseSec) && mouseSec >= 0
  && valuesPresent() && (!soundOn || play === 1);
function valuesPresent() { return VALUE_OPTS.every((o) => !argv.includes(o) || (valueOf(o) !== undefined && !valueOf(o).startsWith('--'))); }
if (!argsOk || (projectRaw !== undefined && !path.isAbsolute(projectRaw))) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(positional[0]);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
// the project named by inputs/p3-project.tsv (#1335): its first data row's first column
function listedProject() {
  const listFile = path.join(repo, 'inputs', 'p3-project.tsv');
  if (!fs.existsSync(listFile)) return null;
  const row = fs.readFileSync(listFile, 'utf8').split('\n').map((l) => l.trimEnd()).find((l) => l !== '' && !l.startsWith('#'));
  if (row === undefined) return null;
  const first = row.split('\t')[0];
  return path.resolve(repo, first);
}
const projectFile = projectRaw !== undefined ? path.resolve(projectRaw) : listedProject();
if (projectFile !== null && !fs.existsSync(projectFile)) {
  process.stderr.write(`no project file ${projectFile}\n`);
  process.exit(2);
}
const BUNDLE = path.basename(bundleDir);
const appDir = appRaw === undefined ? path.join(repo, 'app') : path.resolve(appRaw);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 50;
const CDP_TIMEOUT_MS = 20000;
// --sound (the steps of tools/p2-frames/measure.mjs --load b): the metronome button of the guest's top bar, in CSS px from the desktop's corner; the rise of the non-silent quanta that counts as sound, and how long it may take
const METRONOME = { x: 750, y: 19 };
const SOUND_RISE_QUANTA = 20;
const SOUND_WAIT_MS = 5000;
// the wait for the window's PCM capture (8 s of output from the window's start) after the window ends
const PCM_WAIT_MS = 5000;
// the reads, from engine-idle (ms): the reference (the start-up's mountain is over only later), the window (60 s) and its end
const READ_MS = { reference: 10000, windowStart: 70000, windowEnd: 130000 };
// the room: the stage of style.css (the viewport less the 340 px monitor panel and the 36 px header) is 1836x1056 CSS px
const ROOM = { w: 1836, h: 1056 };
const VIEWPORT = { width: ROOM.w + 340, height: ROOM.h + 36 };
// the pointer: a circle of radius 400 CSS px in the middle of the room, one turn per second; at the nominal 1.2 x 1000 Hz the step per event is about 2.1 CSS px
const POINTER_RADIUS_CSS = 400;
const POINTER_TURNS_PER_S = 1;
const SEND_FACTOR = 1.2;
const MAX_IN_FLIGHT = 256;
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
const sleepUntil = async (at) => { const d = at - Date.now(); if (d > 0) await sleep(d); };
const round = (x, k = 1) => (typeof x === 'number' && Number.isFinite(x) ? Math.round(x * 10 ** k) / 10 ** k : null);

// the page's own numbers at one instant: the audio output (detail(), the same numbers as the header), the input counters and the screen
const READ = `(() => {
  const a = window.__app;
  const d = a.audio.detail();
  return {
    audio: d,
    inputEvents: a.inputEvents,
    driver: a.driverStats,
    presents: a.deskStats ? a.deskStats.presents : null,
    guest: { w: a.screen.guest.width, h: a.screen.guest.height },
    dpr: window.devicePixelRatio,
    pm: window.__pm ? { disp: window.__pm.disp, coal: window.__pm.coal } : null,
    now: Date.now(),
  };
})()`;

async function main() {
  const t0 = Date.now();
  const out = {
    startedAt: new Date(t0).toISOString(), bundle: BUNDLE, error: null, state: null, standIn: null, dpr: null, play, hz, mouseSec, screenDevicePx: null,
    playing: null, nonSilentQuanta: null, nonSilentDelta: null, last60sUnderruns: null, last60sUnderrunsAtIdle10s: null, underrunDelta: null, inputRateOk: null, profile: profilePath,
    pointerEventsPerSecMin: null, pointerEventsPerSecMean: null, inputSentPerSec: null, inputProcessedPerSec: null, engineCyclePerSec: null, guestMouseMovePerSec: null,
    quantaDelta: null, engineGapFrames: null, engineGapFramesDelta: null, presentsDelta: null,
    pointerSent: null, pointerErrors: null, pointerMaxInFlight: null, pointerEventsPerSec: null, readAtMs: null, project: projectFile === null ? null : path.basename(projectFile),
    gesture: null, keyAtGuest: null, chromeVersion: null, nproc: os.availableParallelism?.() ?? os.cpus().length, loadavg: null, elapsedMs: null,
    app: appDir, sound: soundOn, soundStartMs: null, pcmRepeat8192: null, pcmEligible: null, pcmNeighbors: null, pcmCaptureFrames: null, pcmCaptureValid: null, pcmNonFiniteSamples: null,
  };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-live-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
  let probeError = null;
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
    const S = (method, params) => cdp.send(method, params, sessionId);
    const timedS = (method, params) => timed(S(method, params), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await timedS('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    await timedS('Page.enable', {});
    await timedS('Emulation.setDeviceMetricsOverride', { width: VIEWPORT.width, height: VIEWPORT.height, deviceScaleFactor: dpr, mobile: false });
    if (profilePath !== null) await timedS('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });

    navAt = Date.now();
    const nav = await timedS('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let tIdle = 0;
    for (;;) {
      const st = await ev('window.__app ? window.__app.state : null').catch(() => null);
      if (st === 'engine-idle') { tIdle = Date.now(); break; }
      if (st === 'error' || st === 'exited') throw new Error(`state ${st} before the engine was idle`);
      if (Date.now() - navAt >= WAIT_LIMIT_MS) throw new Error(`no engine-idle within ${WAIT_LIMIT_MS} ms (state ${st ?? 'window.__app not set'})`);
      await sleep(POLL_MS);
    }
    out.state = 'engine-idle';
    out.standIn = await ev("document.body.innerText.includes('STAND-IN')");
    log('engine-idle', { ms: tIdle - navAt });
    const engine = profilePath === null ? null : await engineWorker(cdp, workers);
    if (profilePath !== null && engine === null) throw new Error('no engine Worker with fleAudioWorkerState (--profile needs it)');

    out.gesture = await startAudio(S, ev);
    log('AudioContext', { gesture: out.gesture });
    if (out.gesture === null) throw new Error('AudioContext did not start after a press and a release on the status line and a Shift key press');

    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    if (projectFile !== null) await openProject(S, ev, chooser, projectFile);
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    if (soundOn) await closeStartBoxes(S, ev);

    const at = (ms) => tIdle + ms;
    await sleepUntil(at(READ_MS.reference));
    const d10 = await ev(READ);
    out.last60sUnderrunsAtIdle10s = d10.audio.underrunQuantaLast60s;
    log('+10 s', { last60s: d10.audio.underrunQuantaLast60s, ctxState: d10.audio.ctxState });

    await sleepUntil(at(READ_MS.windowStart));
    const d70 = await ev(READ);
    out.dpr = d70.dpr;
    out.screenDevicePx = `${d70.guest.w}x${d70.guest.h}`;
    const centre = await ev("(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()");
    await ev("window.__pm = { disp: 0, coal: 0 }; document.addEventListener('pointermove', (e) => { const n = e.getCoalescedEvents ? e.getCoalescedEvents().length : 0; window.__pm.disp++; window.__pm.coal += n > 0 ? n : 1; }, { capture: true, passive: true }); true");
    const pcmIdx = await ev(armExpression());
    if (play === 1) {
      await tapKey(S, 'Space', ' ', 32);
      out.keyAtGuest = await keyShown(ev, 'vk 0x20', 2000);
      log('Space sent', { reachedGuest: out.keyAtGuest });
    }
    if (soundOn) {
      await sleep(500);
      out.soundStartMs = await pressMetronome(S, ev);
      log('sound', { startMs: out.soundStartMs });
    }
    if (profilePath !== null) await profileStart(cdp, engine);
    const pointer = mouseSec > 0 && hz > 0 ? movePointer(S, sessionId, centre, hz, mouseSec) : null;
    const rates = mouseSec > 0 ? sampleRates(ev, mouseSec) : null;
    const pointerDone = pointer === null ? null : await pointer;
    const ratesDone = rates === null ? null : await rates;
    await sleepUntil(at(READ_MS.windowEnd));
    const d130 = await ev(READ);
    const endAt = Date.now();
    if (profilePath !== null) await profileStop(cdp, engine, profilePath);
    out.readAtMs = [d10.now - tIdle, d70.now - tIdle, d130.now - tIdle];
    const winSec = (d130.now - d70.now) / 1000;
    const a0 = d70.audio, a1 = d130.audio;
    Object.assign(out, {
      playing: a1.deviceState === 'playing', nonSilentQuanta: a1.nonSilentQuanta, nonSilentDelta: a1.nonSilentQuanta - a0.nonSilentQuanta, last60sUnderruns: a1.underrunQuantaLast60s,
      underrunDelta: a1.underrunQuanta - a0.underrunQuanta, quantaDelta: a1.quantaWhilePlaying - a0.quantaWhilePlaying,
      engineGapFrames: a1.engineGapFrames, engineGapFramesDelta: a1.engineGapFrames === null || a0.engineGapFrames === null ? null : a1.engineGapFrames - a0.engineGapFrames,
      presentsDelta: d70.presents === null || d130.presents === null ? null : d130.presents - d70.presents,
      inputSentPerSec: round((d130.inputEvents - d70.inputEvents) / winSec), inputProcessedPerSec: round((d130.driver.injected - d70.driver.injected) / winSec),
      engineCyclePerSec: round((d130.driver.idleCount - d70.driver.idleCount) / winSec),
      inputRateOk: hz > 0 && mouseSec > 0 && ratesDone !== null ? ratesDone.minPerSec >= hz : null,
      pointerEventsPerSecMin: ratesDone === null ? null : ratesDone.minPerSec,
      pointerEventsPerSecMean: ratesDone === null || ratesDone.totalPerWindow === null ? null : round(ratesDone.totalPerWindow / mouseSec),
      pointerEventsPerSec: ratesDone === null ? null : ratesDone.perSec,
      pointerSent: pointerDone === null ? null : pointerDone.sent, pointerErrors: pointerDone === null ? null : pointerDone.errors, pointerMaxInFlight: pointerDone === null ? null : pointerDone.maxInFlight,
      guestMouseMovePerSec: null,
    });
    if (pcmIdx !== null) Object.assign(out, await takePcm(ev, pcmIdx));
    out.endedAtMs = endAt - tIdle;
    log('window', { last60sAt10s: out.last60sUnderrunsAtIdle10s, last60sAt130s: out.last60sUnderruns, underrunDelta: out.underrunDelta, nonSilentQuanta: out.nonSilentQuanta, nonSilentDelta: out.nonSilentDelta, playing: out.playing, sent: out.inputSentPerSec, processed: out.inputProcessedPerSec, cycles: out.engineCyclePerSec, pointer: pointerDone, rateOk: out.inputRateOk });
    if (a1.error) probeError = a1.error;
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

// the press that starts the AudioContext: a press and a release on the status line; a Shift key press if the first did not start it (measure.mjs)
async function startAudio(S, ev) {
  const at = await ev("(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()");
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  if (await audioRunning(ev, 5000)) return 'mouse';
  await tapKey(S, 'ShiftLeft', 'Shift', 16);
  if (await audioRunning(ev, 5000)) return 'key';
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

async function tapKey(S, code, key, vk, modifiers = 0) {
  const text = code === 'Space' ? ' ' : undefined;
  await S('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers, ...(text ? { text } : {}) });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
}

// the engine's status line names the last key the guest was given ("last input: key up vk 0x20" for Space)
async function keyShown(ev, text, ms) {
  const t = Date.now();
  for (;;) {
    const line = (await ev("(document.getElementById('engine-line')?.textContent ?? '')")) ?? '';
    if (line.includes(text)) return true;
    if (Date.now() - t >= ms) return false;
    await sleep(POLL_MS);
  }
}

// The pointer path: the events are sent in order, event i at i / sendHz seconds after the start (sendHz = SEND_FACTOR x hz). Each wake-up sends every event that is due (a batch, as tools/p2-frames/measure.mjs's pointerLoop), so the rate holds whatever the timer's resolution is; the replies are not awaited (at most MAX_IN_FLIGHT outstanding).
async function movePointer(S, sessionId, centre, hz, seconds) {
  const sendHz = SEND_FACTOR * hz;
  const total = Math.round(sendHz * seconds);
  const t0 = performance.now();
  const deadline = t0 + seconds * 3000;
  const inflight = new Set();
  let sent = 0, errors = 0, maxInFlight = 0;
  while (sent < total && performance.now() < deadline) {
    const due = Math.min(total, Math.floor(((performance.now() - t0) * sendHz) / 1000) + 1);
    while (sent < due && inflight.size < MAX_IN_FLIGHT) {
      const a = 2 * Math.PI * POINTER_TURNS_PER_S * (sent / sendHz);
      const x = centre.x + POINTER_RADIUS_CSS * Math.cos(a), y = centre.y + POINTER_RADIUS_CSS * Math.sin(a);
      const p = S('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' }).then(() => {}, () => { errors++; });
      inflight.add(p);
      p.then(() => inflight.delete(p));
      sent++;
      maxInFlight = Math.max(maxInFlight, inflight.size);
    }
    if (inflight.size >= MAX_IN_FLIGHT) await Promise.race(inflight);
    else await sleep(1);
  }
  await Promise.race([Promise.allSettled(inflight), sleep(5000)]);
  log('pointer', { sent, errors, maxInFlight, seconds: Math.round((performance.now() - t0)) / 1000 });
  return { sent, errors, maxInFlight };
}

// the page's count of pointer moves (the coalesced events, each one counted), read once per second over the window
async function sampleRates(ev, seconds) {
  const t0 = Date.now();
  const at = [];
  for (let k = 0; k <= seconds; k++) {
    await sleepUntil(t0 + k * 1000);
    const pm = await ev('window.__pm ? window.__pm.coal : null');
    at.push(pm);
  }
  const perSec = [];
  for (let k = 0; k < seconds; k++) perSec.push(at[k] === null || at[k + 1] === null ? null : at[k + 1] - at[k]);
  const known = perSec.filter((x) => x !== null);
  const totalPerWindow = at[0] === null || at[seconds] === null ? null : at[seconds] - at[0];
  return { perSec, minPerSec: known.length ? Math.min(...known) : null, totalPerWindow };
}

// the project file is opened with the engine's File > Open, by its shortcut Ctrl+O (the menu's pixels are for another screen size, so they are not used). The dialog is the page's own (fileDlg) and takes the file through the browser's file chooser.
async function openProject(S, ev, chooser, file) {
  const stem = path.basename(file, path.extname(file));
  await S('Page.setInterceptFileChooserDialog', { enabled: true });
  await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17 });
  await tapKey(S, 'KeyO', 'o', 79, 2);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17 });
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

// --sound (#1422): the metronome button is pressed at its guest pixel (CSS px x the page's device pixel ratio, as tools/p2-frames/measure.mjs --load b does). Returns the ms until the non-silent quanta had risen by SOUND_RISE_QUANTA; throws when they did not within SOUND_WAIT_MS (with the state of the guest).
async function pressMetronome(S, ev) {
  const pageDpr = await ev('window.devicePixelRatio');
  const n0 = (await ev('window.__app.audio.detail()')).nonSilentQuanta;
  await clickGuest(S, ev, Math.round(METRONOME.x * pageDpr), Math.round(METRONOME.y * pageDpr));
  const t0 = Date.now();
  for (;;) {
    const n1 = (await ev('window.__app.audio.detail()')).nonSilentQuanta;
    if (n1 > n0 + SOUND_RISE_QUANTA) return Date.now() - t0;
    if (Date.now() - t0 >= SOUND_WAIT_MS) throw new Error(`the playback is silent after the metronome button (nonSilentQuanta ${n0} -> ${n1} in ${SOUND_WAIT_MS} ms; ${await guestState(ev)})`.slice(0, 500));
    await sleep(POLL_MS);
  }
}

// the guest's front title, its windows and the engine's status line (for a failure's message)
async function guestState(ev) {
  const title = await ev("window.__app ? window.__app.pageTitle().title : null");
  const line = await ev("(document.getElementById('engine-line')?.textContent ?? '')");
  const wins = await windowsOf(ev).catch(() => []);
  return `front "${title}", windows ${wins.map((w) => `${w.cls}|${w.title}`).join(', ') || 'none'}, status "${String(line).slice(0, 120)}"`;
}

// --sound (#1422): the start-up boxes that take the keys and the pointer are closed first, as tools/p3-audio-probe/measure.mjs does: the Welcome window by its X (tools/p2-frames/measure.mjs's place, in guest pixels of the window list), the "Problems loading the project" box with Enter (its default button). A modal box would take the Space key and the metronome click.
const WELCOME_CLASS = 'TWelcomeWizard';
const PROBLEMS_TITLE = /Problems loading/;
const WINDOWS_TEXT = '(async () => (await window.__app.deskDumpBase64(false)).windows)()';
async function closeStartBoxes(S, ev) {
  const wiz = (await windowsOf(ev)).find((w) => w.cls === WELCOME_CLASS);
  if (wiz) {
    await clickGuest(S, ev, wiz.x + wiz.w - 18, wiz.y + 18);
    await until(async () => !(await windowsOf(ev)).some((w) => w.cls === WELCOME_CLASS), 30000, 'the Welcome window closed');
  }
  for (let i = 0; i < 4 && (await problemsOpen(ev)); i++) {
    const key = { code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 0x0d, nativeVirtualKeyCode: 0x0d };
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
    await sleep(1000);
  }
}
async function problemsOpen(ev) {
  const front = (await ev("window.__app ? window.__app.pageTitle().title : ''")) ?? '';
  if (PROBLEMS_TITLE.test(front)) return true;
  return (await windowsOf(ev)).some((w) => PROBLEMS_TITLE.test(w.title));
}
// the guest's windows (the page answers "ask again" while it draws the picture: asked again for a while)
async function windowsOf(ev) {
  const t0 = Date.now();
  for (;;) {
    try {
      return parseWindows(await ev(WINDOWS_TEXT));
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
      await sleep(100);
    }
  }
}
// the window list of the page's format (lid N at (x, y) WxH ... class "..." title "...")
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)"/.exec(m[6]);
    const title = /title "([^"]*)"/.exec(m[6]);
    return { x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: title ? title[1] : '' };
  });
}

// a left click at a guest pixel (the guest screen is fitted into the desktop element)
async function clickGuest(S, ev, gx, gy) {
  const g = await ev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const s = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: s.width, gh: s.height }; })()`);
  const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(60);
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(50);
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
}

// the PCM check (#1422) of the capture armed at the window's start: waits for it (PCM_WAIT_MS), the summary is computed in the page; null values when it did not come
async function takePcm(ev, index) {
  const t0 = Date.now();
  for (;;) {
    const summary = await ev(takeExpression(index));
    if (summary !== null || Date.now() - t0 >= PCM_WAIT_MS) {
      log('pcm', { frames: summary?.frames ?? null, pcmRepeat8192: summary ? pcmReport(summary).pcmRepeat8192 : null });
      return pcmReport(summary);
    }
    await sleep(POLL_MS);
  }
}

async function until(fn, ms, what) {
  const t = Date.now();
  for (;;) {
    if (await fn()) return;
    if (Date.now() - t >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
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

// --profile: the engine's Worker's CPU profile over the window (the same settings as measure.mjs --profile)
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

await main();
