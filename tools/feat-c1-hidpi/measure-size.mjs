// feat-c1-hidpi/measure-size: the guest's screen, its DPI and the canvas at each device pixel ratio (Issue #1347). Boots the bundle in headless Chrome as the page is served
// (the page's room, and FLE_MONITOR_W / H / DPR in the engine's environment) once for each ratio, waits for the first engine-idle, and prints ONE JSON line per run on stdout.
// The last line is the states of DPR 2.5 and 3 ("highDpr"); a stop there is a result, not a failure (the exit code does not count it).
//
// The ratios: --dprs <list> (default 1,1.25,1.5,1.7647,2: only the inputs of the runs; the expected values are never computed from this list), --random <count> (default 2),
// --random-range <min,max> (default 1.00,2.25). The random values are multiples of 0.01 in the range, drawn from a seed that crypto.randomBytes gives on each start (mulberry32).
// The range's top stays below the DPI at which the engine stops (#1354: DPR 2.5 and more, DPI 240 and more).
//
// What each value is (a null is a value this tool does not measure; the acceptance reads only the others):
//  - dpr: window.devicePixelRatio. randomDprs: the seed and the values (the same in every line of one start). room: window.__app.screen.room (the page's room in CSS pixels).
//  - state, standIn, error: the page (window.__app.state and error; the "STAND-IN" text).
//  - guest: the screen the ENGINE reports (window.__app.screen.reported: GetSystemMetrics SM_CXSCREEN / SM_CYSCREEN of the run).
//  - dpi: LOGPIXELSY of GetDeviceCaps (index 90), from the window-API trace. The guest reads only the Y value in this build; X (88) is used when Y is absent.
//    api: the DPI answers of the trace (LOGPIXELSX / Y, HORZRES / VERTRES, the GetSystemMetrics values of the screen, icons and cursors, GetDpiForMonitor, GetProcessDpiAwareness).
//    The trace is on from the start: window.__app.windowApiTrace(true) runs as soon as the page exists, and the start takes it (main.ts apiTraceWanted; fruity.ts traceOn).
//  - canvas: canvas[data-desk="1"] width and height (the drawing buffer = the guest's screen in device pixels).
//  - display: the canvas's getBoundingClientRect x dpr, rounded (the device pixels the canvas covers). The acceptance wants it to match canvas within 1 pixel.
//  - asked: the size and the environment the page passed to the engine (window.__app.screen.asked).
//  - scale (#1353): the picture (window.__app.deskDumpBase64) and the window table. flScalingPercent = round(100 x the Welcome window's width / its width at the DPR 1 reference of the
//    calibration boot): FL's own window, the only one of its size in the table that does not move. scale.tempoLen: the Tempo box's bright interior in guest pixels (DPR 1: 54), a cross-check.
//    The Tempo box is the longest run of bright interior pixels in the top band (x within 0.2 W .. 0.3 W); the scale of the other parts is its ratio to the DPR 1 reference.
//  - edgeWidth: the 10 % to 90 % width of the transitions at the four edges of the Tempo box (the median of the four; pixels).
//  - parts: the Tempo box's frame is measured; "song-position" and "pattern-selector" are NOT measured (measured: false): their DPR 1 offsets from the Tempo frame, scaled by the Tempo ratio.
//  - pointerErrPx: presses (move, press, release) at three points of the Welcome window's title bar, and the guest pixel the engine received ("hit: down ... ptr=x,y", the trace).
//    The presses are on the Welcome window's title bar (it is open at the start; the main form is not pressed).
//    The error is max |received - floor((clientX - canvas left) x dpr)| over the three points and both axes.
//    pointerSentErrPx: the same error from the page's own record of the presses it sent (app.latency.records: "button L down at screen (x, y)"), which needs no answer from the engine.
//    It is a supplement: pointerErrPx (the engine's answer) is the acceptance's value.
//    pointerDiag: the page's input count (__app.inputEvents) and the log's line count before and after the presses, the element under each press, the log's last lines.
//  - engineFaults: the guest thread failures ("fle thread N failed") in the engine's log before the pointer test. A failed guest thread stops the engine's thread switch (fle_thread_lib.js fail:
//    nothing is handed over), so the engine answers no press while the page still shows "engine-idle": such a row has no "hit" line (pointerErrPx null). Not retried: one boot per row.
//  - mainGuiScaling: FL's last registry read of a value whose name has "scaling", "GUI" or "uiscale" (window-api: RegQueryValueExW, from the trace), { found, value, line }; null when FL read none.
//    The trace of the start-up has no such read (the General settings are read when the settings window opens; #1353).
//  - debugLog: { captured: false } here. The Debug tab is not reached: the Welcome window closes only by No thanks or Escape, and both trap the engine (HostFilesUnsupported).
//  - NOT MEASURED here (null): blackShare (the Browser's rectangle is #1355's).
//  - the room: the calibration. The viewport is set (Emulation.setDeviceMetricsOverride) so that the room is ROOM_CSS (1836 x 1056). The first boot (DPR 1) reads the room the page
//    used; the viewport is corrected by the difference and booted again (at most three boots; the layout outside the stage is assumed to be a fixed offset, and the boots check it).
//
// Exit 1 when a run failed ("error" set), 2 on a usage error, 0 otherwise (a row can still hold nulls: the acceptance reads them).
// Usage: node tools/feat-c1-hidpi/measure-size.mjs <absolute path to the bundle directory raw/<name>> [--dprs <list>] [--random <count>] [--random-range <min,max>] > size.jsonl
// (needs app/node_modules and app/dist; the sign of a clean run is in the acceptance)
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const FIXED_DPRS = [1, 1.25, 1.5, 1.7647, 2];
const DEFAULT_RANDOM = { count: 2, range: [1, 2.25] };
const HIGH_DPRS = [2.5, 3];
const ROOM_CSS = { w: 1836, h: 1056 };
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const SETTLE_S = 3;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same launch as tools/feat-c1-hidpi/measure.mjs)
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

