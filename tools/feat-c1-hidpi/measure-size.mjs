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
//  - NOT MEASURED here (null): flScalingPercent (FL's scale of its UI), mainGuiScaling, parts (the positions of the UI's parts), edgeWidth, blackShare (the Browser's rectangle is
//    #1355's), debugLog. #1353 gives the first four and the Debug log: the window table (fle_desk_describe) has only the top-level windows, the Main GUI Scaling setting is read by no
//    traced call, and the Debug log's text is not read from the guest (pm-work/findings/hidpi-fl-scaling.md).
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

// the random ratios: a seed (from the system's random bytes on each start unless given), then mulberry32; each value is a multiple of 0.01 in the range
export function randomDprs(count, [min, max], seedIn = crypto.randomBytes(4).readUInt32BE(0)) {
  let s = seedIn >>> 0;
  const next = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const lo = Math.round(min * 100), hi = Math.round(max * 100);
  const values = Array.from({ length: count }, () => (lo + Math.floor(next() * (hi - lo + 1))) / 100);
  return { seed: seedIn >>> 0, values };
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

// one boot at one ratio. The row's state is the last state the page showed ("engine-idle" when the engine reached its first idle); a stop shows as another state or as no idle within the limit
async function runOnce(dpr, viewport, port, root, rnd) {
  const out = { dpr, randomDprs: rnd, room: null, state: null, standIn: null, error: null, guest: null, dpi: null, api: null, asked: null,
    flScalingPercent: null, mainGuiScaling: null, parts: null, canvas: null, display: null, edgeWidth: null, blackShare: null, debugLog: null };
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
    if (Math.abs(r.room.w - ROOM_CSS.w) <= 2 && Math.abs(r.room.h - ROOM_CSS.h) <= 2) return vp;
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
    const rnd = randomDprs(opts.random, opts.range);
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
