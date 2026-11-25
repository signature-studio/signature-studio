// P3 audio probe (Issue #1303): starts the page's AudioContext with CDP Input events (a press and a release on the page's status line, which the browser counts as a user gesture), starts the transport with a Space key, and reads the audio output's numbers (window.__app.audio.detail(): decision 0175 / 0211) over about 10 s. The product is not changed: every input is sent from here, and the start-up consent box is closed with "No thanks" from here too.
// Usage: node tools/p3-audio-probe/measure.mjs <absolute path to the bundle directory raw/<name>> [<absolute path to a .flp project>] [--load none|ui] [--make-sound [--shot FILE]] > probe.json
// --load ui: during the 10 s run the pointer moves over the guest's main window every 50 ms (CDP Input.dispatchMouseEvent), so the engine has UI work to do (#1304). The default is none.
// --boot (#1334): the start-up window with nothing sent to the guest. The room is 1836x1056 CSS px at DPR 1.7647 (the guest screen is 3240x1863 device px). t0 is the first observation of engine-idle; the one press on the status line (it starts the AudioContext, no guest event) comes before the window; the numbers are read at t0+10 s and t0+70 s. maxRefillStallMs is the engine's own longest interval between two refills of the output ring (fle_audio_transport.c, the Worker's fleAudioWorkerState).
// --boot options (#1334, only with --boot): --press-at S: the press that starts the AudioContext comes S seconds (0 to 60) after engine-idle (default 0). --trace FILE: every 50 ms in the window, the page's underrun quanta and playing quanta and the engine's WR, RD (frames), publishes and maxPubGapMs, as TSV (tools/p3-audio-probe/trace-stats.mjs reads it). --profile FILE: the CDP CPU profile of the engine's Worker over the window (tools/p3-audio-probe/profile-names.mjs maps its wasm-function[N] to names); the page's main thread is profiled over the same window to FILE with ".main" before its extension (#1352). Both are measurements: they add CDP calls, so their runs are not the acceptance runs.
// stdout: one JSON line. stderr: the log. Exit 1 when "error" is set, 2 when the arguments are wrong. Except (Issue #1400, #1403): once the project is being opened, "error" says why (the guest does not open it, or the engine stops during the open) and the exit is 0 (the open is a result to report, not a failure of the probe). The temporary directory is inside the repository (.tmp-p3-probe-*) and is removed.
// end (Issue #1365): the engine's state, the page's title and its last 400 log lines, at the end of a run or after an error.
// With a project (Issue #1366): projectDialogs (the "Problems loading the project" boxes after the open; [] when none), projectWindows (the windows at the end), projectMissing (the distinct requests of the open for a .wav under Data\Patches\ that found nothing, as the engine logged them), projectMissingLines (their number of lines).
// --app <dir> (#1422): the page measured is the app of that directory (default: this repository's app). --sound (#1422, run mode only): after Space the metronome button is pressed (the Default project then makes sound); soundStartMs is the time to a rise of 20 non-silent quanta (5 s at most, else the run fails).
// PCM repeat (#1422, run mode): the output of the 10 s window's first 8 s is taken and summarised in the page (tools/p3-audio-probe/waveform.mjs): pcmRepeat8192, pcmEligible, pcmNeighbors, pcmCaptureFrames, pcmCaptureValid, pcmNonFiniteSamples.
// With a project (Issue #1412): envLog (the engine's environment-block lines, "env: <API> "<name>" <what> <n>", the first 200 of each API, from the start of the run to the end of the open, or to the failure of the open).
// With a project (Issue #1373): missingDistinct (the distinct missing-path requests of the open, any extension; a request is keyed by its path without the factory root, case ignored), dll (those ending in .dll), inventory (the rest that inputs/fl-installer-files.tsv lists, with the root taken off; inventoryList: their guest spelling, bytes and sha256 from that list), notInInstaller (the rest), notInInstallerFirst10 (the first 10 of the rest, as the engine wrote them), rootStripped ({absolute, variable}: the requests each root was taken off). The two roots are read from the log, not written here: the absolute install folder and the %...% variable that the most requests lead to a row of the list under.
// With a project (Issue #1403): missingInInventory (the distinct absolute spellings that a GetFileAttributesW, CreateFileW or FindFirstFileW request found nothing for, which the list has once the absolute root is taken off, and which are not .dll; missingInInventoryFirst10: {api, path} of the first 10), rawVariableProbes ({count, first10}: the distinct %...%\ spellings as the guest wrote them; FL's own probes, not counted in the others). notInInstaller then excludes the %...%\ spellings and the .dll ones. After an error during the open, the same fields are read from the whole log of the open (not only the end tail).
// Issue #1401: notInInstallerList (every path that notInInstaller counts, not only the first 10; scan-demos.mjs reads it). The wait for the project's title ends at once when the engine is 'exited' or 'error' (the state), not after its 180 s limit.
// --make-sound (Issue #1401): the sound is made by the FL operations, not read from a project. The room is 1836 x 1056 CSS px at DPR 1 (as tools/smoke/welcome-flow.mjs sets it). The Welcome's usage-data box is closed with "No thanks" and its Default row is double-clicked (flow A of welcome-flow.mjs); the project's "Problems loading the project" box is closed with Enter and the "Remix a song" box with Escape. Then the Channel rack's (TStepSeqForm) Kick row is clicked step by step (the points are fractions of the rack's rectangle); a step is lit when the rectangle's picture changes between the clicks. Then Space starts the transport and the probe reads the audio for 10 s as in a run. makeSound: the steps' result (litSteps, the rack, the boxes, the Basic 808 lines of the engine's log). --shot FILE (a test aid): a PNG of the page after the steps is written to FILE. Not combined with a project, --load or --boot.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { armExpression, takeExpression, pcmReport } from './waveform.mjs';