// the options: each list is comma-separated; a value that is not a positive number, or a count that is not a whole number, is a usage error
export function parseArgs(argv) {
  const opts = { bundleDir: '', dprs: FIXED_DPRS, random: DEFAULT_RANDOM.count, range: DEFAULT_RANDOM.range };
  const list = (s, what) => {
    const v = String(s ?? '').split(',').map((x) => x.trim()).filter((x) => x !== '').map(Number);
    if (v.length === 0 || v.some((x) => !Number.isFinite(x) || x <= 0)) throw new Error(`${what}: not a list of positive numbers: ${s}`);
    return v;
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dprs') opts.dprs = list(argv[++i], '--dprs');
    else if (a === '--random') {
      const n = Number(argv[++i]);
      if (!Number.isInteger(n) || n < 0) throw new Error(`--random: not a count: ${argv[i]}`);
      opts.random = n;
    } else if (a === '--random-range') {
      const r = list(argv[++i], '--random-range');
      if (r.length !== 2 || r[0] > r[1]) throw new Error(`--random-range: the minimum and the maximum: ${argv[i]}`);
      opts.range = r;
    } else if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
    else if (opts.bundleDir === '') opts.bundleDir = path.resolve(a);
    else throw new Error(`one bundle directory: ${a}`);
  }
  return opts;
}

// the random ratios: a seed (from the system's random bytes on each start unless given), then mulberry32; each value is a multiple of 0.01 in the range,
// and is drawn without repeat: not one of the fixed ratios (a repeat would make two rows of one ratio, and the acceptance counts the distinct ratios)
export function randomDprs(count, [min, max], seedIn = crypto.randomBytes(4).readUInt32BE(0), fixed = []) {
  let s = seedIn >>> 0;
  const next = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const lo = Math.round(min * 100), hi = Math.round(max * 100);
  const pool = [];
  for (let k = lo; k <= hi; k++) if (!fixed.some((f) => Math.abs(f - k / 100) < 1e-9)) pool.push(k / 100);
  if (count > pool.length) throw new Error(`--random: ${count} values do not fit in the range ${min}..${max} without the fixed ratios`);
  const values = Array.from({ length: count }, () => pool.splice(Math.floor(next() * pool.length), 1)[0]);
  return { seed: seedIn >>> 0, values };
}

// The picture of the guest (deskDumpBase64: RGBA, rows of `stride` bytes) and the analyses of it. Only the picture and the window list are read; nothing is drawn or compared with an expected picture.
const lumAt = (b, stride, x, y) => { const i = y * stride + 4 * x; return 0.299 * b[i] + 0.587 * b[i + 1] + 0.114 * b[i + 2]; };
const BRIGHT_LO = 175, BRIGHT_HI = 215;

