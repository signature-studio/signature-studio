// feat-c1-hidpi/measure-size: the guest's screen, its DPI, the canvas and the windows at each device pixel ratio (Issues #1347, #1348). Boots the bundle in headless Chrome as the page
// is served (the page's room, and FLE_MONITOR_W / H / DPR in the engine's environment) once for each ratio, waits for the first engine-idle, and prints ONE JSON line per row on stdout.
// Exit 2 on a usage error, 1 when the run itself failed (calibration, the bundle), 0 otherwise: a row with an error (a boot or a window that could not be measured) is data, and the acceptance reads it.
// A null is a value this tool does not measure: the acceptance reads it.
//
// Usage: node tools/feat-c1-hidpi/measure-size.mjs <absolute path to the bundle directory raw/<name>> [--dprs <list>] [--random <count>] [--random-range <min,max>]
//                                                 [--windows <list>] [--dynamic <list>] > rows.jsonl
//  --dprs <list>: the fixed ratios (default 1,1.25,1.5,1.7647,2). Only the inputs of the runs: no expected value is computed from them.
//  --random <count> --random-range <min,max>: the random ratios (default 2 in 1.00..2.25; #1348 passes 3 in 1,3). Multiples of 0.01, drawn from a seed that crypto.randomBytes gives on each start
//    (mulberry32). randomDprs is { seed, values } on every row of the start.
//  --windows <list>: the windows measured at each static ratio, in the order of WINDOW_KINDS (default main): welcome, main, mixer, playlist, piano-roll, plugin, tooltip, menu.
//    A window that cannot be opened has its row's error set (the reason); the others of the same ratio are still measured.
//  --dynamic <list>: one boot at the first ratio of the list; the ratio is then changed to each of the following ones in turn (Emulation.setDeviceMetricsOverride). One row per step (step 1..n).
//
// What each value is:
//  - dpr: window.devicePixelRatio. randomDprs: the seed and the values of the start (the same on every row). room: window.__app.screen.room (the stage in CSS pixels; the ratio does not change it).
//  - state, standIn, error: the page (window.__app.state and error; "STAND-IN" in the page). error of a row: the boot's error, else the window's error.
//  - guest: the screen the ENGINE reports (window.__app.screen.reported). dpi: LOGPIXELSY (index 90) of the start, from the window-API trace (the guest reads only Y in this build).
//    api: the DPI answers of the trace. dpiCalls (dynamic rows): the DPI answers made after the step's change (0: FL did not ask again).
//  - canvas: canvas[data-desk="1"] width and height (the drawing buffer). canvasDisplay and display: the device pixels the canvas covers (getBoundingClientRect x dpr, rounded). display = canvasDisplay.
//  - flScalingPercent (#1353): round(100 x the Welcome window's width / its width at the DPR 1 reference of the calibration boot): FL's own window, measured at the start of each boot.
//    For a dynamic step it is measured at the step (the Welcome window stays open in a dynamic run), so a change of FL's scale would show.
//  - windowRect: the window's absolute rectangle in guest pixels (the window table, with the parents' offsets added). edgeWidth: the 10 % to 90 % width of the transitions at the four edges of
//    the window (the median of the four; pixels). For main it is the Tempo box's frame (the top bar's LCD, #1353's measure); for the other windows it is the window's own frame.
//    The contrast an edge needs is 40 (of 255), except the menu's: 20, in either direction (its frame is dark on the black desktop: 28 at DPR 3; #1348).
//  - blackShare: the share of the pixels of the Browser panel (TEdgeBrowser, main's row) whose R, G and B are all at most 16.
//  - parts (main row and dynamic rows): the Tempo box's frame is measured; "song-position" and "pattern-selector" are estimated from it (measured: false).
//  - pointerErrPx (main row): presses at the centres of the three parts; the error is max |engine's guest pixel (hit: down ... ptr=x,y) - the guest pixel pressed| over the presses.
//    pointerSentErrPx: the same from the page's own record of the presses it sent. pointerDiag: the input count, the element under each press, the log's last lines.
//  - settings (#1358): the Settings window (F10) after the windows of the boot; debugLog: FL's start-up lines in the guest memory (the first idle, and the Debug page after it);
//    scalingLines: the lines with dpi, scal, screen, monitor, display, resol, zoom, window, error or warn; scalingFactor: FL's own line "Main GUI scaling factor: F (system set to N PPI)".
//  - mainGuiScaling: "System" when FL's log line says "system set to N PPI" (inferred: the Display selection itself is not identified; #1358 10.4), else null.
//  - restartNotice (dynamic rows): the page's element with data-restart-notice is shown.
//  - displayLines (dynamic rows): the engine's "input: WM_DISPLAYCHANGE" lines of the step (count and the first two); dpiChangedLines: the WM_DPICHANGED lines of the step (0: not sent).
//  - pluginScalePercent (plugin row): not measured in this build (the plugin window is not opened: its row's error says why).
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
const ROOM_CSS = { w: 1836, h: 1056 };
// the windows in the order they are opened (welcome first: it is open at the start and the main window's row closes it)
const WINDOW_KINDS = ['welcome', 'main', 'mixer', 'playlist', 'piano-roll', 'plugin', 'tooltip', 'menu'];
const DEFAULT_WINDOWS = ['main'];
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 1000;
const SETTLE_S = 3;
const FOLLOW_LIMIT_MS = 15000;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same launch as tools/feat-c1-hidpi/measure.mjs)
export const ISOLATION = [
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
  const opts = { bundleDir: '', dprs: FIXED_DPRS, random: DEFAULT_RANDOM.count, range: DEFAULT_RANDOM.range, windows: DEFAULT_WINDOWS, dynamic: null };
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
    } else if (a === '--windows') {
      const names = String(argv[++i] ?? '').split(',').map((x) => x.trim()).filter((x) => x !== '');
      if (names.length === 0 || names.some((x) => !WINDOW_KINDS.includes(x))) throw new Error(`--windows: not a list of ${WINDOW_KINDS.join(',')}: ${argv[i]}`);
      opts.windows = WINDOW_KINDS.filter((k) => names.includes(k));
    } else if (a === '--dynamic') opts.dynamic = list(argv[++i], '--dynamic');
    else if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
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