const USAGE = 'usage: node tools/p3-audio-probe/measure.mjs <absolute path to the bundle directory raw/<name>> [<absolute path to a .flp project>] [--load none|ui] [--app <dir>] [--sound] [--boot [--press-at S] [--trace FILE] [--profile FILE] [--pubs FILE]] [--make-sound [--shot FILE]]\n';
const argv = process.argv.slice(2);
const boot = argv.includes('--boot');
const makeSound = argv.includes('--make-sound');
const li = argv.indexOf('--load');
const load = li < 0 ? 'none' : argv[li + 1];
const BOOT_VALUE_OPTS = ['--press-at', '--trace', '--profile', '--pubs'];
const valueOf = (name) => { const i = argv.indexOf(name); return i < 0 ? undefined : argv[i + 1]; };
const bootOptUsed = BOOT_VALUE_OPTS.some((o) => argv.includes(o));
const pressRaw = valueOf('--press-at');
const pressAt = pressRaw === undefined ? 0 : Number(pressRaw);
const tracePath = valueOf('--trace') ?? null;
const profilePath = valueOf('--profile') ?? null;
const pubsPath = valueOf('--pubs') ?? null;
// #1422: --app <dir> (the page measured; default this repository's app), --sound (the metronome button is pressed after the transport starts: the steps of measure-live.mjs --sound)
const appRaw = valueOf('--app');
const soundOn = argv.includes('--sound');
const shotPath = valueOf('--shot') ?? null;
const skipped = new Set();
argv.forEach((a, i) => {
  if (a === '--boot' || a === '--sound' || a === '--make-sound') skipped.add(i);
  if (a === '--load' || a === '--app' || a === '--shot' || BOOT_VALUE_OPTS.includes(a)) { skipped.add(i); skipped.add(i + 1); }
});
const positional = argv.filter((a, i) => !skipped.has(i));
const pressOk = pressRaw === undefined || (Number.isFinite(pressAt) && pressAt >= 0 && pressAt <= 60);
const bootOk = !bootOptUsed || boot;
const valuesOk = BOOT_VALUE_OPTS.every((o) => !argv.includes(o) || (valueOf(o) !== undefined && !valueOf(o).startsWith('--')));
const appOk = !argv.includes('--app') || (appRaw !== undefined && !appRaw.startsWith('--'));
const soundOk = !makeSound || (!boot && li < 0 && !soundOn && positional.length === 1);
const shotOk = !argv.includes('--shot') || (makeSound && shotPath !== null && !shotPath.startsWith('--'));
if (positional.length < 1 || positional.length > 2 || (load !== 'none' && load !== 'ui') || (boot && (li >= 0 || positional.length !== 1 || soundOn)) || !pressOk || !bootOk || !valuesOk || !appOk || !soundOk || !shotOk) {
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
const appDir = appRaw === undefined ? path.join(repo, 'app') : path.resolve(appRaw);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 500;
const CDP_TIMEOUT_MS = 20000;
const RUN_MS = 10000;
const UI_STEP_MS = 50;
const AUDIO_WAIT_MS = 5000;
// #1422, --sound: the metronome button of the guest's top bar, in CSS px from the desktop's corner (tools/p2-frames/measure.mjs --load b); the rise of non-silent quanta that counts as sound and how long it may take; the wait for the window's PCM capture after the window
const METRONOME = { x: 750, y: 19 };
const SOUND_RISE_QUANTA = 20;
const SOUND_WAIT_MS = 5000;
const PCM_WAIT_MS = 5000;
// the start-up consent box ("Help us improve FL Studio by sending usage data") is drawn in the guest picture (not a window). Its "No thanks" button is at this guest pixel on the 780x480 screen that the default window gives (measured with a screenshot: the box is gone after the press, the status line reads "last input: button L up at screen (513, 308)")
const NO_THANKS = { gw: 780, gh: 480, x: 513, y: 308 };
// --boot (#1334): the room (the stage: the viewport less the 340 px monitor panel and the 36 px header of style.css, screen-fit) is 1836x1056 CSS px at the device pixel ratio of the user's machine (1.7647 = 30/17: the guest screen is then 3240x1863 device px, floor of the room times the ratio); the poll of engine-idle; the three read times after engine-idle (+10, +70, +90 s; bootLast60sAt90s is the one the acceptance reads)
const BOOT_VIEWPORT = { width: 1836 + 340, height: 1056 + 36, dpr: 30 / 17 };
const BOOT_POLL_MS = 50;
const BOOT_READ_MS = { first: 10000, last: 70000, final: 90000 };
const TRACE_MS = 50;
const PROFILE_INTERVAL_US = 250;
// Issue #1366: the window list of the guest (the page's deskDumpBase64, as tools/delivery-split/measure.mjs reads it); the box's title; a sample request that found nothing (a .wav in the engine's missing-path line)
const WINDOWS_TEXT = '(async () => (await window.__app.deskDumpBase64(false)).windows)()';
const PROBLEMS_TITLE = /Problems loading/;
const SAMPLE_MISSING = /^(?:\[err\] )?missing-path: .*\\Data\\Patches\\.*\.wav"$/i;
const PROJECT_DIALOG_MS = 15000;
// Issue #1373: the missing-path line (the runtime's fle_vfs_note_missing); the installer's list (the guest column, one path per line after the comment and the header); the number of first paths the breakdown lists; the two shapes a factory root can take (an absolute path's leading folder, on any drive; a %...%\ variable)
const MISSING_LINE = /^(?:\[err\] )?missing-path: (\S+) (\d+) "(.*)"$/;
// Issue #1412: the environment block's observation lines (the runtime's fle_env.c, first 200 per API): envLog is every one in the engine's log
const ENV_LINE = /^(?:\[err\] )?env: /;
const INSTALLER_LIST = path.join(repo, 'inputs', 'fl-installer-files.tsv');
const NOT_IN_INSTALLER_FIRST = 10;
const ABSOLUTE_PATH = /^[A-Za-z]:\\/;
const VARIABLE_PATH = /^%[^%\\]+%\\/;
// Issue #1403: the guest file requests that count as missingInInventory (a module load, LoadLibraryW, does not)
const FILE_APIS = new Set(['GetFileAttributesW', 'CreateFileW', 'FindFirstFileW']);
const MISSING_FIRST = 10;

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
    startedAt: new Date(t0).toISOString(), bundle: BUNDLE, mode: boot ? 'boot' : makeSound ? 'make-sound' : 'run', load, project: projectFile === null ? null : path.basename(projectFile),
    state: null, standIn: null, error: null, gesture: null, consent: null, makeSound: null, keyAtGuest: null, before: null, projectDialogs: null, projectWindows: null, projectMissing: null, projectMissingLines: null,
    missingDistinct: null, dll: null, inventory: null, inventoryList: null, missingInInventory: null, missingInInventoryFirst10: null, rawVariableProbes: null, notInInstaller: null, notInInstallerFirst10: null, notInInstallerList: null, rootStripped: null, imageReads: null, envLog: null, variableMissing: null,
    ctxState: null, status: null, quantaWhilePlaying: null, underrunQuanta: null, engineGapFrames: null, playingSeconds: null, deviceState: null,
    presentsDelta: null, measuredMs: null, elapsedMs: null, chromeVersion: null, nproc: os.availableParallelism?.() ?? os.cpus().length, loadavg: null,
    screenDevicePx: null, dpr: null, profileMain: null, quantaDelta: null, bootLast60sAt10s: null, bootLast60sAt70s: null, bootLast60sAt90s: null, maxRefillStallMs: null, maxRefillStallInWindowMs: null, pubLog: null, publishesInWindow: null, readAtMs: null,
    app: appDir, sound: soundOn, soundStartMs: null, pcmRepeat8192: null, pcmEligible: null, pcmNeighbors: null, pcmCaptureFrames: null, pcmCaptureValid: null, pcmNonFiniteSamples: null,
  };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-p3-probe-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
  let probeError = null, evAfterError = null, openLogFrom = null;
  const fromAudio = (d) => Object.assign(out, {
    ctxState: d.ctxState, status: d.status, quantaWhilePlaying: d.quantaWhilePlaying, underrunQuanta: d.underrunQuanta, engineGapFrames: d.engineGapFrames,
    playingSeconds: d.playingSeconds, deviceState: d.deviceState, underrunQuantaSinceFirstIdle: d.underrunQuantaSinceFirstIdle, underrunEpisodes: d.episodes,
    longestRunQuanta: d.longestRunQuanta, nonSilentQuanta: d.nonSilentQuanta, engineGapFramesLast60s: d.engineGapFramesLast60s,
  });
  // the engine's state, the page's title and the last 400 log lines (its own stop lines, e.g. "fle: LockResource: ...") at the end of a run or after an error (Issue #1365)
  const engineEnd = () => (evAfterError === null ? null : evAfterError("(() => { const a = window.__app; if (!a) return null; const log = a.engineLog() ?? []; return { state: a.state, title: a.pageTitle().title, tail: log.slice(-400) }; })()").catch((x) => String(x?.message ?? x).slice(0, 200)));
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
    const workers = [];
    cdp.on((m) => {
      if (m.sessionId === sessionId && m.method === 'Target.attachedToTarget' && m.params.targetInfo.type === 'worker') workers.push(m.params.sessionId);
    });
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
      return r.result.value;
    };
    evAfterError = ev;
    const audioNow = () => ev('window.__app && window.__app.audio ? window.__app.audio.detail() : null');
    const presentsNow = () => ev('window.__app && window.__app.deskStats ? window.__app.deskStats.presents : null');
    await S('Page.enable');
    // --sound (#1422) uses the boot room too: the metronome button's place is measured in that room (tools/p2-frames/measure.mjs)
    if (boot || soundOn) await S('Emulation.setDeviceMetricsOverride', { width: BOOT_VIEWPORT.width, height: BOOT_VIEWPORT.height, deviceScaleFactor: BOOT_VIEWPORT.dpr, mobile: false });
    if (boot) await S('Target.setAutoAttach', { autoAttach: true, waitForDebuggerOnStart: false, flatten: true });

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
      await bootWindow(S, ev, audioNow, fromAudio, cdp, workers, out, tIdle, sessionId);
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

    if (makeSound) {
      out.makeSound = await makeSoundSteps(S, ev);
      log('make sound', out.makeSound);
    } else {
      out.consent = await dismissStartupBox(S, ev);
      log('consent box', out.consent);
    }

    if (projectFile !== null) {
      const logBefore = await ev('window.__app.engineLog().length');
      openLogFrom = logBefore;
      await openProject(S, ev, projectFile);
      await projectReport(ev, out, logBefore);
    }
    if (soundOn) await closeStartupBoxes(S, ev);
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await tapKey(S, 'Space', ' ', 32);
    out.keyAtGuest = await keyShown(ev, 'vk 0x20', 2000);
    log('Space sent', { reachedGuest: out.keyAtGuest });
    if (soundOn) {
      await sleep(500);
      out.soundStartMs = await pressMetronome(S, ev);
      log('sound', { startMs: out.soundStartMs });
    }

    const pcmIdx = await ev(armExpression());
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
    Object.assign(out, { presentsDelta: p0 === null || p1 === null ? null : p1 - p0, quantaDelta: d1.quantaWhilePlaying - d0.quantaWhilePlaying, underrunDelta: d1.underrunQuanta - d0.underrunQuanta, nonSilentDelta: d1.nonSilentQuanta - d0.nonSilentQuanta });
    if (pcmIdx !== null) Object.assign(out, await takePcm(ev, pcmIdx));
    if (makeSound) out.makeSound.log = await soundLog(ev);
    if (d1.error) probeError = d1.error;
    out.end = await engineEnd();
  } catch (e) {
    probeError = String(e?.message ?? e).slice(0, 500);
    if (evAfterError) out.end = await engineEnd();
    if (evAfterError && openLogFrom !== null) await openBreakdownAfterError(evAfterError, openLogFrom, out);
    // Issue #1412: projectReport did not run (the open failed or stopped), so envLog is the environment lines of the engine's log from the start of the run
    if (projectFile !== null && out.envLog === null && evAfterError) out.envLog = await evAfterError("(window.__app ? window.__app.engineLog() : [])").then((log) => log.filter((l) => ENV_LINE.test(l))).catch(() => null);
  } finally {
    out.error = probeError;
    if (navAt !== null) out.elapsedMs = Date.now() - navAt;
    out.loadavg = os.loadavg();
    // Issue #1400: the engine's reads of the real modules' images (name, requested) in the order they happened: a Sytrus row with requested true is the plugin DLL's image read by the guest
    if (evAfterError) out.imageReads = await evAfterError("(() => { const a = window.__app; return a ? a.imageReads.map((r) => ({ name: r.name, requested: r.requested, atMs: r.atMs })) : null; })()").catch(() => null);
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
    process.stdout.write(JSON.stringify(out) + '\n');
    process.exitCode = out.error && openLogFrom === null ? 1 : 0;
  }
}