/** the Tempo box: the longest run of bright interior pixels (the LCD's background) in the top band of the picture, x within [0.2 W, 0.3 W] (at DPR 1 it is guest x 437..490, y 13) */
export function tempoBox(b, stride, w, h) {
  const xa = Math.floor(0.2 * w), xb = Math.floor(0.3 * w), yb = Math.floor(0.04 * h);
  let best = null;
  for (let y = 0; y < yb; y++) {
    let run = 0, start = -1;
    for (let x = xa; x <= xb; x++) {
      const l = lumAt(b, stride, x, y);
      if (l >= BRIGHT_LO && l <= BRIGHT_HI) {
        if (run === 0) start = x;
        run++;
        if (best === null || run > best.len) best = { len: run, x0: start, x1: x, y };
      } else run = 0;
    }
  }
  return best;
}

/** the 10 % to 90 % width (pixels) of a transition in a profile that runs from the outside (first samples) to the inside (last samples); null when there is no transition */
export function transitionWidth(prof) {
  const lo = (prof[0] + prof[1]) / 2, hi = (prof[prof.length - 2] + prof[prof.length - 1]) / 2;
  if (!(hi - lo >= 40)) return null;
  const at = (t) => {
    const v = lo + t * (hi - lo);
    for (let i = 1; i < prof.length; i++) {
      const a = prof[i - 1], c = prof[i];
      if ((a - v) * (c - v) <= 0 && a !== c) return i - 1 + (v - a) / (c - a);
    }
    return null;
  };
  const p10 = at(0.1), p90 = at(0.9);
  return p10 === null || p90 === null ? null : Math.abs(p90 - p10);
}

/** the frame edges of the Tempo box: the widths (median of the horizontal and the vertical ones) of the transitions at its four edges, and the outer box (the frame's top-left) */
export function tempoEdges(b, stride, box) {
  const L = (x, y) => lumAt(b, stride, x, y);
  const row = box.y, xc = box.x0 + 1;
  let top = row;
  while (top > 0 && L(xc, top) >= BRIGHT_LO) top--;
  let bot = row;
  while (L(xc, bot) >= BRIGHT_LO && bot < box.y + 200) bot++;
  const yTop = top + 1, yBot = bot - 1;
  const run = (a, c) => Array.from({ length: Math.abs(c - a) + 1 }, (_, i) => (c >= a ? a + i : a - i));
  const widths = [
    transitionWidth(run(box.x0 - 4, box.x0 + 3).map((x) => L(x, row))),
    transitionWidth(run(box.x1 + 4, box.x1 - 3).map((x) => L(x, row))),
    transitionWidth(run(yTop - 4, yTop + 3).map((y) => L(xc, y))),
    transitionWidth(run(yBot + 4, yBot - 3).map((y) => L(xc, y))),
  ].filter((v) => v !== null);
  const sorted = widths.slice().sort((p, q) => p - q);
  const mid = sorted.length === 0 ? null : sorted.length % 2 ? sorted[(sorted.length - 1) / 2] : (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2;
  return { edgeWidth: mid, frame: { x: box.x0 - 1, y: yTop - 1, x1: box.x1 + 1, y1: yBot + 1 } };
}

// the DPR 1 boxes of the top bar's other parts (tools/topbar-parts/measure.mjs: box x0, y0, x1, y1 at DPR 1); their places are read from the Tempo box's frame, scaled by the Tempo's width
const DPR1_TEMPO = { x: 436, y: 8, w: 56, h: 25 };
const DPR1_PARTS = {
  'song-position': { x: 500, y: 4, w: 129, h: 33 },
  'pattern-selector': { x: 592, y: 48, w: 47, h: 28 },
};

/** the parts' places and centres in guest pixels: the Tempo box is measured in the picture; the others are its DPR 1 offsets scaled by the Tempo's scale */
export function partPlaces(frame, s) {
  const place = (p) => {
    const x = frame.x + (p.x - DPR1_TEMPO.x) * s, y = frame.y + (p.y - DPR1_TEMPO.y) * s, w = p.w * s, h = p.h * s;
    return { x: Math.round(x), y: Math.round(y), cx: Math.round(x + w / 2), cy: Math.round(y + h / 2), measured: false };
  };
  const tw = (frame.x1 - frame.x) * 1, th = (frame.y1 - frame.y);
  return {
    tempo: { x: frame.x, y: frame.y, cx: Math.round(frame.x + tw / 2), cy: Math.round(frame.y + th / 2), measured: true },
    'song-position': place(DPR1_PARTS['song-position']),
    'pattern-selector': place(DPR1_PARTS['pattern-selector']),
  };
}

/** a window of the window table (window table: "lid N at (x, y) WxH ... class \"...\""): { x, y, w, h }, or null when it is not in the list */
export function windowRect(windows, cls) {
  for (const l of windows.split('\n')) {
    const m = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (m && m[6].includes(`class "${cls}"`)) return { x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]) };
  }
  return null;
}