// The picture of the guest (deskDumpBase64: RGBA, rows of `stride` bytes) and the analyses of it. Nothing is drawn or compared with an expected picture.
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

/** the 10 % to 90 % width (pixels) of a transition in a profile that runs from the outside (first samples) to the inside (last samples); null when there is no transition.
 *  The contrast must be at least `minContrast` (the inside minus the outside; `anySign`: its size, in either direction) */
export function transitionWidth(prof, minContrast = 40, anySign = false) {
  const lo = (prof[0] + prof[1]) / 2, hi = (prof[prof.length - 2] + prof[prof.length - 1]) / 2;
  if (!((anySign ? Math.abs(hi - lo) : hi - lo) >= minContrast)) return null;
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

const median = (vs) => {
  const s = vs.slice().sort((p, q) => p - q);
  if (s.length === 0) return null;
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};

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
  return { edgeWidth: median(widths), frame: { x: box.x0 - 1, y: yTop - 1, x1: box.x1 + 1, y1: yBot + 1 } };
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

/** #1348: every window of the window table with its absolute rectangle (guest px: the offsets of its parents added). Top-level windows count as visible. */
export function windowTable(windows) {
  const recs = [];
  for (const l of String(windows).split('\n')) {
    const m = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (!m) continue;
    const rest = m[8];
    recs.push({
      lid: Number(m[2]), parent: m[3] === undefined ? null : Number(m[3]), x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]),
      child: m[1] !== undefined,
      vis: m[1] === undefined ? true : /\bvis 1\b/.test(rest),
      cls: /class "([^"]*)"/.exec(rest)?.[1] ?? '',
      text: /(?:text|title) "([^"]*)"/.exec(rest)?.[1] ?? '',
    });
  }
  const by = new Map(recs.map((r) => [r.lid, r]));
  for (const r of recs) {
    let x = r.x, y = r.y, p = r.parent, guard = 0;
    while (p !== null && by.has(p) && guard++ < 64) { const q = by.get(p); x += q.x; y += q.y; p = q.parent; }
    r.ax = x; r.ay = y;
  }
  return recs;
}

/** FL's reading of the scaling settings: the last registry read (window-api: RegQueryValueExW) of a value whose name has "scaling", "GUI", "uiscale" or "PPI" in it (FL's "Main PPI" etc. in Software\Image-Line\Shared\General): { found, value, line }; null when FL read no such value */
export function mainGuiScalingRead(lines) {
  const hits = lines.map(String).filter((l) => /RegQueryValueExW "[^"]*" "[^"]*(scaling|gui|uiscale|ppi)[^"]*" -> /i.test(l));
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

const CHILD_RE = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "([^"]*)"\s*$/;
const TOP_RE = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) .*class "([^"]*)" title "([^"]*)"\s*$/;
const SETTINGS_SLOTS = 10;
// #1358: the start-up lines of FL (the first words of each line; the whole lines first, then the shorter ones) and the labels of the General page's Display section
const DEBUG_MARKERS = ['Browser caching took 0 milliseconds.', 'There were no updates for MIDI scripts.', 'There were no updates for themes.', 'Mixer track map: 1,2,3,4', 'There were no updates for', 'Browser caching took'];
const INFO_MARKERS = ['FL Studio', 'Windows'];
const DISPLAY_LABELS = ['Main GUI Scaling', 'Pop-ups', 'Toolbars', 'Cursors', 'Legacy scaling'];
const DEBUG_HEAD = 30;
const SCALING_RE = /dpi|scal|screen|monitor|display|resol|zoom|window|error|warn/i;
const SCALING_FACTOR_RE = /Main GUI scaling factor: ([\d.]+) \(system set to (\d+)PPI\)/;
const INFO_KEEP = /FL Studio|version|build|windows|\bos\b|screen|display|dpi|monitor|resol|scal|graphic|gpu|opengl|direct/i;

/** the text with the most lines among the scans ([foundAt, scan] pairs; a scan is the guestText answer, or an error): { foundAt, best, memBytes } or null */
export function bestText(scans) {
  let pick = null;
  for (const [foundAt, s] of scans) {
    if (!s || !s.best) continue;
    if (pick === null || s.best.lineCount > pick.best.lineCount) pick = { foundAt, best: s.best, memBytes: s.memBytes };
  }
  return pick;
}

/** #1358: the Debug page's log as the guest memory holds it (the start-up lines); captured when it has a line */
export function debugLogOf(scans, why) {
  const pick = bestText(scans);
  const lines = pick ? pick.best.lines : [];
  const n = pick ? pick.best.lineCount : 0;
  const f = lines.map((l) => SCALING_FACTOR_RE.exec(l)).find((m) => m) ?? null;
  return {
    captured: n >= 1,
    lines: n,
    method: 'guest-memory-scan',
    foundAt: pick ? pick.foundAt : null,
    marker: pick ? pick.best.marker : null,
    enc: pick ? pick.best.enc : null,
    offset: pick ? pick.best.offset : null,
    head: pick ? lines.slice(0, DEBUG_HEAD) : null,
    scalingLines: pick ? lines.filter((l) => SCALING_RE.test(l)).slice(0, 200).map((l) => l.slice(0, 200)) : null,
    scalingFactor: f ? { factor: Number(f[1]), systemPpi: Number(f[2]), line: f[0] } : null,
    why: n >= 1 ? null : why,
  };
}

/** #1358: the Info text's lines that mention FL, its version, the OS or the screen (40 at most); null when there is no Info text */
export function infoOf(scans) {
  const pick = bestText(scans);
  if (!pick) return null;
  const keep = pick.best.lines.filter((l) => INFO_KEEP.test(l)).slice(0, 40).map((l) => l.slice(0, 200));
  return keep.length ? keep : null;
}

/** the child windows of a window list (the page's text of the window table: "child lid N parent lid M at (x, y) WxH vis V class "..." text "..."") */
export function childWindows(windows) {
  return windows.split('\n').map((l) => CHILD_RE.exec(l)).filter(Boolean).map((m) => ({ lid: Number(m[1]), parent: Number(m[2]), x: Number(m[3]), y: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9] }));
}

/** the top-level window of a class: { lid, x, y, w, h, title }, or null */
export function topWindow(windows, cls) {
  for (const l of windows.split('\n')) {
    const m = TOP_RE.exec(l);
    if (m && m[6] === cls) return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), title: m[7] };
  }
  return null;
}