// --boot (#1334): the one press that starts the AudioContext comes --press-at seconds after engine-idle (default 0; the status line, as startAudio does; no guest event). The window is t0+10 s to t0+70 s: nothing is sent to the guest in it, and the reads are taken at its two ends. The read at t0+90 s (bootLast60sAt90s: the last 60 s are t0+30 s to t0+90 s, past the start-up peak) comes after the one at +70 s.
async function bootWindow(S, ev, audioNow, fromAudio, cdp, workers, out, tIdle, pageSession) {
  const engine = await engineWorker(cdp, workers);
  if (engine === null) throw new Error('no engine Worker with fleAudioWorkerState (the boot window reads it)');
  const stop = { done: false };
  const trace = tracePath !== null ? traceLoop(audioNow, cdp, engine, tIdle, stop) : null;
  try {
    // #1375: the publish log of the output ring (PUB_HOOK) is taken from the engine's Worker for the whole boot window; its window is read in the Worker's own clock (w0, w1)
    const hooked = await workerEval(cdp, engine, PUB_HOOK);
    let w0 = null, w1 = null, pubLog = null, s0 = null, s1 = null;
    if (profilePath !== null) await profileStart(cdp, engine);
    if (profilePath !== null) await profileStart(cdp, pageSession);
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
      w0 = await workerEval(cdp, engine, 'performance.now()');
      s0 = await workerEval(cdp, engine, SCHED_STATS);
    };
    // the read at +10 s comes before the press when the press is later than that (the reads are at their own times, in time order)
    if (out.pressAtMs <= BOOT_READ_MS.first) { await press(); await read10(); } else { await read10(); await press(); }
    await sleepUntil(tIdle + BOOT_READ_MS.last);
    const d70 = await audioNow();
    const at70 = Date.now() - tIdle;
    await sleepUntil(tIdle + BOOT_READ_MS.final);
    const d90 = await audioNow();
    const at90 = Date.now() - tIdle;
    w1 = await workerEval(cdp, engine, 'performance.now()');
    s1 = await workerEval(cdp, engine, SCHED_STATS);
    pubLog = await workerEval(cdp, engine, PUB_LOG);
    out.schedStats = { at10s: s0, at90s: s1, names: ['loopSlices', 'loopNoThreads', 'loopNotRunning', 'loopInCallback', 'loopDepth1', 'loopSwitched', 'progressSlices', 'progressInCallback'] };
    fromAudio(d90);
    out.readAtMs = [at10, at70, at90];
    out.bootLast60sAt10s = d10.underrunQuantaLast60s;
    out.bootLast60sAt70s = d70.underrunQuantaLast60s;
    out.bootLast60sAt90s = d90.underrunQuantaLast60s;
    out.quantaDelta = d70.quantaWhilePlaying - d10.quantaWhilePlaying;
    out.maxRefillStallMs = await enginePublishGap(cdp, workers);
    if (hooked !== null && pubLog !== null) {
      const win = windowMaxGap(pubLog, w0, w1);
      out.maxRefillStallInWindowMs = win.maxMs;
      out.publishesInWindow = win.publishes;
      if (pubsPath !== null) fs.writeFileSync(pubsPath, JSON.stringify({ w0, w1, entries: pubLog }));
      out.pubLog = pubsPath;
    }
    log('boot window', { at10, at70, at90, last60sAt10s: out.bootLast60sAt10s, last60sAt70s: out.bootLast60sAt70s, last60sAt90s: out.bootLast60sAt90s, quantaDelta: out.quantaDelta, maxRefillStallMs: out.maxRefillStallMs, maxRefillStallInWindowMs: out.maxRefillStallInWindowMs, ctxState: d90.ctxState });
    if (profilePath !== null) await profileStop(cdp, engine, profilePath, true);
    if (profilePath !== null) await profileStop(cdp, pageSession, mainProfilePath(profilePath), false);
    out.profile = profilePath;
    out.profileMain = profilePath === null ? null : mainProfilePath(profilePath);
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

// --profile (#1352): the page's main thread is profiled over the same window, to FILE with ".main" before its extension
function mainProfilePath(file) {
  const e = path.extname(file);
  return file.slice(0, file.length - e.length) + '.main' + e;
}

// --profile (#1375): the Worker's performance.now() taken just before the stop, with the CDP clock at the end of the profile, so that the publish log (in the Worker's clock) can be put on the samples' timeline (profile.fleClock.offsetUs: CDP microseconds = performance.now() ms x 1000 + offsetUs)
async function profileStop(cdp, engine, file, clock) {
  const perfStop = clock ? await workerEval(cdp, engine, 'performance.now()') : null;
  const r = await timed(cdp.send('Profiler.stop', {}, engine), 300000, 'Profiler.stop');
  if (perfStop !== null) r.profile.fleClock = { perfAtStopMs: perfStop, offsetUs: r.profile.endTime - perfStop * 1000 };
  fs.writeFileSync(file, JSON.stringify(r.profile));
  log('profile', { file, samples: r.profile.samples?.length ?? null });
}

async function sleepUntil(at) {
  const d = at - Date.now();
  if (d > 0) await sleep(d);
}

// #1375: the publish log of the output ring. fle_js_publish (fle_audio_transport.c) sets A.runPub (whether the output was running) and then assigns A.lastPub = now at the end of each publish; a setter on lastPub appends [now, running] to A.__pl. Only this probe reads it; the product is not changed.
const PUB_HOOK = "(() => { const A = globalThis.__fleAud; if (!A) return null; if (!A.__pl) { let lp = A.lastPub; A.__pl = []; Object.defineProperty(A, 'lastPub', { configurable: true, enumerable: true, get() { return lp; }, set(v) { lp = v; A.__pl.push([v, A.runPub ? 1 : 0]); } }); } return performance.now(); })()";
const PUB_LOG = "(() => { const A = globalThis.__fleAud; return A && A.__pl ? A.__pl : null; })()";
// #1375: the counts of the scheduler points (fle_thread.c sched_stat, read through fle_audio_sched_stat in fle_audio_transport.c)
const SCHED_STATS = 'globalThis.fleSchedStats ? globalThis.fleSchedStats() : null';

// the engine's Worker: an expression's value (returnByValue)
async function workerEval(cdp, sid, expression) {
  const r = await timed(cdp.send('Runtime.evaluate', { expression, returnByValue: true }, sid), CDP_TIMEOUT_MS, 'the engine worker');
  if (r.exceptionDetails) throw new Error(`worker evaluate: ${r.exceptionDetails.text}`);
  return r.result.value;
}

// #1375: the longest interval between two publishes whose both ends are in [w0, w1] and which both found the output running (the engine's own rule for maxPubGap). publishes: the count of publishes in the window.
function windowMaxGap(entries, w0, w1) {
  let maxMs = 0, publishes = 0;
  for (let i = 0; i < entries.length; i++) {
    const [t] = entries[i];
    if (t >= w0 && t <= w1) publishes++;
    if (i === 0) continue;
    const [tp, rp] = entries[i - 1];
    if (tp < w0 || t > w1) continue;
    if (rp && entries[i][1] && t - tp > maxMs) maxMs = t - tp;
  }
  return { maxMs: Math.round(maxMs * 10) / 10, publishes };
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

// The project is opened with Ctrl+O. The usage-data box's "No thanks" (above) leaves the Welcome window up: it is closed with Escape (Issue #1366), not by opening its Default row, because the Default project's load stopped the engine (exit code 134, after Python312.dll was not found) before the open box came. Then Ctrl+O gives the engine's open box: the page's file box (#filedlg-input), or the file manager in selection mode (#fx-input-files: the file is uploaded, chosen in its list, and Open is pressed). The engine's "Problems loading the project" box, if any, is closed by Enter.
const WELCOME_CLASS = 'TWelcomeWizard';
async function openProject(S, ev, file) {
  const stem = path.basename(file, path.extname(file));
  const name = path.basename(file);
  const welcomeListed = (await windowsOf(ev)).some((w) => w.cls === WELCOME_CLASS);
  if (welcomeListed) {
    await tapKey(S, 'Escape', 'Escape', 0x1b);
    await until(async () => !(await windowsOf(ev)).some((w) => w.cls === WELCOME_CLASS), 30000, 'the Welcome window closed with Escape');
  }
  await dismissProblems(S, ev);
  log('before Ctrl+O', { welcome: welcomeListed, title: await ev('window.__app.pageTitle().title'), windows: (await windowsOf(ev)).map((w) => `${w.cls}|${w.title}`) });
  await ctrlO(S, ev);
  let box = null;
  let again = false;
  const t0 = Date.now();
  while (!box && Date.now() - t0 < 40000) {
    box = await ev("document.getElementById('filedlg-input') ? 'filedlg' : (document.getElementById('fx-input-files') ? 'explorer' : null)");
    if (!box && (await dismissProblems(S, ev)) > 0 && !again) {
      again = true;
      await ctrlO(S, ev);
    }
    if (!box) await sleep(500);
  }
  if (!box) {
    const elog = await ev('window.__app.engineLog()');
    log('no open box', { title: await ev('window.__app.pageTitle().title'), windows: (await windowsOf(ev)).map((w) => `${w.cls}|${w.title}`), engineLine: await ev("(document.getElementById('engine-line')?.textContent ?? '')"), tail: elog.slice(-6).map((l) => l.slice(0, 160)) });
    throw new Error('no open box (#filedlg-input or #fx-input-files) 40 s after Ctrl+O');
  }
  const selector = box === 'filedlg' ? '#filedlg-input' : '#fx-input-files';
  const doc = await S('DOM.getDocument', {});
  const q = await S('DOM.querySelector', { nodeId: doc.root.nodeId, selector });
  if (!q.nodeId) throw new Error(`${selector} is not a node`);
  await S('DOM.setFileInputFiles', { nodeId: q.nodeId, files: [file] });
  if (box === 'explorer') {
    const row = `#fx-list [data-name="${name}"]`;
    const t2 = Date.now();
    while (!(await ev(`!!document.querySelector(${JSON.stringify(row)})`))) {
      if (Date.now() - t2 >= 30000) throw new Error(`${name} is not listed in the file manager 30 s after the upload`);
      await sleep(500);
    }
    await clickSelector(S, ev, row);
    await clickSelector(S, ev, '#fx-ok');
  }
  // the title is the project's name as the guest shows it (its case may differ from the file name), or the "Problems loading the project" box's title while that box is the active window (the box is then the one projectReport reads)
  const title = () => ev('window.__app.pageTitle().title');
  const opened = async () => {
    const st = await ev('window.__app ? window.__app.state : null');
    if (st === 'exited' || st === 'error') throw new Error(`the engine is ${st} during the open`);
    const t = (await title()) ?? '';
    return t.toLowerCase().startsWith(stem.toLowerCase()) || PROBLEMS_TITLE.test(t);
  };
  try {
    await until(opened, 180000, `the title of ${stem}`);
  } catch (e) {
    throw new Error(`${e.message} (the title is ${JSON.stringify(await title())})`);
  }
  log('project open', { title: await title() });
  await sleep(4000);
}

// the "Problems loading the project" box is seen by the front window's title (pageTitle: the box is the active window while it is shown) or by its entry in the window list
async function problemsOpen(ev) {
  const front = (await ev("window.__app ? window.__app.pageTitle().title : ''")) ?? '';
  if (PROBLEMS_TITLE.test(front)) return true;
  return (await windowsOf(ev)).some((w) => PROBLEMS_TITLE.test(w.title));
}

// the engine's "Problems loading the project" box is modal: Enter closes it (its default button). Returns how many were closed.
async function dismissProblems(S, ev) {
  let n = 0;
  for (let i = 0; i < 4 && (await problemsOpen(ev)); i++) {
    const key = { code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 0x0d, nativeVirtualKeyCode: 0x0d };
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
    await sleep(1000);
    n++;
  }
  return n;
}

// Ctrl+O: the engine reads the Ctrl state from its own key messages, so Control goes down first (VK_LCONTROL, as tools/delivery-split/measure.mjs sends it)
async function ctrlO(S, ev) {
  await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
  await sleep(100);
  const ctrl = { code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2 };
  const key = { code: 'KeyO', key: 'o', windowsVirtualKeyCode: 79, nativeVirtualKeyCode: 79 };
  await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...ctrl, modifiers: 2 });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key, modifiers: 2 });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', ...key, modifiers: 2 });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', ...ctrl, modifiers: 0 });
}