/** FL's reading of the Main GUI Scaling setting: the last registry read (window-api: RegQueryValueExW) of a value whose name has "scaling", "GUI" or "uiscale" in it: { found, value, line }; null when FL read no such value */
export function mainGuiScalingRead(lines) {
  const hits = lines.map(String).filter((l) => /RegQueryValueExW "[^"]*" "[^"]*(scaling|gui|uiscale)[^"]*" -> /i.test(l));
  if (hits.length === 0) return null;
  const last = hits[hits.length - 1];
  const m = /value="([^"]*)"|value=(-?\d+)/.exec(last);
  return { found: /-> found/.test(last), value: m ? (m[1] ?? m[2]) : null, line: last.replace(/^\[err\] /, '').slice(0, 300) };
}

/** the pointer's answers of the engine's trace: the guest pixel the engine received (ptr=x,y) for each button-down line, in order (the last ones are the presses of this run: the engine log is a ring) */
export function pointerDowns(lines) {
  const out = [];
  for (const l of lines.map(String)) {
    const m = /hit: down .* ptr=(-?\d+),(-?\d+)/.exec(l);
    if (m) out.push({ x: Number(m[1]), y: Number(m[2]) });
  }
  return out;
}

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

const PAGE = `(() => {
  const a = window.__app ?? null;
  const c = document.querySelector('canvas[data-desk="1"]');
  const cr = c ? c.getBoundingClientRect() : null;
  const dpr = window.devicePixelRatio;
  return {
    dpr,
    state: a ? a.state : null,
    error: a && a.error ? a.error : null,
    standIn: document.body ? document.body.innerText.includes('STAND-IN') : null,
    room: a && a.screen && a.screen.room ? a.screen.room : null,
    asked: a && a.screen && a.screen.asked ? { width: a.screen.asked.width, height: a.screen.asked.height, env: a.screen.asked.env } : null,
    reported: a && a.screen && a.screen.reported ? a.screen.reported : null,
    canvas: c ? { w: c.width, h: c.height } : null,
    canvasRect: cr ? { x: cr.left, y: cr.top, w: cr.width, h: cr.height } : null,
  };
})()`;

const TRACE_ON = `(function () { const t = setInterval(() => { if (window.__app) { clearInterval(t); window.__app.windowApiTrace(true); } }, 5); })();`;

// the DPI answers of the run, read from the engine's trace lines (window-api: GetDeviceCaps / GetSystemMetrics / GetDpiForMonitor / GetProcessDpiAwareness):
// the first answer of each call; the indexes are the Windows constants (LOGPIXELSX 88, LOGPIXELSY 90, HORZRES 8, VERTRES 10; SM_CXICON 11, SM_CXSMICON 49, SM_CXCURSOR 13)
function dpiAnswers(lines) {
  const first = (re) => {
    for (const l of lines) {
      const m = re.exec(l);
      if (m) return Number(m[1]);
    }
    return null;
  };
  const dc = (i) => first(new RegExp(`window-api: GetDeviceCaps index=${i} -> (\\d+)`));
  const sm = (i) => first(new RegExp(`window-api: GetSystemMetrics index=${i} -> (\\d+)`));
  return {
    logpixelsX: dc(88), logpixelsY: dc(90), horzres: dc(8), vertres: dc(10),
    smCxScreen: sm(0), smCxIcon: sm(11), smCxSmIcon: sm(49), smCxCursor: sm(13),
    dpiForMonitorX: first(/window-api: GetDpiForMonitor type=\d+ -> (\d+)/),
    processAwareness: first(/window-api: GetProcessDpiAwareness -> (\d+)/),
    calls: lines.filter((l) => /window-api: (GetDeviceCaps|GetSystemMetrics|GetDpiForMonitor|GetProcessDpiAwareness)/.test(l)).length,
  };
}