/** the windows under `lid` (its children, grandchildren ...), in window-table order */
export function descendantsOf(children, lid) {
  const out = [];
  let frontier = [lid];
  while (frontier.length) {
    const next = children.filter((c) => frontier.includes(c.parent));
    out.push(...next);
    frontier = next.map((c) => c.lid);
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
    restartNotice: (() => { const e = document.querySelector('[data-restart-notice]'); return e ? !e.hidden : null; })(),
    diag: (() => {
      const st = document.getElementById('stage');
      const sr = st ? st.getBoundingClientRect() : null;
      return { stage: sr ? { w: sr.width, h: sr.height } : null, canvasStyle: c ? { w: c.style.width, h: c.style.height } : null, guest: a && a.screen && a.screen.guest ? a.screen.guest : null };
    })(),
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

export async function openPage(chrome, dpr, viewport) {
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
  // the exceptions the page throws (its own, not the engine's): kept for the rows (pageExceptions)
  cdp.pageExceptions = [];
  cdp.on((m) => {
    if (m.sessionId === sessionId && m.method === 'Runtime.exceptionThrown') cdp.pageExceptions.push(String(m.params?.exceptionDetails?.exception?.description ?? m.params?.exceptionDetails?.text ?? '').slice(0, 300));
  });
  await S('Runtime.enable');
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

/** the picture and the window table of the guest, read once (deskDumpBase64: nothing is drawn) */
async function snapshot(ev) {
  const d = await ev(`window.__app.deskDumpBase64(true)`);
  const b = Buffer.from(d.base64, 'base64');
  return { b, stride: b.length / d.height, w: d.width, h: d.height, recs: windowTable(d.windows) };
}

/** the Tempo box of a snapshot: its frame, the edge width of the frame, the parts (when the scale is known) */
function tempoOf(s, ref) {
  const box = tempoBox(s.b, s.stride, s.w, s.h);
  if (!box) return { tempo: null, edgeWidth: null, parts: null, scale: null };
  const edges = tempoEdges(s.b, s.stride, box);
  const sc = ref && ref.tempoLen ? box.len / ref.tempoLen : null;
  return { tempo: { len: box.len, x0: box.x0, x1: box.x1, y: box.y, frame: edges.frame }, edgeWidth: edges.edgeWidth, parts: sc === null ? null : partPlaces(edges.frame, sc), scale: sc };
}

// #1348: the contrast a menu's edge must have (the frame of the menu is dark on the black desktop: 28 of 255 at DPR 3, against 40 for the other windows)
const MENU_MIN_CONTRAST = 20;

/** #1348: the four edges of a window's rectangle (guest px): at the middle of each side, the transition from 4 px outside to 3 px inside; the median of the widths found */
export function rectEdges(s, rect, minContrast = 40, anySign = false) {
  const L = (x, y) => lumAt(s.b, s.stride, Math.min(s.w - 1, Math.max(0, x)), Math.min(s.h - 1, Math.max(0, y)));
  const seq = (a, c) => Array.from({ length: Math.abs(c - a) + 1 }, (_, i) => (c >= a ? a + i : a - i));
  const mx = Math.round(rect.ax + rect.w / 2), my = Math.round(rect.ay + rect.h / 2);
  const widths = [
    transitionWidth(seq(rect.ax - 4, rect.ax + 3).map((x) => L(x, my)), minContrast, anySign),
    transitionWidth(seq(rect.ax + rect.w + 3, rect.ax + rect.w - 4).map((x) => L(x, my)), minContrast, anySign),
    transitionWidth(seq(rect.ay - 4, rect.ay + 3).map((y) => L(mx, y)), minContrast, anySign),
    transitionWidth(seq(rect.ay + rect.h + 3, rect.ay + rect.h - 4).map((y) => L(mx, y)), minContrast, anySign),
  ].filter((v) => v !== null);
  return median(widths);
}

/** #1355's rule: the share of the pixels of a rectangle (guest px) whose R, G and B are all at most 16 */
export function blackShareOf(s, rect) {
  const x0 = Math.max(0, rect.ax), y0 = Math.max(0, rect.ay), x1 = Math.min(s.w, rect.ax + rect.w), y1 = Math.min(s.h, rect.ay + rect.h);
  let n = 0, black = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const i = y * s.stride + 4 * x;
      n++;
      if (s.b[i] <= 16 && s.b[i + 1] <= 16 && s.b[i + 2] <= 16) black++;
    }
  }
  return n ? black / n : null;
}

/** FL's scale in percent from the Welcome window's width (#1353); null without the reference or the window */
function flScalingOf(s, ref) {
  const welcome = s.recs.find((r) => r.vis && r.cls === 'TWelcomeWizard') ?? null;
  return { welcomeW: welcome ? welcome.w : null, flScalingPercent: ref && ref.welcomeW && welcome ? Math.round((100 * welcome.w) / ref.welcomeW) : null };
}

// #1358: the text the guest's memory holds for the markers (window.__app.guestText). A failure is an { error } value: it does not fail the row
const scanSafe = (ev, markers, radius = 0) => ev(`window.__app.guestText(${JSON.stringify(markers)}, ${radius})`).catch((e) => ({ error: String(e?.message ?? e).slice(0, 200) }));

/** #1358: the Settings window (F10). The Welcome window and the dialog after it are closed with Escape (while listed, at most four times); F10 opens the Settings window; the tab bar's ten press points
 * (its rectangle split evenly; a press is a click of the page's own pointer path) show the pages in turn: each one's caption is the visible TVectorSheet's text, and the title is the window's own.
 * The guest memory is read after the Debug page and the Info page are pressed (debugScan, infoScan), and near the Display labels of the General page (displayNear). A null always says why (settings.why, debugLog.why). */
async function settingsPhase(S, ev, v) {
  const ret = { opened: false, title: null, bar: null, tabs: [], captions: [], generalPanels: [], debugTab: false, infoTab: false, debugMemo: null, debugScan: null, infoScan: null, displayNear: null, displayWhy: null, scalingRegistry: null, textDrawingReached: 0, why: null };
  const windows = async () => String(await ev(`(async () => (await window.__app.deskDumpBase64(true)).windows)()`));
  const focusDesk = () => ev(`document.getElementById('desktop')?.focus()`);
  const pressKey = async (code, vk) => {
    await focusDesk();
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(700);
  };
  const rect = v.canvasRect;
  const pressGuest = async (gx, gy) => {
    await cdpPress(S, rect.x + (gx + 0.5) / v.dpr, rect.y + (gy + 0.5) / v.dpr);
    await sleep(600);
  };
  for (let i = 0; i < 4; i++) {
    if (!/TWelcomeWizard|TMissingPluginsForm/.test(await windows())) break;
    await pressKey('Escape', 27);
    await sleep(2000);
  }
  await pressKey('F10', 121);
  let w = '', top = null;
  for (let i = 0; i < 15 && !top; i++) {
    await sleep(1000);
    w = await windows();
    top = topWindow(w, 'TMIDIForm');
  }
  if (!top) {
    ret.why = 'the Settings window (TMIDIForm) did not open within 15 s of F10';
    return ret;
  }
  ret.opened = true;
  ret.title = top.title;
  const bar = childWindows(w).find((c) => c.parent === top.lid && c.cls === 'TQuickSheetSelector');
  if (!bar) {
    ret.why = 'no tab bar (TQuickSheetSelector) in the Settings window';
    return ret;
  }
  ret.bar = { x: bar.x, y: bar.y, w: bar.w, h: bar.h };
  const pageNow = (kids) => kids.find((c) => c.parent === top.lid && c.cls === 'TVectorSheet' && c.vis) ?? null;
  for (let s = 0; s < SETTINGS_SLOTS; s++) {
    await pressGuest(top.x + bar.x + ((s + 0.5) * bar.w) / SETTINGS_SLOTS, top.y + bar.y + bar.h / 2);
    const cw = await windows();
    const page = pageNow(childWindows(cw));
    ret.tabs.push({ slot: s, page: page ? page.text : null, title: topWindow(cw, 'TMIDIForm')?.title ?? null });
  }
  ret.captions = [...new Set(ret.tabs.map((t) => t.page).filter((p) => p))];
  const slotOf = (caption) => ret.tabs.find((t) => t.page === caption)?.slot ?? -1;
  const general = slotOf('General');
  if (general >= 0) {
    await pressGuest(top.x + bar.x + ((general + 0.5) * bar.w) / SETTINGS_SLOTS, top.y + bar.y + bar.h / 2);
    const kids = childWindows(await windows());
    const page = pageNow(kids);
    ret.generalPanels = page ? descendantsOf(kids, page.lid).filter((c) => c.cls === 'TVectorPanel' && c.text).map((c) => c.text) : [];
    ret.displayNear = await scanSafe(ev, DISPLAY_LABELS, 512);
    ret.displayWhy = 'the selections are not identified: the window table keeps no text of them (FL paints them: QuickFontCache, Blend2D), and the memory holds the labels only as the runs in displayNear';
  }
  const debug = slotOf('Debug');
  ret.debugTab = debug >= 0;
  if (debug >= 0) {
    await pressGuest(top.x + bar.x + ((debug + 0.5) * bar.w) / SETTINGS_SLOTS, top.y + bar.y + bar.h / 2);
    const kids = childWindows(await windows());
    const page = pageNow(kids);
    const memo = page ? descendantsOf(kids, page.lid).find((c) => c.cls === 'TQuickMemo') : null;
    ret.debugMemo = memo ? { lid: memo.lid, text: memo.text } : null;
    ret.debugScan = await scanSafe(ev, DEBUG_MARKERS);
  }
  const info = slotOf('Info');
  ret.infoTab = info >= 0;
  if (info >= 0) {
    await pressGuest(top.x + bar.x + ((info + 0.5) * bar.w) / SETTINGS_SLOTS, top.y + bar.y + bar.h / 2);
    ret.infoScan = await scanSafe(ev, INFO_MARKERS);
  }
  const log = await ev(`window.__app.engineLog().map((l) => String(l))`);
  ret.scalingRegistry = mainGuiScalingRead(log);
  ret.textDrawingReached = log.filter((l) => /TEXT DRAWING REACHED/.test(l)).length;
  if (general < 0) ret.why = `the General page was not reached (the presses showed: ${JSON.stringify(ret.captions)})`;
  else if (!ret.debugTab) ret.why = `no Debug page in the tab bar at this window size (the presses showed: ${JSON.stringify(ret.captions)})`;
  return ret;
}

/** the keys and the presses of a boot's page (the page's own pointer path; the guest's window positions are in guest pixels) */
function inputsOf(S, ev, v) {
  const focusDesk = () => ev(`document.getElementById('desktop')?.focus()`);
  const pressKey = async (code, vk) => {
    await focusDesk();
    await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(40);
    await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(1500);
  };
  const cssOf = (gx, gy) => ({ x: v.canvasRect.x + (gx + 0.5) / v.dpr, y: v.canvasRect.y + (gy + 0.5) / v.dpr });
  const pressGuest = async (gx, gy) => {
    const p = cssOf(gx, gy);
    await cdpPress(S, p.x, p.y);
    await sleep(600);
  };
  const hover = async (gx, gy) => {
    const p = cssOf(gx, gy);
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x - 3, y: p.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(2500);
  };
  return { pressKey, pressGuest, hover };
}

const isVisible = (s, pred) => s.recs.find((r) => r.vis && pred(r)) ?? null;
const named = (cls, text) => (r) => r.cls === cls && (text === undefined || r.text === text);

/** opens (or finds open) the window `kind` and returns its snapshot and its record; throws with the reason when it cannot be opened */
async function openWindow(kind, ctx) {
  const { ev, inputs } = ctx;
  const showBy = async (pred, what, action) => {
    let s = await snapshot(ev);
    let rec = isVisible(s, pred);
    if (!rec && action) {
      await action();
      // a window opens a little after its key: the table is read again for 4 s
      for (let i = 0; i < 8 && !rec; i++) {
        await sleep(500);
        s = await snapshot(ev);
        rec = isVisible(s, pred);
      }
    }
    if (!rec) {
      const shown = s.recs.filter((r) => r.vis && !/TNewCaption|TQuick|TWPControl|TVector|TNum|TWinControl|TNewMenu|TEdgeBrowser|TSampleList|TStepSeq|TNewMenu/.test(r.cls)).map((r) => `${r.cls}${r.text ? `:${r.text}` : ''}`);
      throw new Error(`${what} is not visible${action ? ' after its key or press' : ''} (visible: ${shown.join(', ')})`);
    }
    return { s, rec };
  };
  if (kind === 'welcome') return showBy(named('TWelcomeWizard'), 'the Welcome window (open at the start)');
  if (kind === 'main') {
    for (let i = 0; i < 4; i++) {
      const s0 = await snapshot(ev);
      if (!isVisible(s0, named('TWelcomeWizard')) && !isVisible(s0, named('TMissingPluginsForm'))) break;
      await inputs.pressKey('Escape', 27);
      await sleep(2000);
    }
    return showBy(named('TFruityLoopsMainForm'), 'the main window');
  }
  if (kind === 'mixer') return showBy(named('TFXForm'), 'the Mixer', () => inputs.pressKey('F9', 120));
  if (kind === 'playlist') return showBy(named('TEventEditForm', 'Playlist -'), 'the Playlist', () => inputs.pressKey('F5', 116));
  if (kind === 'piano-roll') return showBy(named('TEventEditForm', 'Piano roll -'), 'the Piano roll', () => inputs.pressKey('F7', 118));
  if (kind === 'menu') return showBy(named('TQuickPopupMenuWindow'), 'the menu popup (a press on the menu bar at 30,20)', () => inputs.pressGuest(30, 20));
  if (kind === 'tooltip') {
    const before = new Set((await snapshot(ev)).recs.filter((r) => !r.child).map((r) => r.lid));
    for (const [gx, gy] of [[464, 20], [900, 20], [1460, 20]]) {
      await inputs.hover(gx, gy);
      const s = await snapshot(ev);
      const rec = s.recs.find((r) => !r.child && !before.has(r.lid)) ?? null;
      if (rec) return { s, rec };
    }
    throw new Error('no tooltip window appeared: hovering over the top bar (3 points, 2.5 s each) showed no new top-level window');
  }
  if (kind === 'plugin') throw new Error('no plugin window: no native plugin (Fruity Balance) is open in the project, and this tool does not insert one into a mixer slot');
  throw new Error(`unknown window ${kind}`);
}

/** the presses on the parts' centres of the main window (guest px): the engine's answer (hit: down ... ptr=x,y) against the pixel pressed */
async function pointerTest(ctx, points) {
  const { S, ev, v } = ctx;
  const rect = v.canvasRect;
  const want = points.map((p) => ({ x: p.x, y: p.y }));
  const inputState = `({ inputEvents: window.__app.inputEvents, input: document.getElementById('desktop')?.dataset.input ?? null, state: window.__app.state, lines: window.__app.engineLog().length })`;
  const before = await ev(inputState);
  const downsBefore = pointerDowns(await ev(`window.__app.engineLog().map((l) => String(l))`)).length;
  const at = [];
  for (const p of points) {
    const cx = rect.x + (p.x + 0.5) / v.dpr, cy = rect.y + (p.y + 0.5) / v.dpr;
    at.push(await ev(`(() => { const e = document.elementFromPoint(${cx}, ${cy}); return e ? e.tagName + (e.id ? '#' + e.id : '') : null; })()`));
    await cdpPress(S, cx, cy);
  }
  // the engine's log reaches the page through the worker's flush (every 100 ms): the lines of the presses are waited for, 5 s at most
  let lines = [];
  for (let i = 0; i < 10; i++) {
    lines = await ev(`window.__app.engineLog().map((l) => String(l))`);
    if (pointerDowns(lines).length >= downsBefore + want.length) break;
    await sleep(500);
  }
  const got = pointerDowns(lines).slice(downsBefore, downsBefore + want.length);
  const errOf = (g, w) => Math.max(Math.abs(g.x - w.x), Math.abs(g.y - w.y));
  const sent = (await ev(`window.__app.latency.records.map((r) => r.desc)`)).map((d) => /^button \w+ down at screen \((-?\d+), (-?\d+)\)$/.exec(d)).filter((m) => m !== null).map((m) => ({ x: Number(m[1]), y: Number(m[2]) })).slice(-want.length);
  return {
    pointerPoints: want.map((w, i) => ({ want: w, got: got[i] ?? null })),
    pointerErrPx: got.length === want.length ? Math.max(...want.map((w, i) => errOf(got[i], w))) : null,
    pointerSentErrPx: sent.length === want.length ? Math.max(...want.map((w, i) => errOf(sent[i], w))) : null,
    pointerDiag: { before, after: await ev(inputState), at, downsBefore, downs: pointerDowns(lines).length, tail: lines.slice(-4).map((l) => String(l).slice(0, 160)) },
  };
}

/** one row of a window of a static boot: the window is opened (or found), its rectangle and edges are measured; the main window also gets the parts, the Browser's blackShare and the pointer test */
async function windowRow(kind, ctx) {
  const row = { window: kind, windowRect: null, error: null, edgeWidth: null, blackShare: null, parts: null, pluginScalePercent: null, pointerErrPx: null, pointerSentErrPx: null, pointerPoints: null, pointerDiag: null };
  try {
    const { s, rec } = await openWindow(kind, ctx);
    row.windowRect = { x: rec.ax, y: rec.ay, w: rec.w, h: rec.h, cls: rec.cls, title: rec.text };
    if (kind === 'main') {
      const t = tempoOf(s, REF);
      row.edgeWidth = t.edgeWidth;
      row.parts = t.parts;
      const browser = s.recs.find((r) => r.vis && r.cls === 'TEdgeBrowser') ?? null;
      row.blackShare = browser ? blackShareOf(s, browser) : null;
      // the presses are made after the other windows are measured (they change the top bar's state: they must not change what the other windows open with)
      if (t.parts) ctx.pointerPoints = ['tempo', 'song-position', 'pattern-selector'].map((k) => ({ x: t.parts[k].cx, y: t.parts[k].cy }));
      else row.pointerDiag = { why: 'the parts are not known (no reference: the calibration boot has none)' };
    } else {
      // the menu's frame is dark on the black desktop (its outside contrast is about 28 at 3x, #1348): its edges are measured with a contrast of at least 20 in either direction
      row.edgeWidth = kind === 'menu' ? rectEdges(s, rec, MENU_MIN_CONTRAST, true) : rectEdges(s, rec);
    }
  } catch (e) {
    row.error = String(e?.message ?? e).slice(0, 300);
  } finally {
    if (kind === 'menu' || kind === 'tooltip') await ctx.inputs.pressKey('Escape', 27).catch(() => {});
  }
  return row;
}

/** the boot's common fields: the same on every row of one boot (its ratio, the page, the engine's DPI, FL's scale and debugLog) */
function blankBoot(dpr, rnd) {
  return {
    dpr, randomDprs: rnd, room: null, state: null, standIn: null, error: null, guest: null, dpi: null, api: null, asked: null,
    flScalingPercent: null, canvas: null, canvasDisplay: null, display: null, scale: null, engineFaults: null,
    settings: null, info: null, mainGuiScaling: null,
    debugLog: { captured: false, lines: 0, method: null, foundAt: null, marker: null, enc: null, offset: null, head: null, scalingLines: null, why: 'the engine did not reach its first idle' },
  };
}

/** the engine's log and the page's first facts: navigate, wait for the first engine-idle (WAIT_LIMIT_MS), settle, and read the DPI answers and the start-up lines */
export async function bootToIdle(S, ev, port, state) {
  const navAt = Date.now();
  const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
  if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
  let st = null;
  for (;;) {
    const q = await ev(`(() => { const a = window.__app; return a ? { state: a.state } : null; })()`).catch(() => null);
    st = q ? q.state : null;
    state.state = st;
    if (st === 'engine-idle' || st === 'error' || st === 'exited') break;
    if (Date.now() - navAt >= WAIT_LIMIT_MS) break;
    await sleep(POLL_MS);
  }
  if (st !== 'engine-idle') {
    const tail = await ev(`(() => { const a = window.__app; if (!a) return null; const l = a.engineLog(); return { n: l.length, memBytes: a.engineMemoryBytes, idles: a.idles.length, last: l.slice(-2).map((s) => String(s).slice(0, 120)), error: a.error, room: a.screen && a.screen.room, asked: a.screen && a.screen.asked ? a.screen.asked.env : null }; })()`).catch(() => null);
    throw new Error(`state ${st} before the first engine-idle (no engine-idle within ${WAIT_LIMIT_MS} ms): ${JSON.stringify(tail).slice(0, 600)}`);
  }
  for (let i = 0; i < SETTLE_S; i++) await sleep(1000);
}

/** the page's facts of the boot (the ratio, the room, the guest, the canvas) into `boot` */
async function pageFacts(ev, boot) {
  const v = await ev(PAGE);
  boot.dpr = v.dpr;
  boot.state = v.state;
  boot.standIn = v.standIn;
  boot.error = v.error ? String(v.error.what ?? v.error).slice(0, 500) : null;
  boot.room = v.room ? { w: v.room.width, h: v.room.height } : null;
  boot.guest = v.reported && v.reported.width > 0 ? { w: v.reported.width, h: v.reported.height } : null;
  boot.asked = v.asked;
  if (!v.canvas || !v.canvasRect) throw new Error('no canvas[data-desk="1"] in the page');
  boot.canvas = { w: v.canvas.w, h: v.canvas.h };
  boot.canvasDisplay = { w: Math.round(v.canvasRect.w * v.dpr), h: Math.round(v.canvasRect.h * v.dpr) };
  boot.display = boot.canvasDisplay;
  return v;
}

/** the start-up facts of the boot that come from the engine's log: the DPI answers, the faults, the debugLog's first reading (the idle) */
export async function startFacts(ev, boot) {
  const lines = await ev(`(() => { const a = window.__app; return a ? a.engineLog().map((l) => String(l)) : []; })()`);
  boot.api = dpiAnswers(lines);
  // the guest reads LOGPIXELSY (90) and not LOGPIXELSX (88) in this build: the X value is taken only when the Y call is absent
  boot.dpi = boot.api.logpixelsY ?? boot.api.logpixelsX;
  const faults = lines.filter((l) => /fle thread \d+ failed/.test(String(l)));
  boot.engineFaults = { threadFailed: faults.length, first: faults.length ? faults[0].slice(0, 200) : null };
  return scanSafe(ev, DEBUG_MARKERS);
}

/** one static boot at one ratio: the windows of `windows` (each a row), the settings (F10) for the debugLog, and the picture's reference. Any failure is the boot's error, on each row. */
async function staticBoot({ dpr, viewport, port, root, rnd, windows }) {
  const boot = blankBoot(dpr, rnd);
  const state = { state: null };
  const rows = new Map();
  let reference = null;
  let idleDebug = null;
  const profile = path.join(root, `profile-${++runNo}`);
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    const p = await openPage(chrome, dpr, viewport);
    cdp = p.cdp;
    const { S, ev } = p;
    await bootToIdle(S, ev, port, state);
    idleDebug = await startFacts(ev, boot);
    const v = await pageFacts(ev, boot);
    const s0 = await snapshot(ev);
    const fl = flScalingOf(s0, REF);
    const t0 = tempoOf(s0, REF);
    boot.flScalingPercent = fl.flScalingPercent;
    boot.scale = { welcomeW: fl.welcomeW, tempoLen: t0.tempo ? t0.tempo.len : null, ratio: t0.scale };
    reference = { welcomeW: fl.welcomeW, tempoLen: t0.tempo ? t0.tempo.len : null };
    if (windows.length) {
      const inputs = inputsOf(S, ev, v);
      const ctx = { S, ev, v, inputs, pointerPoints: null };
      for (const kind of windows) rows.set(kind, await windowRow(kind, ctx));
      if (ctx.pointerPoints && rows.has('main')) Object.assign(rows.get('main'), await pointerTest(ctx, ctx.pointerPoints));
      boot.settings = await settingsPhase(S, ev, v);
    }
  } catch (e) {
    boot.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  const s = boot.settings;
  const why = s?.why ?? 'no start-up line of FL in the guest memory at the first idle or after the Debug tab was pressed (the markers: DEBUG_MARKERS)';
  boot.debugLog = debugLogOf([['idle', idleDebug], ['debug-tab', s?.debugScan ?? null]], s ? why : 'the Settings window was not reached');
  boot.info = s ? infoOf([['info-tab', s.infoScan]]) : null;
  boot.mainGuiScaling = mainGuiScalingOf(boot.debugLog);
  const out = windows.map((kind) => {
    const r = rows.get(kind) ?? { window: kind, windowRect: null, error: 'not measured: the boot failed before the window', edgeWidth: null, blackShare: null, parts: null, pluginScalePercent: null };
    return { ...rowOfBoot(boot), mode: 'static', step: null, ...r, error: boot.error ?? r.error ?? null };
  });
  return { rows: out, reference, room: boot.room, error: boot.error };
}

/** the row's fields from the boot: the same on every row of the boot */
function rowOfBoot(boot) {
  const { settings, ...rest } = boot;
  return { ...rest, settings, restartNotice: false, dpiCalls: null, displayLines: null, dpiChangedLines: null };
}

/** FL's Main GUI Scaling, as far as the log says: "system set to N PPI" is FL's own line for the system's scale (#1358 10.3). Inferred; the selection itself is not identified. */
function mainGuiScalingOf(debugLog) {
  const f = debugLog && debugLog.scalingFactor;
  return f && /system set to/.test(f.line) ? 'System' : null;
}

/** #1348: waits until the guest's screen is the room's device pixels at the ratio (the engine's report), 15 s at most */
async function waitFollow(ev, dpr) {
  const t0 = Date.now();
  for (;;) {
    const q = await ev(`(() => { const a = window.__app; return { room: a.screen.room, rep: a.screen.reported, dpr: window.devicePixelRatio }; })()`);
    const ok = q.room && q.rep && Math.abs(q.dpr - dpr) < 1e-3 && Math.abs(q.rep.width - Math.round(q.room.width * q.dpr)) <= 1 && Math.abs(q.rep.height - Math.round(q.room.height * q.dpr)) <= 1;
    if (ok) return true;
    if (Date.now() - t0 >= FOLLOW_LIMIT_MS) return false;
    await sleep(500);
  }
}

/** #1348: one dynamic boot: the first ratio at the start, then each following ratio set in turn on the same page (the Welcome window stays open: its width is FL's scale). One row per step. */
async function dynamicBoot({ steps, viewport, port, root, rnd }) {
  const boot = blankBoot(steps[0], rnd);
  const state = { state: null };
  const rows = [];
  const profile = path.join(root, `profile-${++runNo}`);
  let chrome = null, cdp = null;
  try {
    chrome = await launchChrome({ profile, extraArgs: ISOLATION });
    const p = await openPage(chrome, steps[0], viewport);
    cdp = p.cdp;
    const { S, ev } = p;
    await bootToIdle(S, ev, port, state);
    const idleDebug = await startFacts(ev, boot);
    await pageFacts(ev, boot);
    boot.debugLog = debugLogOf([['idle', idleDebug]], 'the dynamic run does not open the Settings window');
    boot.mainGuiScaling = mainGuiScalingOf(boot.debugLog);
    const s0 = await snapshot(ev);
    boot.scale = { welcomeW: flScalingOf(s0, REF).welcomeW, tempoLen: tempoOf(s0, REF).tempo?.len ?? null, ratio: tempoOf(s0, REF).scale };
    for (let i = 0; i < steps.length; i++) {
      const dpr = steps[i];
      const beforeLen = await ev(`window.__app.engineLog().length`);
      if (i > 0) await S('Emulation.setDeviceMetricsOverride', { width: viewport.w, height: viewport.h, deviceScaleFactor: dpr, mobile: false });
      const followed = await waitFollow(ev, dpr);
      await sleep(2000);
      const v = await ev(PAGE);
      const s = await snapshot(ev);
      const t = tempoOf(s, REF);
      const fl = flScalingOf(s, REF);
      const mainRec = s.recs.find((r) => r.cls === 'TFruityLoopsMainForm') ?? null;
      const log = await ev(`window.__app.engineLog().map((l) => String(l))`);
      const fresh = log.slice(beforeLen);
      const disp = fresh.filter((l) => /WM_DISPLAYCHANGE/.test(l));
      const browser = s.recs.find((r) => r.vis && r.cls === 'TEdgeBrowser') ?? null;
      rows.push({
        ...rowOfBoot({ ...boot, dpr: v.dpr, state: v.state, standIn: v.standIn, room: v.room ? { w: v.room.width, h: v.room.height } : null, guest: v.reported && v.reported.width > 0 ? { w: v.reported.width, h: v.reported.height } : null, asked: v.asked, canvas: { w: v.canvas.w, h: v.canvas.h }, canvasDisplay: { w: Math.round(v.canvasRect.w * v.dpr), h: Math.round(v.canvasRect.h * v.dpr) }, display: { w: Math.round(v.canvasRect.w * v.dpr), h: Math.round(v.canvasRect.h * v.dpr) }, flScalingPercent: fl.flScalingPercent, error: v.error ? String(v.error.what ?? v.error).slice(0, 500) : followed ? null : `the guest did not follow the ratio ${dpr} within ${FOLLOW_LIMIT_MS} ms` }),
        mode: 'dynamic',
        step: i + 1,
        window: 'main',
        windowRect: mainRec ? { x: mainRec.ax, y: mainRec.ay, w: mainRec.w, h: mainRec.h, cls: mainRec.cls, title: mainRec.text } : null,
        edgeWidth: t.edgeWidth,
        blackShare: browser ? blackShareOf(s, browser) : null,
        parts: t.parts,
        restartNotice: v.restartNotice,
        diag: { ...v.diag, pageExceptions: cdp.pageExceptions.slice(0, 3) },
        dpiCalls: fresh.filter((l) => /window-api: (GetDeviceCaps|GetDpiForMonitor|GetProcessDpiAwareness)/.test(l)).length,
        displayLines: { count: disp.length, first: disp.slice(0, 2).map((l) => l.slice(0, 200)) },
        dpiChangedLines: fresh.filter((l) => /DPICHANGED/.test(l)).length,
        randomDprs: rnd,
        pluginScalePercent: null,
        pointerErrPx: null,
        pointerSentErrPx: null,
        pointerPoints: null,
        pointerDiag: null,
      });
    }
  } catch (e) {
    const error = String(e?.message ?? e).slice(0, 500);
    for (let i = rows.length; i < steps.length; i++) rows.push({ ...rowOfBoot({ ...boot, error }), mode: 'dynamic', step: i + 1, window: 'main', dpr: steps[i], windowRect: null, edgeWidth: null, blackShare: null, parts: null, restartNotice: null, dpiCalls: null, displayLines: null, dpiChangedLines: null, randomDprs: rnd, pluginScalePercent: null, pointerErrPx: null, pointerSentErrPx: null, pointerPoints: null, pointerDiag: null });
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  return rows;
}

// the viewport that gives the room ROOM_CSS: the page's layout outside the stage is fixed, so the viewport is corrected by the difference the page shows at DPR 1 (at most three boots)
async function calibrate(port, root) {
  let vp = { w: ROOM_CSS.w, h: ROOM_CSS.h };
  for (let i = 0; i < 3; i++) {
    const r = await staticBoot({ dpr: 1, viewport: vp, port, root, rnd: null, windows: [] });
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
    process.stderr.write('usage: node tools/feat-c1-hidpi/measure-size.mjs <absolute path to the bundle directory> [--dprs <list>] [--random <count>] [--random-range <min,max>] [--windows <list>] [--dynamic <list>]\n');
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
      const { rows } = await staticBoot({ dpr, viewport: vp, port, root, rnd, windows: opts.windows });
      for (const r of rows) process.stdout.write(JSON.stringify(r) + '\n');
      process.stderr.write(`static ${dpr}: ${rows.map((r) => `${r.window}${r.error ? ` ERR(${r.error.slice(0, 120)})` : ''}`).join(', ')}\n`);
    }
    if (opts.dynamic) {
      const rows = await dynamicBoot({ steps: opts.dynamic, viewport: vp, port, root, rnd: null });
      for (const r of rows) process.stdout.write(JSON.stringify(r) + '\n');
      process.stderr.write(`dynamic ${opts.dynamic.join(',')}: ${rows.map((r) => `step ${r.step} dpr ${r.dpr} ${r.error ? `ERR(${r.error.slice(0, 120)})` : 'ok'}`).join('; ')}\n`);
    }
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