async function clickSelector(S, ev, selector) {
  const r = await ev(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()`);
  if (!r) throw new Error(`no element ${selector}`);
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r.x, y: r.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(60);
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: r.x, y: r.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(50);
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: r.x, y: r.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(300);
}

async function until(fn, ms, what) {
  const t = Date.now();
  for (;;) {
    if (await fn()) return;
    if (Date.now() - t >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
  }
}

// Issue #1366: what the guest shows once the project is open. The "Problems loading the project" box is a window of its own, so projectDialogs lists the windows with that title. The box comes with the load: the windows are polled for PROJECT_DIALOG_MS after the open, and the list is empty only when no such box came in that time. projectWindows: the windows at the end (title and class). projectMissing: the sample requests of the open that found nothing (the engine's missing-path lines of a .wav, from the log after logBefore).
async function projectReport(ev, out, logBefore) {
  const t0 = Date.now();
  while (!(await problemsOpen(ev)) && Date.now() - t0 < PROJECT_DIALOG_MS) await sleep(POLL_MS);
  const wins = await windowsOf(ev);
  out.projectDialogs = wins.filter((w) => PROBLEMS_TITLE.test(w.title)).map((w) => ({ cls: w.cls, title: w.title, x: w.x, y: w.y, w: w.w, h: w.h }));
  const front = (await ev("window.__app ? window.__app.pageTitle().title : ''")) ?? '';
  if (out.projectDialogs.length === 0 && PROBLEMS_TITLE.test(front)) out.projectDialogs = [{ cls: '', title: front }];
  out.projectWindows = wins.map((w) => ({ cls: w.cls, title: w.title }));
  const log = await ev('window.__app.engineLog()');
  out.envLog = log.filter((l) => ENV_LINE.test(l));
  // Issue #1412: the missing-path lines of a request that still spells a %...% variable (the API and the code are in each line; the first 40)
  out.variableMissing = log.filter((l) => MISSING_LINE.test(l) && l.includes('"%')).slice(0, 40);
  const missing = log.slice(logBefore).filter((l) => SAMPLE_MISSING.test(l));
  out.projectMissingLines = missing.length;
  out.projectMissing = [...new Set(missing)];
  process.stderr.write(`project: ${out.projectDialogs.length} problem box(es), ${out.projectMissing.length} distinct factory sample request(s) not found (${out.projectMissingLines} lines)\n`);
  Object.assign(out, missingBreakdown(log.slice(logBefore)));
  process.stderr.write(`missing-path: ${out.missingDistinct} distinct: ${out.dll} dll, ${out.missingInInventory} absolute file requests in the installer's list, ${out.rawVariableProbes.count} %...% probes, ${out.notInInstaller} not in it\n`);
}

// Issue #1403: the breakdown of the whole log of the open, when the engine stopped during it (the end tail is only the last 400 lines, so the requests before it would be missed)
async function openBreakdownAfterError(ev, from, out) {
  const log = await ev('window.__app.engineLog()').catch(() => null);
  if (Array.isArray(log)) Object.assign(out, missingBreakdown(log.slice(from)));
}

// Issue #1373: the installer's list as a map from the guest path (lower case) to its row: the guest's spelling, bytes and sha256 (inputs/fl-installer-files.tsv; the header and the comment are skipped)
function installerRows() {
  const rows = new Map();
  for (const line of fs.readFileSync(INSTALLER_LIST, 'utf8').split('\n')) {
    const [guest, bytes, sha256] = line.replace(/\r$/, '').split('\t');
    if (!guest || guest.startsWith('#') || guest === 'guest') continue;
    rows.set(guest.toLowerCase(), { guest, bytes: bytes === '' ? null : Number(bytes), sha256: sha256 === '' ? null : sha256 });
  }
  return rows;
}

// Issue #1373: the candidates for a factory root of a path, as [kind, root]: each leading part that ends in \ of an absolute path, and the %...%\ variable at its start
function rootCandidates(full) {
  const out = [];
  const v = VARIABLE_PATH.exec(full);
  if (v) out.push(['variable', v[0]]);
  if (ABSOLUTE_PATH.test(full)) {
    for (let i = full.indexOf('\\'); i >= 0; i = full.indexOf('\\', i + 1)) out.push(['absolute', full.slice(0, i + 1)]);
  }
  return out;
}

// Issue #1373: the two factory roots, read from the log. Each kind's root is the candidate (lower case) that takes the most of the distinct paths to a row of the installer's list; a kind with no such candidate has no root (null). Nothing is written here: the values come from the engine's own paths.
function factoryRoots(paths, rows) {
  const count = { absolute: new Map(), variable: new Map() };
  for (const full of new Set(paths)) {
    for (const [kind, root] of rootCandidates(full)) {
      const key = root.toLowerCase();
      if (rows.has(full.slice(root.length).toLowerCase())) count[kind].set(key, (count[kind].get(key) ?? 0) + 1);
    }
  }
  const top = (m) => {
    let best = null;
    for (const [root, n] of m) if (best === null || n > best[1]) best = [root, n];
    return best === null ? null : best[0];
  };
  return { absolute: top(count.absolute), variable: top(count.variable) };
}

// Issue #1373: which root a path has ('absolute', 'variable', or null), the lower-case roots of factoryRoots
function rootKind(full, roots) {
  for (const kind of ['absolute', 'variable']) {
    const r = roots[kind];
    if (r !== null && full.slice(0, r.length).toLowerCase() === r) return kind;
  }
  return null;
}

// Issue #1373: the breakdown of the missing-path lines of an engine log. The runtime writes a UTF-16 unit that is not printable ASCII (and the quote) as \uXXXX, which is decoded here. A request is keyed by its path without the factory root, in lower case; the first spelling seen is the one listed.
// Issue #1403: missingInInventory counts the absolute spellings (lower case, once each) of a FILE_APIS request, once the absolute root is taken off they are a row of the list, and they are not a .dll. rawVariableProbes counts the %...%\ spellings (lower case, once each). notInInstaller is the rest: not a .dll, not in the list, not a %...%\ spelling.
function missingBreakdown(lines) {
  const reqs = [];
  for (const line of lines) {
    const m = MISSING_LINE.exec(line);
    if (m) reqs.push({ api: m[1], full: m[3].replace(/\\u([0-9A-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16))) });
  }
  const rows = installerRows();
  const roots = factoryRoots(reqs.map((r) => r.full), rows);
  const rootStripped = { absolute: 0, variable: 0 };
  const seen = new Map();
  const rawVariable = new Map();
  const absFile = new Map();
  for (const { api, full } of reqs) {
    const kind = rootKind(full, roots);
    if (kind !== null) rootStripped[kind]++;
    const key = (kind === null ? full : full.slice(roots[kind].length)).toLowerCase();
    if (!seen.has(key)) seen.set(key, full);
    const low = full.toLowerCase();
    if (VARIABLE_PATH.test(full) && !rawVariable.has(low)) rawVariable.set(low, full);
    if (FILE_APIS.has(api) && ABSOLUTE_PATH.test(full) && !absFile.has(low)) absFile.set(low, { api, full });
  }
  const missing = [];
  for (const [low, { api, full }] of absFile) {
    if (roots.absolute === null || !low.startsWith(roots.absolute)) continue;
    if (/\.dll$/i.test(full) || !rows.has(low.slice(roots.absolute.length))) continue;
    missing.push({ api, path: full });
  }
  const dll = [];
  const inList = [];
  const notIn = [];
  for (const [key, full] of seen) {
    if (/\.dll$/i.test(full)) dll.push(full);
    else if (rows.has(key)) inList.push(rows.get(key));
    else if (!VARIABLE_PATH.test(full)) notIn.push(full);
  }
  return {
    missingDistinct: seen.size, dll: dll.length, inventory: inList.length, inventoryList: inList,
    missingInInventory: missing.length, missingInInventoryFirst10: missing.slice(0, MISSING_FIRST),
    rawVariableProbes: { count: rawVariable.size, first10: [...rawVariable.values()].slice(0, MISSING_FIRST) },
    notInInstaller: notIn.length, notInInstallerFirst10: notIn.slice(0, NOT_IN_INSTALLER_FIRST), notInInstallerList: notIn, rootStripped,
  };
}

// the guest's windows, read again for a while when the page answers "ask again" (the picture is being drawn: tools/delivery-split/measure.mjs does the same)
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

/** the windows of a window list (the page's format: lid N at (x, y) WxH ... class "..." title "..."), as tools/delivery-split/measure.mjs reads them */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)"/.exec(m[6]);
    const title = /title "([^"]*)"/.exec(m[6]);
    return { x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: title ? title[1] : '' };
  });
}