async function openPage(chrome, dpr, viewport) {
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const S = (method, params) => timed(cdp.send(method, params, sessionId), 30000, method);
  const ev = async (expression) => {
    const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.text}`);
    return r.result.value;
  };
  await S('Page.enable');
  await S('Emulation.setDeviceMetricsOverride', { width: viewport.w, height: viewport.h, deviceScaleFactor: dpr, mobile: false });
  // the window-API trace on from the engine's first instruction: the page turns it on as soon as window.__app exists, and the start takes the request
  await S('Page.addScriptToEvaluateOnNewDocument', { source: TRACE_ON });
  return { cdp, S, ev };
}

let runNo = 0;
// the DPR 1 reference of the scale (the calibration boot): the Welcome window's width and the Tempo box's interior width at 96 DPI
let REF = null;

// one press (move, press, release) at a CSS point; the engine gets the pointer as the page's events say (the pointer path of the page, not the engine's)
async function cdpPress(S, x, y) {
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
  await sleep(60);
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(50);
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(300);
}

// the picture measures of one boot: the scale (the Welcome window and the Tempo box against the DPR 1 reference), the frame's edge width, the parts' places
async function pictureMeasures(ev) {
  const d = await ev(`window.__app.deskDumpBase64(true)`);
  const b = Buffer.from(d.base64, 'base64');
  const stride = b.length / d.height;
  const box = tempoBox(b, stride, d.width, d.height);
  const welcome = windowRect(d.windows, 'TWelcomeWizard');
  const welcomeW = welcome ? welcome.w : null;
  if (!box) return { welcomeW, welcome, tempo: null, edgeWidth: null, parts: null, scale: null };
  const edges = tempoEdges(b, stride, box);
  const tempoLen = box.len;
  const s = REF && REF.tempoLen ? tempoLen / REF.tempoLen : null;
  return {
    welcomeW,
    welcome,
    tempo: { len: tempoLen, x0: box.x0, x1: box.x1, y: box.y, frame: edges.frame },
    edgeWidth: edges.edgeWidth,
    parts: s === null ? null : partPlaces(edges.frame, s),
    scale: s,
  };
}

// one boot at one ratio. The row's state is the last state the page showed ("engine-idle" when the engine reached its first idle); a stop shows as another state or as no idle within the limit
async function runOnce(dpr, viewport, port, root, rnd) {
  const out = { dpr, randomDprs: rnd, room: null, state: null, standIn: null, error: null, guest: null, dpi: null, api: null, asked: null,
    flScalingPercent: null, mainGuiScaling: null, parts: null, canvas: null, display: null, edgeWidth: null, blackShare: null,
    debugLog: { captured: false, lines: 0, method: null, scalingLines: null, why: 'not reached: the Welcome window closes only by No thanks or Escape, and both trap the engine (HostFilesUnsupported) in this build; F10 opens nothing while it is open' },
    pointerErrPx: null, pointerSentErrPx: null, pointerPoints: null, pointerDiag: null, engineFaults: null, scale: null };
  const profile = path.join(root, `profile-${++runNo}`);
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    const p = await openPage(chrome, dpr, viewport);
    cdp = p.cdp;
    const { S, ev } = p;
    const navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    let state = null;
    for (;;) {
      const q = await ev(`(() => { const a = window.__app; return a ? { state: a.state } : null; })()`).catch(() => null);
      state = q ? q.state : null;
      out.state = state;
      if (state === 'engine-idle' || state === 'error' || state === 'exited') break;
      if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
      await sleep(POLL_MS);
    }
    if (state !== 'engine-idle') {
      const tail = await ev(`(() => { const a = window.__app; if (!a) return null; const l = a.engineLog(); return { n: l.length, memBytes: a.engineMemoryBytes, idles: a.idles.length, last: l.slice(-2).map((s) => String(s).slice(0, 120)), error: a.error, room: a.screen && a.screen.room, asked: a.screen && a.screen.asked ? a.screen.asked.env : null }; })()`).catch(() => null);
      throw new Error(`state ${state} before the first engine-idle (no engine-idle within ${WAIT_LIMIT_MS} ms): ${JSON.stringify(tail).slice(0, 600)}`);
    }
    for (let i = 0; i < SETTLE_S; i++) await sleep(1000);
    const lines = await ev(`(() => { const a = window.__app; return a ? a.engineLog().map((l) => String(l)) : []; })()`);
    out.api = dpiAnswers(lines);
    out.mainGuiScaling = mainGuiScalingRead(lines);
    // the engine's faults in its log before the pointer test: a guest thread that fails halts the engine's thread switch (fle_thread_lib.js fail: nothing is handed over), and the page stays "engine-idle"
    const faults = lines.filter((l) => /fle thread \d+ failed/.test(String(l)));
    out.engineFaults = { threadFailed: faults.length, first: faults.length ? faults[0].slice(0, 200) : null };
    // the guest reads LOGPIXELSY (90) and not LOGPIXELSX (88) in this build: the X value is taken only when the Y call is absent
    out.dpi = out.api.logpixelsY ?? out.api.logpixelsX;
    const v = await ev(PAGE);
    out.dpr = v.dpr;
    out.state = v.state;
    out.standIn = v.standIn;
    out.error = v.error ? String(v.error.what ?? v.error).slice(0, 500) : null;
    out.room = v.room ? { w: v.room.width, h: v.room.height } : null;
    out.guest = v.reported && v.reported.width > 0 ? { w: v.reported.width, h: v.reported.height } : null;
    out.asked = v.asked;
    if (!v.canvas || !v.canvasRect) throw new Error('no canvas[data-desk="1"] in the page');
    out.canvas = { w: v.canvas.w, h: v.canvas.h };
    out.display = { w: Math.round(v.canvasRect.w * v.dpr), h: Math.round(v.canvasRect.h * v.dpr) };
    const pm = await pictureMeasures(ev);
    out.scale = { welcomeW: pm.welcomeW, tempoLen: pm.tempo ? pm.tempo.len : null, ratio: pm.scale };
    if (REF && REF.welcomeW && pm.welcomeW) out.flScalingPercent = Math.round((100 * pm.welcomeW) / REF.welcomeW);
    out.edgeWidth = pm.edgeWidth;
    out.parts = pm.parts;
    if (!REF) out.reference = { welcomeW: pm.welcomeW, tempoLen: pm.tempo ? pm.tempo.len : null };
    if (pm.welcome && REF && REF.welcomeW) {
      // the pointer: presses on the Welcome window's title bar (three points, guest pixels), then the guest pixel the engine received ("hit: down ... ptr=x,y").
      // The main form gets no pointer input while this modal window is open (a press on the top bar reaches no engine line), so the presses are inside it.
      const rect = v.canvasRect;
      const w = pm.welcome;
      const ty = w.y + Math.round(15 * (w.w / REF.welcomeW));
      const points = [0.3, 0.5, 0.7].map((k) => ({ gx: w.x + Math.round(k * w.w), gy: ty }));
      const expect = [];
      const at = [];
      const inputState = `({ inputEvents: window.__app.inputEvents, input: document.getElementById('desktop')?.dataset.input ?? null, state: window.__app.state, lines: window.__app.engineLog().length })`;
      const before = await ev(inputState);
      const downsBefore = pointerDowns(await ev(`window.__app.engineLog().map((l) => String(l))`)).length;
      for (const p of points) {
        const cx = rect.x + (p.gx + 0.5) / v.dpr, cy = rect.y + (p.gy + 0.5) / v.dpr;
        expect.push({ client: { x: cx, y: cy }, want: { x: Math.floor((cx - rect.x) * v.dpr), y: Math.floor((cy - rect.y) * v.dpr) } });
        at.push(await ev(`(() => { const e = document.elementFromPoint(${cx}, ${cy}); return e ? e.tagName + (e.id ? '#' + e.id : '') : null; })()`));
        await cdpPress(S, cx, cy);
      }
      // the engine's log reaches the page through the worker's flush (every 100 ms): the lines of the three presses are waited for, 5 s at most
      let lines = [];
      for (let i = 0; i < 10; i++) {
        lines = await ev(`window.__app.engineLog().map((l) => String(l))`);
        if (pointerDowns(lines).length >= downsBefore + expect.length) break;
        await sleep(500);
      }
      const got = pointerDowns(lines).slice(downsBefore, downsBefore + expect.length);
      out.pointerPoints = expect.map((e, i) => ({ want: e.want, got: got[i] ?? null }));
      out.pointerErrPx = got.length === expect.length ? Math.max(...expect.map((e, i) => Math.max(Math.abs(got[i].x - e.want.x), Math.abs(got[i].y - e.want.y)))) : null;
      // the page's own record of the presses it sent to the engine (describeRaw: "button L down at screen (x, y)"): the same coordinates, read without the engine's answer
      const sentDesc = await ev(`window.__app.latency.records.map((r) => r.desc)`);
      const sent = sentDesc.map((d) => /^button \w+ down at screen \((-?\d+), (-?\d+)\)$/.exec(d)).filter((m) => m !== null).map((m) => ({ x: Number(m[1]), y: Number(m[2]) })).slice(-expect.length);
      out.pointerSentErrPx = sent.length === expect.length ? Math.max(...expect.map((e, i) => Math.max(Math.abs(sent[i].x - e.want.x), Math.abs(sent[i].y - e.want.y)))) : null;
      out.pointerDiag = { before, after: await ev(inputState), at, downsBefore, downs: pointerDowns(lines).length, tail: lines.slice(-4).map((l) => String(l).slice(0, 160)) };
    }
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  return out;
}

// the viewport that gives the room ROOM_CSS: the page's layout outside the stage is fixed, so the viewport is corrected by the difference the page shows at DPR 1 (at most three boots)
async function calibrate(port, root) {
  let vp = { w: ROOM_CSS.w, h: ROOM_CSS.h };
  for (let i = 0; i < 3; i++) {
    const r = await runOnce(1, vp, port, root, null);
    process.stderr.write(`calibrate ${i}: viewport ${vp.w}x${vp.h} gives room ${JSON.stringify(r.room)}${r.error ? ` error ${r.error.slice(0, 300)}` : ''}\n`);
    if (r.error || !r.room) throw new Error(`calibration: ${r.error ?? 'no room'}`);
    if (Math.abs(r.room.w - ROOM_CSS.w) <= 2 && Math.abs(r.room.h - ROOM_CSS.h) <= 2) {
      REF = r.reference && r.reference.tempoLen ? r.reference : null;
      if (!REF) process.stderr.write('calibrate: no Tempo box or Welcome window in the picture: the scale and the parts are null\n');
      return vp;
    }
    vp = { w: vp.w + ROOM_CSS.w - r.room.w, h: vp.h + ROOM_CSS.h - r.room.h };
  }
  throw new Error('calibration: the room is not the size asked after three boots');
}

async function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`measure-size: ${e.message}\n`);
    process.exitCode = 2;
    return;
  }
  if (!opts.bundleDir) {
    process.stderr.write('usage: node tools/feat-c1-hidpi/measure-size.mjs <absolute path to the bundle directory> [--dprs <list>] [--random <count>] [--random-range <min,max>]\n');
    process.exitCode = 2;
    return;
  }
  const bundleDir = opts.bundleDir;
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-c1-size-'));
  const assets = path.join(root, 'assets');
  let server = null, failed = false;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const vp = await calibrate(port, root);
    const rnd = randomDprs(opts.random, opts.range, undefined, opts.dprs);
    for (const dpr of [...opts.dprs, ...rnd.values]) {
      const r = await runOnce(dpr, vp, port, root, rnd);
      if (r.error) failed = true;
      process.stdout.write(JSON.stringify(r) + '\n');
    }
    const high = {};
    for (const dpr of HIGH_DPRS) {
      const r = await runOnce(dpr, vp, port, root, null);
      high[String(dpr)] = r.state === 'engine-idle' ? 'engine-idle' : 'stopped';
      process.stderr.write(`highDpr ${dpr}: ${high[String(dpr)]}${r.error ? ` (${r.error.slice(0, 300)})` : ''}\n`);
    }
    process.stdout.write(JSON.stringify({ highDpr: high }) + '\n');
  } catch (e) {
    failed = true;
    process.stderr.write(`measure-size: ${String(e?.message ?? e).slice(0, 500)}\n`);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.exitCode = failed ? 1 : 0;
}

if (isMain) await main();
