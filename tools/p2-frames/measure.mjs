// P2 frames measure (#1350). The page is the real engine's page in headless Chrome, in its own room at a device pixel ratio (the stage is 1836 x 1056 CSS px; the guest screen the page gives for it is 3240 x 1864 device px at 1.7647), with the pointer sent at 1000 Hz or more for 30 s.
// Usage: node tools/p2-frames/measure.mjs <absolute path to the bundle directory raw/<name>> [--dpr <value>] [--load a|b] [--app <dir>] > p2.json   (needs app/node_modules and app/dist). Defaults: --dpr 1.7647 --load a, --app is this repository's app. Exit 0 when error is null, 1 when the run failed, 2 when the arguments are wrong.
// --app <dir> (#1422): the page measured is the app of that directory (the A/B measurement's base build); its node_modules and dist are used.
// stdout: ONE JSON line {error,dpr,load,presentedPerSec,minWindowFps,inputDropped,cpuPerPresentMsP50,cpuPerPresentMsP95,budgetRatioP95,inputToPresentMsP50,inputToPresentMsP95,inputRateOk,valid,desk,notes}. Nothing else goes to stdout.
// minWindowFps (#1422): the least of the 30 one-second windows of the span, each one's presents (the engine's presents, deskStats) divided by its length. The mean is presentedPerSec; the least is the number P2 is judged on.
// inputDropped (#1422): the inputs of the window that the page's latency records mark as dropped (the same count as droppedN in notes: the engine's input queue was full, or the input was merged into a later move).
// desk (the breakdown of one present over the window, from the presenter's counters): presents, rectsPerPresent, pixelsPerPresent, pixelShare (of the guest screen), uploadsPerPresent, uploadBytesPerPresent, drawsPerPresent, drawMsPerPresent, drawPixelsPerPresent, drawPixelShare, composeMsPerPresent, putMsPerPresent, wholePerWindow, texAllocsPerWindow, and the CPU per present of the threads (chrome = the main thread (presumed), DedicatedWorker = the engine and render Workers, Compositor, GPU process).
// What is counted (the definitions are in notes too):
//   presentedPerSec   the frames the compositor drew to the display in the window: 'Display::DrawAndSwap' events of the trace category viz (a record of what was shown, not a count of rAF callbacks). The engine's own count of its presents (window.__app.deskStats.presents) is the cross-check in notes.
//   cpuPerPresentMs   the CPU time of the renderer process (SystemInfo.getProcessInfo cpuTime: the page's main thread, its Workers and its compositor) in each second of the window, divided by the engine's presents in that second. P50 and P95 over the seconds that had a present. budgetRatioP95 = P95 / (1000 / 120). The GPU process's CPU is in notes, not in this number.
//   inputToPresentMs  for each input the page sent to the engine in the window that a present answered: from the DOM event's own timeStamp (tPage when that is not usable) to the present the render Worker drew for it (window.__app.latency.records, answers). Inputs that changed no pixel have no answer and are counted in notes.
//   inputRateOk       the pointer events that reached the page (document, capture: getCoalescedEvents count) were at least 1000 in every second of the window.
// The pointer is sent by CDP Input.dispatchMouseEvent (trusted events) at SEND_HZ, paced on the clock (catch-up, not one timer per event). The count that reaches the page is tested, not the count sent: the browser may merge events the page has not taken yet.
// load a: the pointer goes round an ellipse over the Playlist, the wheel scrolls it (one notch every 50 ms), and every 2 s three notches are sent with Control held (zoom; whether it zooms is not checked on the picture).
// load b: the audio output is started (a press on the audio status line, which is outside the desktop), then Space starts the transport and the metronome button of the guest's top bar is pressed: the Default project then makes sound, and the Mixer meters move. The start is checked (the non-silent quanta of the audio output rise). The pointer goes round an ellipse over the Mixer.
// Nothing in the product is changed or enabled for the measurement: no frame-rate flag, no URL parameter. The start is the Default project. The values are the runner's, not this machine's.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const USAGE = 'usage: node tools/p2-frames/measure.mjs <absolute path to the bundle directory raw/<name>> [--dpr <value>] [--load a|b] [--app <dir>]\n';
const argv = process.argv.slice(2);
const valueOf = (flag) => { const i = argv.indexOf(flag); return i < 0 ? null : argv[i + 1] ?? null; };
const dprText = valueOf('--dpr') ?? '1.7647';
const load = valueOf('--load') ?? 'a';
const appText = valueOf('--app');
const positional = argv.filter((a, i) => !a.startsWith('--') && !['--dpr', '--load', '--app'].includes(argv[i - 1]));
const DPR = Number(dprText);
if (positional.length !== 1 || !(DPR > 0) || !['a', 'b'].includes(load) || (argv.includes('--app') && (appText === null || appText.startsWith('--')))) {
  process.stderr.write(USAGE);
  process.exit(2);
}
const bundleDir = path.resolve(positional[0]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = appText === null ? path.join(repo, 'app') : path.resolve(appText);

const ROOM_W = 1836, ROOM_H = 1056;
// the window that gives the room above at the first layout (the page's right panel is 340 px wide and its header 36 px high; the fit below corrects a difference)
const VIEW_START = { width: 2176, height: 1092 };
const WINDOW_S = 30;
const SAMPLE_MS = 1000;
const GRACE_MS = 1500;
// the pointer is sent above 1000 Hz (1000 is the least the input must reach): one trial at 1000 gave a least second of 915 (the browser merged some events; the sampling was not yet exact), so the margin is 200 Hz
const SEND_HZ = 1200;
const REV_PER_S = 2;
// the pointer's ellipse, as fractions of the room: the Playlist (right, middle) for load a, the Mixer (bottom left) for load b
const AREA = { a: { cx: 0.78, cy: 0.55, rx: 0.05, ry: 0.05 }, b: { cx: 0.22, cy: 0.85, rx: 0.05, ry: 0.05 } }[load];
const WHEEL_MS = 50, ZOOM_MS = 2000, ZOOM_NOTCHES = 3;
const RECORD_POLL_MS = 100;
const FRAME_BUDGET_MS = 1000 / 120;
const READY_LIMIT_MS = 180000, IDLE_LIMIT_MS = 30000, CDP_TIMEOUT_MS = 20000, POLL_MS = 250;
const AUDIO_WAIT_MS = 5000, PLAY_WAIT_MS = 5000;
// the metronome button of the guest's top bar, in CSS px from the desktop's corner (the top bar is drawn at the same size in CSS px at every ratio: the guest's DPI follows the device pixel ratio, so the button is at DPR x these px in guest device px). Pressing it makes the Default project audible while it plays
const METRONOME = { x: 750, y: 19 };

// the isolation flags of tools/boot-check/boot-bundle.mjs (copied: that file does not export them). No frame-rate flag: the display's own rate limits the frames, as on a real screen.
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

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, error: a.error ? String(a.error.message ?? a.error).slice(0, 300) : null }; })()`;
// the room is the stage (app/src/main.ts measureRoom); the pointer goes to the desktop's box, the one the engine maps the pointer on (its view rect)
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); const r = document.getElementById('desktop').getBoundingClientRect(); const a = window.__app; return { sw: s.width, sh: s.height, x: r.x, y: r.y, w: r.width, h: r.height, iw: window.innerWidth, ih: window.innerHeight, dpr: window.devicePixelRatio, guest: a && a.screen ? a.screen.guest : null }; })()`;
const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
// the window list of the compositor (the dump composes from the engine's memory; this is read before the measured span only)
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const PM_INSTALL = `(() => { window.__pm = { n: 0 }; document.addEventListener('pointermove', (e) => { window.__pm.n += Math.max(1, e.getCoalescedEvents ? e.getCoalescedEvents().length : 1); }, true); return true; })()`;
// one row per record of the page's latency tracker: [seq, tPage, tEvent, dropped, answer's present time (-1: none), outcome]
const RECORDS = `(() => { const L = window.__app.latency.records; const out = []; for (const r of L) { let at = -1; for (const f of Object.values(r.answers)) if (at < 0 || f.at < at) at = f.at; out.push([r.seq, r.tPage, r.tEvent, r.dropped !== null, at, r.outcome]); } return out; })()`;
const NOW = `(performance.timeOrigin + performance.now())`;
// the page's own input number (decision 0202, eventToRaf over its last 20 inputs) and the engine's cumulative compose and put (the definition of the #1308 table: totals over the presents since the engine started)
const PAGE_NUMBERS = `(() => { const a = window.__app; const d = a.deskStats || {}; const e = a.perf.snapshot().input.eventToRaf; return { compose: d.composeMs ?? null, put: d.putMs ?? null, presents: d.presents ?? null, eventN: e.n, eventP50: e.p50Ms, eventP95: e.p95Ms }; })()`;
// the page's clock, the pointer count and the engine's presents read at one instant (one evaluation), so that a second's count is exact
const TICK = `(() => { const s = window.__app.deskStats; return { t: performance.now(), pm: window.__pm ? window.__pm.n : null, presents: s && typeof s.presents === 'number' ? s.presents : null }; })()`;
// the presenter's cumulative counters (the page's deskStats: the render Worker's 'desk' report, at most every 100 ms); read at the window's two ends, the window's per-present values are their differences
const DESK = `(() => { const s = window.__app.deskStats; if (!s) return null; const o = {}; for (const k of ['presents', 'rects', 'pixels', 'whole', 'composeMs', 'putMs', 'uploads', 'uploadBytes', 'draws', 'drawMs', 'drawPixels', 'texAllocs']) o[k] = typeof s[k] === 'number' ? s[k] : 0; return o; })()`;
const AUDIO = `(() => { const a = window.__app; return a && a.audio ? a.audio.detail() : null; })()`;
const AUDIO_AT = `(() => { const r = document.getElementById('audio-status').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
// nearest rank, as the page's own percentile (app/src/ui/latency.ts percentile)
const percentile = (sorted, p) => (sorted.length === 0 ? null : sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1))]);
const r3 = (x) => (x === null ? null : Math.round(x * 1000) / 1000);
const r2 = (x) => (x === null ? null : Math.round(x * 100) / 100);

function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { cls: names ? names[1] : '', title: names ? names[2] : '', x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  });
}

async function main() {
  const out = { error: null, dpr: DPR, load, presentedPerSec: null, minWindowFps: null, inputDropped: null, cpuPerPresentMsP50: null, cpuPerPresentMsP95: null, budgetRatioP95: null, inputToPresentMsP50: null, inputToPresentMsP95: null, inputRateOk: null, valid: false, desk: null, notes: '' };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-p2-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null;
  let notes = {};
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
    const B = (method, params, ms = CDP_TIMEOUT_MS) => timed(cdp.send(method, params), ms, method);
    await S('Page.enable');
    await S('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: DPR, mobile: false });
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);

    // the engine's idle: returns the state when it is idle (or after the limit); throws when the engine stopped
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
    const first = await waitIdle(READY_LIMIT_MS);
    if (first !== 'engine-idle') throw new Error(`no engine-idle within ${READY_LIMIT_MS} ms (state ${first ?? 'window.__app not set'})`);

    // the room: 1836 x 1056 CSS px (the stage), by the window's size: the stage is the window less the page's own panel and header, so the window is set from the difference
    let room = null;
    for (let i = 0; i < 4 && room === null; i++) {
      const r = await ev(ROOM);
      if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) room = r;
      else {
        log('room', { sw: r.sw, sh: r.sh, iw: r.iw, ih: r.ih });
        await S('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: DPR, mobile: false });
        await sleep(1500);
      }
    }
    if (room === null) throw new Error(`the room is not ${ROOM_W} x ${ROOM_H} CSS px`);
    log('room', { sw: room.sw, sh: room.sh, desktop: [room.w, room.h], dpr: room.dpr, guest: room.guest });

    const click = async (sx, sy, geo) => {
      const c = { x: geo.x + (sx + 0.5) * (geo.w / geo.gw), y: geo.y + (sy + 0.5) * (geo.h / geo.gh) };
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    };
    // the welcome wizard (and its usage-data question inside it), if the page shows it: its X, then wait until it is gone
    // the page refuses the window list while the picture is being shown ("ask again"): it is asked again
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
      await click(wiz.x + wiz.w - 18, wiz.y + 18, await ev(GEOMETRY));
      const t0 = Date.now();
      while ((await windowsNow()).some((w) => w.cls === 'TWelcomeWizard')) {
        if (Date.now() - t0 >= 15000) throw new Error('the welcome wizard did not close');
        await sleep(POLL_MS);
      }
    }
    // the project's own warning after the start (the plugins and samples it names could not be found): a modal dialog that takes the keys and the pointer until its OK is pressed; the OK is found by the dialog's window (title) and pressed at its place (the same fraction of the window at every ratio)
    const WARN_TITLE = 'Problems loading the project';
    const warnNow = async () => (await windowsNow()).find((w) => w.title === WARN_TITLE);
    for (let i = 0; i < 40 && await warnNow(); i++) {
      const w = await warnNow();
      log('the warning dialog is open; its OK is pressed', { x: w.x, y: w.y, w: w.w, h: w.h });
      await click(w.x + Math.round(w.w * 0.534), w.y + Math.round(w.h * 0.916), await ev(GEOMETRY));
      await sleep(POLL_MS);
    }
    if (await warnNow()) throw new Error(`the dialog "${WARN_TITLE}" did not close`);
    await sleep(1500);
    if ((await waitIdle(IDLE_LIMIT_MS)) !== 'engine-idle') throw new Error('the engine did not return to engine-idle after the wizard');
    await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");

    const tap = async (code, key, vk) => {
      const text = code === 'Space' ? ' ' : undefined;
      await S('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, ...(text ? { text } : {}) });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    };
    if (load === 'b') {
      // the audio output starts with a gesture of the page's own (the status line is outside the desktop: the guest gets no event)
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
      if (!(await running(AUDIO_WAIT_MS))) {
        await tap('ShiftLeft', 'Shift', 16);
        if (!(await running(AUDIO_WAIT_MS))) throw new Error('the audio output did not start (no gesture took)');
      }
      // the transport: Space starts it (the default project's pattern); the metronome button of the guest's top bar is then pressed, so the
      // default project makes sound (its clicks), and the Mixer's meters move. Without it the default project is silent (nonSilentQuanta stays 0)
      await ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
      await tap('Space', ' ', 32);
      await sleep(500);
      await click(Math.round(METRONOME.x * DPR), Math.round(METRONOME.y * DPR), await ev(GEOMETRY));
      const n0 = (await ev(AUDIO)).nonSilentQuanta;
      const t0 = Date.now();
      for (;;) {
        const n1 = (await ev(AUDIO)).nonSilentQuanta;
        if (n1 > n0 + 20) break;
        if (Date.now() - t0 >= PLAY_WAIT_MS) throw new Error(`the playback is silent (nonSilentQuanta ${n0} -> ${n1} in ${PLAY_WAIT_MS} ms): no meter moves`);
        await sleep(POLL_MS);
      }
      log('playing with sound', { nonSilentQuanta: await ev(`window.__app.audio.detail().nonSilentQuanta`) });
      await sleep(1000);
    }

    // ---- the measured span: WINDOW_S seconds ----
    const st = { stopInput: false, finished: false, pointerSent: 0, wheelSent: 0, zooms: 0 };
    const inputs = new Map();
    let drawAndSwap = 0;
    cdp.on((m) => { if (m.method === 'Tracing.dataCollected') for (const e of m.params.value) if (e.name === 'Display::DrawAndSwap') drawAndSwap++; });
    const tracingDone = new Promise((res) => cdp.on((m) => { if (m.method === 'Tracing.tracingComplete') res(); }));
    await ev(PM_INSTALL);
    // the CPU of the renderer process (cpuTime, seconds) and of its threads by name (/proc: utime + stime, clock ticks of 1/100 s): the page's main thread, the Workers (engine, render) and the compositor
    const cpuNow = async () => {
      const { processInfo } = await B('SystemInfo.getProcessInfo', undefined);
      const sum = (type) => processInfo.filter((p) => p.type === type).reduce((a, p) => a + p.cpuTime, 0);
      const threads = new Map();
      for (const p of processInfo.filter((x) => x.type === 'renderer')) {
        let tids = [];
        try { tids = fs.readdirSync(`/proc/${p.id}/task`); } catch { continue; }
        for (const tid of tids) {
          try {
            const comm = fs.readFileSync(`/proc/${p.id}/task/${tid}/comm`, 'utf8').trim();
            const stat = fs.readFileSync(`/proc/${p.id}/task/${tid}/stat`, 'utf8');
            const f = stat.slice(stat.lastIndexOf(')') + 2).split(' ');   // f[0] is field 3 (state): utime is field 14 (f[11]), stime field 15 (f[12])
            threads.set(comm, (threads.get(comm) ?? 0) + Number(f[11]) + Number(f[12]));
          } catch { /* the thread ended */ }
        }
      }
      return { renderer: sum('renderer'), gpu: sum('GPU'), threads };
    };
    const center = (A) => ({ x: room.x + A.cx * room.w, y: room.y + A.cy * room.h });
    const pointer = (i) => {
      const th = 2 * Math.PI * REV_PER_S * (i / SEND_HZ);
      return { x: room.x + (AREA.cx + AREA.rx * Math.cos(th)) * room.w, y: room.y + (AREA.cy + AREA.ry * Math.sin(th)) * room.h };
    };
    const pointerLoop = async () => {
      const t0 = performance.now();
      let sent = 0;
      while (!st.stopInput) {
        const due = Math.floor(((performance.now() - t0) * SEND_HZ) / 1000);
        while (sent < due) {
          const p = pointer(sent);
          cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y, button: 'none', buttons: 0, pointerType: 'mouse' }, sessionId).catch(() => {});
          sent++;
        }
        await sleep(2);
      }
      st.pointerSent = sent;
    };
    const wheel = (x, y, deltaY) => S('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: 0, deltaY, button: 'none', buttons: 0 });
    const wheelLoop = async () => {
      if (load !== 'a') return;
      const c = center(AREA);
      let k = 0, lastZoom = performance.now();
      while (!st.stopInput) {
        if (performance.now() - lastZoom >= ZOOM_MS) {
          lastZoom = performance.now();
          st.zooms++;
          await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17 });
          for (let n = 0; n < ZOOM_NOTCHES; n++) { await wheel(c.x, c.y, -120); await sleep(30); st.wheelSent++; }
          await S('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17 });
        } else {
          await wheel(c.x, c.y, k % 2 === 0 ? 120 : -120);
          st.wheelSent++;
          k++;
        }
        await sleep(WHEEL_MS);
      }
    };
    const pollRecords = async () => {
      for (const [seq, tPage, tEvent, dropped, answerAt, outcome] of await ev(RECORDS)) {
        const cur = inputs.get(seq);
        if (!cur) inputs.set(seq, { tPage, tEvent, dropped, answerAt, outcome });
        else { cur.dropped = dropped; cur.outcome = outcome; if (cur.answerAt < 0 && answerAt >= 0) cur.answerAt = answerAt; }
      }
    };
    const recordLoop = async () => {
      while (!st.finished) { await pollRecords().catch(() => {}); await sleep(RECORD_POLL_MS); }
      await pollRecords().catch(() => {});
    };

    const tr0 = performance.now();
    await B('Tracing.start', { categories: 'viz', transferMode: 'ReportEvents' });
    const samples = [];
    const tWin0 = await ev(NOW);
    const desk0 = await ev(DESK);
    const silent0 = (await ev(AUDIO))?.nonSilentQuanta ?? null;
    const pointerP = pointerLoop();
    const wheelP = wheelLoop();
    const recordP = recordLoop();
    const t0 = performance.now();
    for (let k = 0; k <= WINDOW_S; k++) {
      if (k > 0) await sleep(Math.max(0, t0 + k * SAMPLE_MS - performance.now()));
      const cpu = await cpuNow();
      samples.push({ ...(await ev(TICK)), cpu });
    }
    const tWin1 = await ev(NOW);
    const desk1 = await ev(DESK);
    const silent1 = (await ev(AUDIO))?.nonSilentQuanta ?? null;
    st.stopInput = true;
    await B('Tracing.end');
    const traceSeconds = (performance.now() - tr0) / 1000;
    await Promise.race([tracingDone, sleep(15000)]);
    await Promise.all([pointerP, wheelP]);
    await sleep(GRACE_MS);
    st.finished = true;
    await recordP;
    const page = await ev(PAGE_NUMBERS);

    // ---- the numbers ----
    const seconds = samples.slice(1).map((s, i) => ({ dt: (s.t - samples[i].t) / 1000, pm: s.pm - samples[i].pm, presents: s.presents - samples[i].presents, cpuMs: (s.cpu.renderer - samples[i].cpu.renderer) * 1000 }));
    const rates = seconds.map((s) => s.pm / s.dt);
    const minRate = Math.min(...rates), medRate = percentile([...rates].sort((a, b) => a - b), 50);
    const inputRateOk = rates.length === WINDOW_S && minRate >= 1000;
    const per = seconds.filter((s) => s.presents > 0).map((s) => s.cpuMs / s.presents).sort((a, b) => a - b);
    const presentsTotal = samples[samples.length - 1].presents - samples[0].presents;
    const cpuTotal = (samples[samples.length - 1].cpu.renderer - samples[0].cpu.renderer) * 1000;
    const gpuTotal = (samples[samples.length - 1].cpu.gpu - samples[0].cpu.gpu) * 1000;
    const winSeconds = (tWin1 - tWin0) / 1000;
    const inWin = [...inputs.values()].filter((r) => r.tPage >= tWin0 && r.tPage <= tWin1);
    const answered = inWin.filter((r) => r.answerAt >= r.tPage && !r.dropped);
    const lat = answered.map((r) => r.answerAt - (r.tEvent ?? r.tPage)).filter((x) => x >= 0).sort((a, b) => a - b);
    const droppedN = inWin.filter((r) => r.dropped).length;
    const presentedPerSec = drawAndSwap / traceSeconds;
    // #1422: the least one-second window of the span (the presents of each second over its length)
    const windowFps = samples.every((s) => typeof s.presents === 'number') ? seconds.map((s) => s.presents / s.dt) : [];
    out.minWindowFps = windowFps.length > 0 ? r2(Math.min(...windowFps)) : null;
    out.inputDropped = droppedN;
    const CLK_TCK = 100;
    const threadMs = [...samples[samples.length - 1].cpu.threads].map(([comm, v]) => [comm, ((v - (samples[0].cpu.threads.get(comm) ?? 0)) / CLK_TCK) * 1000]).sort((a, b) => b[1] - a[1]);
    const threadsText = threadMs.slice(0, 6).map(([comm, ms]) => `${comm} ${ms.toFixed(0)} ms (${(ms / Math.max(1, presentsTotal)).toFixed(2)} ms per present)`).join(', ');
    // the work per present, from the presenter's counters over the window (the breakdown of #1351): rectangles, uploaded pixels and bytes, draws, and the time of compose, put and draw; the threads' CPU per present (main thread, engine and render Workers, compositor, GPU process)
    const dp = desk0 && desk1 ? desk1.presents - desk0.presents : 0;
    const deskPer = (k) => (dp > 0 ? (desk1[k] - desk0[k]) / dp : null);
    const threadPer = (name) => { const e = threadMs.find(([c]) => c === name); return e && presentsTotal > 0 ? r3(e[1] / presentsTotal) : null; };
    const screenPixels = room.guest.width * room.guest.height;
    out.desk = dp > 0 ? {
      presents: dp, screenPixels,
      rectsPerPresent: r3(deskPer('rects')), pixelsPerPresent: r3(deskPer('pixels')), pixelShare: r3(deskPer('pixels') / screenPixels),
      uploadsPerPresent: r3(deskPer('uploads')), uploadBytesPerPresent: r3(deskPer('uploadBytes')),
      drawsPerPresent: r3(deskPer('draws')), drawMsPerPresent: r3(deskPer('drawMs')), drawPixelsPerPresent: r3(deskPer('drawPixels')), drawPixelShare: r3(deskPer('drawPixels') / screenPixels),
      composeMsPerPresent: r3(deskPer('composeMs')), putMsPerPresent: r3(deskPer('putMs')),
      wholePerWindow: desk1.whole - desk0.whole, texAllocsPerWindow: desk1.texAllocs - desk0.texAllocs,
      chromeThreadMsPerPresent: threadPer('chrome'), dedicatedWorkerMsPerPresent: threadPer('DedicatedWorker'), compositorMsPerPresent: threadPer('Compositor'),
      gpuProcessMsPerPresent: presentsTotal > 0 ? r3(gpuTotal / presentsTotal) : null,
    } : null;
    out.presentedPerSec = r2(presentedPerSec);
    out.cpuPerPresentMsP50 = r3(percentile(per, 50));
    out.cpuPerPresentMsP95 = r3(percentile(per, 95));
    out.budgetRatioP95 = out.cpuPerPresentMsP95 === null ? null : r3(out.cpuPerPresentMsP95 / FRAME_BUDGET_MS);
    out.inputToPresentMsP50 = r3(percentile(lat, 50));
    out.inputToPresentMsP95 = r3(percentile(lat, 95));
    out.inputRateOk = inputRateOk;
    const finite = [out.presentedPerSec, out.cpuPerPresentMsP50, out.cpuPerPresentMsP95, out.budgetRatioP95, out.inputToPresentMsP50, out.inputToPresentMsP95].every((x) => typeof x === 'number' && Number.isFinite(x));
    out.valid = finite && drawAndSwap > 0 && presentsTotal >= 1 && inputRateOk === true;
    notes = {
      frames: `the frames counted are Display::DrawAndSwap of the trace category viz in the ${traceSeconds.toFixed(2)} s window: ${drawAndSwap} (${presentedPerSec.toFixed(2)}/s). Cross-check, the engine's own presents: ${presentsTotal} (${(presentsTotal / winSeconds).toFixed(2)}/s). The display's rate limit of headless Chrome is in force (no frame-rate flag): above that rate nothing can be counted; the 120 fps verdict is not made here (not measured on a display).`,
      cpu: `cpuPerPresentMs: renderer process CPU (cpuTime of SystemInfo.getProcessInfo) in each of ${seconds.length} seconds over that second's engine presents; ${per.length} seconds had a present; mean ${(cpuTotal / Math.max(1, presentsTotal)).toFixed(3)} ms per present over the window (${cpuTotal.toFixed(0)} ms / ${presentsTotal}). GPU process CPU in the window: ${gpuTotal.toFixed(0)} ms (${(gpuTotal / Math.max(1, presentsTotal)).toFixed(3)} ms per present; not in cpuPerPresentMs).`,
      threads: `renderer threads by name over the window (the sum is the renderer's CPU above): ${threadsText}.`,
      windows: `minWindowFps ${out.minWindowFps}: the least of the ${seconds.length} one-second windows (presents in the second / its length; the page's deskStats, read once a second). inputDropped ${droppedN} (the window's inputs the page marks as dropped: the engine's input queue full, or merged into a later move).`,
      sound: `the audio output's non-silent quanta: ${silent0} -> ${silent1} in the window${silent0 !== null && silent1 !== null && silent1 > silent0 ? ' (the output was not silent)' : ' (silent: no meter moved)'}.`,
      input: `the pointer was sent at ${SEND_HZ}/s nominal on an ellipse, ${REV_PER_S} turns a second: ${st.pointerSent} sent. The page received (pointermove, getCoalescedEvents) ${seconds.reduce((a, s) => a + s.pm, 0)} in the window: per second minimum ${minRate.toFixed(0)}, median ${medRate.toFixed(0)}; inputRateOk ${inputRateOk} (every one of the ${WINDOW_S} seconds at 1000 or more). Inputs the page sent to the engine in the window ${inWin.length}; answered by a present ${answered.length}; dropped by the engine ${droppedN}; latency from the DOM event timeStamp to the present that answered it.`,
      load: load === 'a'
        ? `load a: the wheel scrolls the Playlist (one notch every 50 ms, ${st.wheelSent} notches sent with the zoom bursts), ${st.zooms} Control+wheel bursts (zoom; whether the picture zoomed is not checked).`
        : `load b: playback (Space) with the metronome on (its button in the top bar, at ${METRONOME.x},${METRONOME.y} CSS px = guest px ${Math.round(METRONOME.x * DPR)},${Math.round(METRONOME.y * DPR)}), so the Default project makes sound and the Mixer meters move; the non-silent quanta rose during the start check; no wheel.`,
      room: `room (stage) ${room.sw}x${room.sh} CSS px at device pixel ratio ${room.dpr} (the desktop box ${room.w.toFixed(1)}x${room.h.toFixed(1)}); guest screen ${room.guest.width}x${room.guest.height} device px. Start: the Default project. Chrome ${chrome.version.Browser}.`,
      page: `the page's own input number (decision 0202, eventToRaf: DOM event timeStamp to the next animation frame after the present) over its last ${page.eventN} answered inputs at the end: p50 ${page.eventP50} ms, p95 ${page.eventP95} ms. The engine's cumulative compose ${(page.compose / Math.max(1, page.presents)).toFixed(3)} ms and put ${(page.put / Math.max(1, page.presents)).toFixed(3)} ms per present (deskStats totals over ${page.presents} presents since the engine started, the #1308 table's definition).`,
      runner: `nproc ${os.availableParallelism?.() ?? os.cpus().length}, loadavg ${os.loadavg().map((x) => x.toFixed(2)).join(' ')} (at the end of the run).`,
    };
    out.notes = Object.values(notes).join(' ');
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
    out.valid = false;
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

await main();