// guest pixel to client pixel of the desktop element (the guest's screen is fitted into the element)
async function guestPoint(ev, gx, gy) {
  const g = await ev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const s = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: s.width, gh: s.height }; })()`);
  return { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
}

// a left click at a guest pixel; clicks = 2 is a double click (clickCount 1, then 2)
async function clickGuest(S, ev, gx, gy, clicks = 1) {
  const { x, y } = await guestPoint(ev, gx, gy);
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(120);
  for (let k = 1; k <= clicks; k++) {
    await S('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: k, pointerType: 'mouse' });
    await sleep(70);
    await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: k, pointerType: 'mouse' });
    await sleep(60);
  }
}

// --sound (#1422): the metronome button is pressed at its guest pixel (CSS px x the page's device pixel ratio, the steps of tools/p2-frames/measure.mjs --load b). Returns the ms until the non-silent quanta had risen by SOUND_RISE_QUANTA; throws when they did not within SOUND_WAIT_MS (with the state of the guest).
async function pressMetronome(S, ev) {
  const pageDpr = await ev('window.devicePixelRatio');
  const n0 = (await ev('window.__app.audio.detail()')).nonSilentQuanta;
  await clickGuest(S, ev, Math.round(METRONOME.x * pageDpr), Math.round(METRONOME.y * pageDpr));
  const t0 = Date.now();
  for (;;) {
    const n1 = (await ev('window.__app.audio.detail()')).nonSilentQuanta;
    if (n1 > n0 + SOUND_RISE_QUANTA) return Date.now() - t0;
    if (Date.now() - t0 >= SOUND_WAIT_MS) {
      const wins = (await windowsOf(ev).catch(() => [])).map((w) => `${w.cls}|${w.title}`).join(', ') || 'none';
      throw new Error(`the playback is silent after the metronome button (nonSilentQuanta ${n0} -> ${n1} in ${SOUND_WAIT_MS} ms; windows ${wins})`.slice(0, 500));
    }
    await sleep(POLL_MS);
  }
}

// --sound (#1422): the Welcome window is closed by its X (the place of tools/p2-frames/measure.mjs's welcome step, in guest pixels of the window list) and the "Problems loading the project" box with Enter (its default button, as openProject does): a modal box would take the Space key and the metronome click
async function closeStartupBoxes(S, ev) {
  const wiz = (await windowsOf(ev)).find((w) => w.cls === WELCOME_CLASS);
  if (wiz) {
    await clickGuest(S, ev, wiz.x + wiz.w - 18, wiz.y + 18);
    await until(async () => !(await windowsOf(ev)).some((w) => w.cls === WELCOME_CLASS), 30000, 'the Welcome window closed');
  }
  await dismissProblems(S, ev);
}

// the PCM check (#1422) of the capture armed at the window's start: waits for it (PCM_WAIT_MS); the summary is computed in the page; null values when it did not come
async function takePcm(ev, index) {
  const t0 = Date.now();
  for (;;) {
    const summary = await ev(takeExpression(index));
    if (summary !== null || Date.now() - t0 >= PCM_WAIT_MS) {
      process.stderr.write(`pcm: ${summary?.frames ?? 'no capture'} frames, pcmRepeat8192 ${summary ? pcmReport(summary).pcmRepeat8192 : null}\n`);
      return pcmReport(summary);
    }
    await sleep(POLL_MS);
  }
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

// Issue #1401 --make-sound: the room (1836 x 1056 CSS px at DPR 1, as tools/smoke/welcome-flow.mjs sets it); the Welcome's usage-data box and its Default row (flow A of welcome-flow.mjs: the offsets from the Welcome window's top-left corner, measured at this room); the Channel rack's Kick row (its fractions of the rack's rectangle, measured from the picture at this room)
const MS_VIEW = { width: 2176, height: 1092 };
const ROOM = { w: 1836, h: 1056 };
const STAGE_TEXT = "(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()";
const WELCOME_TITLE = /^Welcome to FL Studio/;
const NO_THANKS_AT = { dx: 512, dy: 307 };
const LIST_AT = { dx: 235, dy: 180 };
const DEFAULT_ROW_AT = { dx: 235, dy: 117 };
const NOTCH = { up: 12, down: 3, delta: 120 };
const REMIX_CLASS = 'TDemucsForm';
const RACK_CLASS = 'TStepSeqForm';
// the Channel rack's Kick row (the first channel): fy is the centre of the row as a fraction of the rack's height; the steps are n points whose centres run from fx0 to fx1 (fractions of the rack's width), spaced evenly. Measured from the picture at this room (the rack at 872, 82, 499 x 246 guest px): the buttons are 16 px apart, the first centre at x 1096, the last at x 1336, the row's centre at y 145.
const KICK_ROW = { fy: 0.256 };
const STEPS = { n: 16, fx0: 0.448, fx1: 0.929 };
const STEP_SETTLE_MS = 150;
const WELCOME_WAIT_MS = 30000, OPEN_WAIT_MS = 120000, RACK_WAIT_MS = 15000;
const ESCAPE_VK = 27;
const WINDOWS_TABLE = '(async () => (await window.__app.deskDumpBase64(false)).windows)()';
const PICTURE_TEXT = '(async () => { const d = await window.__app.deskDumpBase64(true); return { width: d.width, height: d.height, base64: d.base64 }; })()';

// the calls that the page answers "ask again" to (the picture or the window table is being drawn) are asked again every 100 ms, for up to 15 s
async function askAgain(fn) {
  const t0 = Date.now();
  for (;;) {
    try {
      return await fn();
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
      await sleep(100);
    }
  }
}

/** the window table with the children (Issue #1401): { child, lid, parent, x, y, w, h, cls, text }; a child's x and y are relative to its parent (absolutePosition adds them) */
function parseTable(text) {
  return text.split('\n').map((l) => /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    return { child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]), cls: cls ? cls[1] : '', text: cap ? cap[1] : '' };
  });
}

/** the guest position of a window: a child's position is added to its parents' */
function absolutePosition(list, win) {
  let x = win.x, y = win.y, cur = win;
  while (cur.child && cur.parent !== null) {
    const up = list.find((w) => w.lid === cur.parent);
    if (!up) break;
    x += up.x; y += up.y;
    cur = up;
  }
  return { x, y };
}

async function tableOf(ev) { return parseTable(await askAgain(() => ev(WINDOWS_TABLE))); }

/** the hash of a rectangle of the guest's picture (4 bytes a pixel): the picture changes when a step is lit */
function rectHash(picture, rect) {
  const bytes = Buffer.from(picture.base64, 'base64');
  const stride = bytes.length / picture.height;
  const h = createHash('sha1');
  const x0 = Math.max(0, rect.x), x1 = Math.min(picture.width, rect.x + rect.w);
  for (let y = Math.max(0, rect.y); y < Math.min(picture.height, rect.y + rect.h); y++) h.update(bytes.subarray(y * stride + x0 * 4, y * stride + x1 * 4));
  return h.digest('hex');
}

/** polls fn every POLL_MS until it returns a true value; throws after ms */
async function pollFor(fn, ms, what) {
  const t = Date.now();
  for (;;) {
    const v = await fn();
    if (v) return v;
    if (Date.now() - t >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
  }
}

// a wheel of notches over a guest pixel (deltaY: NOTCH.delta down, -NOTCH.delta up)
async function wheelGuest(S, ev, gx, gy, notches, deltaY) {
  const { x, y } = await guestPoint(ev, gx, gy);
  for (let i = 0; i < notches; i++) {
    await S('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: 0, deltaY, pointerType: 'mouse' });
    await sleep(300);
  }
}

// --make-sound (Issue #1401): the Welcome's "No thanks" and Default row (flow A), the project's boxes, then the Kick row of the Channel rack is clicked step by step. The record (makeSound in the JSON): the room, the Welcome window, the title the project opened with, the boxes closed, the rack's rectangle, clicks, litSteps (the steps whose rectangle's picture changed) and the shot.
async function makeSoundSteps(S, ev) {
  const steps = [];
  const rec = { room: null, welcome: null, opened: null, problemBoxes: 0, remixClosed: null, channelRack: null, stepPoints: STEPS.n, clicks: 0, litSteps: 0, shot: null, steps };
  await S('Emulation.setDeviceMetricsOverride', { width: MS_VIEW.width, height: MS_VIEW.height, deviceScaleFactor: 1, mobile: false });
  await sleep(1500);
  for (let i = 0; i < 4; i++) {
    const s = await ev(STAGE_TEXT);
    if (Math.abs(s.sw - ROOM.w) <= 0.5 && Math.abs(s.sh - ROOM.h) <= 0.5) break;
    await S('Emulation.setDeviceMetricsOverride', { width: Math.round(s.iw + (ROOM.w - s.sw)), height: Math.round(s.ih + (ROOM.h - s.sh)), deviceScaleFactor: 1, mobile: false });
    await sleep(1500);
  }
  rec.room = `${ROOM.w}x${ROOM.h}`;
  const welcome = await pollFor(async () => (await tableOf(ev)).find((w) => w.cls === WELCOME_CLASS && !w.child) ?? null, WELCOME_WAIT_MS, 'the Welcome window');
  rec.welcome = { x: welcome.x, y: welcome.y, w: welcome.w, h: welcome.h };
  await clickGuest(S, ev, welcome.x + NO_THANKS_AT.dx, welcome.y + NO_THANKS_AT.dy);
  steps.push(`"No thanks" at guest (${welcome.x + NO_THANKS_AT.dx}, ${welcome.y + NO_THANKS_AT.dy})`);
  await sleep(2500);
  await wheelGuest(S, ev, welcome.x + LIST_AT.dx, welcome.y + LIST_AT.dy, NOTCH.up, -NOTCH.delta);
  await wheelGuest(S, ev, welcome.x + LIST_AT.dx, welcome.y + LIST_AT.dy, NOTCH.down, NOTCH.delta);
  await sleep(400);
  await clickGuest(S, ev, welcome.x + DEFAULT_ROW_AT.dx, welcome.y + DEFAULT_ROW_AT.dy, 2);
  steps.push('Default row double-clicked');
  const opened = await pollFor(async () => {
    const st = await ev('window.__app ? window.__app.state : null');
    if (st === 'error' || st === 'exited') throw new Error(`state ${st} while the default project opens`);
    const title = (await ev('window.__app.pageTitle().title')) ?? '';
    const welcomeListed = (await tableOf(ev)).some((w) => w.cls === WELCOME_CLASS);
    return st === 'engine-idle' && title !== '' && !WELCOME_TITLE.test(title) && !welcomeListed ? title : null;
  }, OPEN_WAIT_MS, 'the default project opens');
  rec.opened = opened;
  await sleep(2000);
  rec.problemBoxes = await dismissProblems(S, ev);
  await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
  for (let i = 0; i < 3 && (await tableOf(ev)).some((w) => w.cls === REMIX_CLASS); i++) {
    await tapKey(S, 'Escape', 'Escape', ESCAPE_VK);
    await sleep(1500);
  }
  rec.remixClosed = !(await tableOf(ev)).some((w) => w.cls === REMIX_CLASS);
  steps.push(`project open: title "${opened}", ${rec.problemBoxes} Problems box(es) closed, Remix box closed ${rec.remixClosed}`);
  const rack = await pollFor(async () => {
    const t = await tableOf(ev);
    const w = t.find((x) => x.cls === RACK_CLASS);
    return w ? { lid: w.lid, w: w.w, h: w.h, at: absolutePosition(t, w) } : null;
  }, RACK_WAIT_MS, 'the Channel rack (TStepSeqForm)');
  const rect = { x: rack.at.x, y: rack.at.y, w: rack.w, h: rack.h };
  rec.channelRack = { lid: rack.lid, ...rect };
  const neutral = await ev("(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()");
  const away = () => S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: neutral.x, y: neutral.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  for (let i = 0; i < STEPS.n; i++) {
    const gx = Math.round(rect.x + (STEPS.fx0 + ((STEPS.fx1 - STEPS.fx0) * i) / (STEPS.n - 1)) * rect.w);
    const gy = Math.round(rect.y + KICK_ROW.fy * rect.h);
    await away();
    await sleep(STEP_SETTLE_MS);
    const before = rectHash(await askAgain(() => ev(PICTURE_TEXT)), rect);
    await clickGuest(S, ev, gx, gy);
    await away();
    await sleep(STEP_SETTLE_MS);
    const after = rectHash(await askAgain(() => ev(PICTURE_TEXT)), rect);
    rec.clicks++;
    if (after !== before) rec.litSteps++;
  }
  steps.push(`${rec.clicks} clicks on the Kick row, ${rec.litSteps} lit`);
  if (shotPath !== null) {
    const s = await S('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(shotPath, Buffer.from(s.data, 'base64'));
    rec.shot = shotPath;
  }
  return rec;
}

// --make-sound: the engine's lines about the Default project's sample files (Basic 808 *.wav): how many, how many of them missing-path lines (basic808MissingWav: those for a .wav, which the sample needs), and the first 5 (cut to 200 characters)
async function soundLog(ev) {
  const lines = (await ev('window.__app.engineLog()')).map((l) => l.replace(/^\[err\] /, ''));
  const b = lines.filter((l) => /Basic 808/.test(l));
  const missing = b.filter((l) => l.startsWith('missing-path:'));
  return { basic808Lines: b.length, basic808Missing: missing.length, basic808MissingWav: missing.filter((l) => /\.wav"$/i.test(l)).length, basic808First: b.slice(0, 5).map((l) => l.slice(0, 200)) };
}

await main();
