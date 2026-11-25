// Issues #1389 and #1406: the engine's Copy and Paste against the browser's clipboard (navigator.clipboard, navigator.permissions), and what a right click on a knob asks of it.
// One start of the bundle in a Chrome of its own per session, at the room of 1836 x 1056 CSS px (as tools/smoke/welcome-flow.mjs).
// DPR: 1 and three random points in 1.01..3.00 (four points). The positions are measured at each DPR, not scaled from DPR 1: FL picks a UI scale per DPR and the layout changes at its steps.
//   calibrateDpr (one session per try, the browser's clipboard permissions 'denied'; tried twice, then the DPR is an error entry with its trace):
//     1. the Problems box's Ok: the window table's child 'Ok' (its centre from the box's top-left corner), else the box's width ratio to 691 (formula);
//     2. the first channel's Volume knob: the pointer is put over the guess and stays there (so the knob's hover look is the same in both pictures), one wheel notch
//        down and back; the changed pixels (outside the windows that the notch opened) form parts; the part nearest the guess is the knob's changed arc (method 'wheel'),
//        and it must lie within 8 px of the guess (the Pan knob, the next one, lies about 25 px to the right). The guess (the formula) is the next candidate, then its
//        four neighbours at 4 px. A candidate is taken when its right click opens the knob's menu and a row of it differs by the permission;
//     3. the menu's 'Paste value' row: the knob's menu is pictured with the permission 'denied' (the row is grey) and 'prompt' (the row is enabled); the band of pixels that differ
//        inside the menu's rectangle is the row; its centre (dy from the menu's top-left) is clicked in the runs.
//     4. the channel name (F2 opens its name box): the formula (the Problems box's width ratio) first, else the knob's measured centre with the same offset;
//        the first candidate where F2 opens the name box is taken (checked by the window table, then Escape).
//   copyCheck (DPR 1): the Copy of the Problems box's 'FLEX (FL)' row gives the engine's text, which navigator.clipboard.readText() returns after it (copied, match).
//   runs (12: permission 'prompt', 'denied', 'granted' x 4 DPR; one session each). Each run:
//     1. the clipboard holds a random number text (writeText, while the permission is granted), then the run's permission is set;
//     2. the knob's right click: its menu shows; 5 s. Counted: the page's readText and read calls, the permission state, the visible status/alert/aria-live elements, the engine's
//        IsClipboardFormatAvailable answers (pasteEnabledOnKnob);
//     3. Ctrl+V in the channel's name box (F2): the paste event (the page's listener), the engine's GetClipboardData text (ctrlVMatch); 5 s. Counted: readText and read calls;
//     4. the knob's menu 'Paste value' (a right click on the knob, then the row): 5 s. Counted: readText and read calls (menuPasteReadCalls), the engine's text (menuPasteMatch),
//        the status element shown by the operation (toastOnMenuPaste);
//     'prompt' runs choose 'Paste value' once more after Browser.setPermission(granted): that second choice gives menuPasteMatch.
//   copyFailure (DPR 1, clipboard-write denied): the Copy of the Problems box's 'FLEX (FL)' row: the page's status element shows the reason (toastShown), hides within 5 s and has a close button.
//   The status element of the page (#clipboard-status, role="status") is sampled every 50 ms in the page: toastCount is the most visible at once, and each shown
//   episode's time to hide (toastHiddenAfter5s: within 5 s, with 300 ms for the sampling).
//   calibration sweep (--calibration, the test aid for the positions): calibrateDpr at 1.00, 1.04, ... 3.00 (51 points), or the list given by --dprs; no runs.
// Usage: node tools/clipboard/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] > clip.json   (needs app/node_modules and app/dist's
//   vite, as tools/smoke/welcome-flow.mjs does). stdout: one JSON line; exit 0 when the JSON was written (its error field says what failed), 2 when no bundle directory is given.
//   node tools/clipboard/measure.mjs <bundle> --calibration [--dprs a,b,...] > cal.json   (calibrations only)
// --shots DIR (a test aid): PNG pictures of the page at the points of the flow are written to DIR (the wheel test's knob pictures, magnified x4, too).
// The JSON: error (null, or the reason), copied, match, text (the engine's text, 200 characters at most), textLength, dprs, calibrations (one per DPR: ok, scale, okAt, the knob
//   and its method, the menu row, the name; a failed DPR has its error and trace: the wheel's changed parts and the menus tried), runs (one object per run: permission, dpr, knobMenuShown, knobReadCalls, permissionAfterKnob, knobVisibleAlerts,
//   pasteEnabledOnKnob, ctrlVMatch, ctrlVReadCalls, pasteEventOnCtrlV, permissionAfterCtrlV, menuPasteReadCalls, menuPasteMatch, toastOnMenuPaste, toastCount,
//   toastHiddenAfter5s, toastHasClose, engineState, and the window-API lines of each step), copyFailure (toastShown, toastHiddenAfter5s, toastHasClose, toastCount).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');

const ROOM_W = 1836, ROOM_H = 1056;   // the room in CSS px at every DPR; the guest screen is the room in device px (ROOM x DPR), and FL draws its UI at the scale of DPR
const VIEW_START = { width: 2176, height: 1092 };
const PERMISSIONS = ['clipboardReadWrite', 'clipboardSanitizedWrite'];
const PERMISSION_NAMES = ['clipboard-read', 'clipboard-write'];
const RUN_PERMISSIONS = ['prompt', 'denied', 'granted'];
const PROBLEMS = /Problems loading/;
const POPUP = 'TQuickPopupMenuWindow';
const NAME_EDIT = 'TNameEditForm';
// guest pixels, measured on the default project at the room of 1836 x 1056 at DPR 1 (the box is 691 guest px wide at scale 1); the copy check runs at DPR 1 only
const FLEX_ROW = { dx: 105, dy: 118 };        // the Problems box's 'FLEX (FL)' row, from the box's top-left corner
const OK = { dx: 368, dyFromBottom: 29 };     // the Problems box's Ok, when the window table has no child 'Ok': scaled by the box's width ratio
const CHANNEL_NAME = { x: 520, y: 144 };      // the first channel's name (F2 opens its name box); scaled by the box's width ratio
const VOLUME_KNOB = { x: 407, y: 144 };       // the first channel's Volume knob (scaled): the guess the wheel test and the menu check start from
const PROBLEMS_W = 691;                       // the Problems box's width at scale 1
const READY_LIMIT_MS = 180000, POLL_MS = 300, CDP_TIMEOUT_MS = 20000, PROBLEMS_LIMIT_MS = 150000;
const OP_WAIT_MS = 5000, TOAST_MAX_MS = 5000, TOAST_SLOP_MS = 300;
const KNOB_HALF = 48, WHEEL_NOTCH = 120;   // the square around the knob's guess that the wheel test reads (guest px), and one notch
const VOLUME_REACH = 8;                     // the wheel's knob must lie this near the guess (the Pan knob, the next one, lies about 25 px right of it)
const NEAR_KNOB = [[-4, 0], [4, 0], [0, -4], [0, 4]];   // the guess's neighbours, tried when the wheel and the guess give no 'Paste value' row
const ROW_MIN_PX = 30, ROW_MAX_H = 40;     // a menu row: at least this many pixels differ, in a band this high at most (guest px)
const CALIB_TRIES = 2;
const CLIP_LINE = /window-api: (OpenClipboard|EmptyClipboard|SetClipboardData|CloseClipboard|IsClipboardFormatAvailable|CountClipboardFormats|EnumClipboardFormats|GetClipboardData)\b/;
const OS_TEXT_GET = /window-api: GetClipboardData fmt=13 -> 0x[0-9a-f]+ \(os text\) bytes=\d+ text="(.*)"$/;
const AVAILABLE_TEXT = /IsClipboardFormatAvailable fmt=(1|7|13) -> 1\b/;

const ISOLATION = [
  '--password-store=basic',
  '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1',
  '--no-proxy-server', '--disable-ipv6', '--disable-quic',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-default-apps', '--disable-domain-reliability',
  '--no-pings', '--metrics-recording-only', '--disable-client-side-phishing-detection', '--disable-breakpad', '--disable-extensions', '--safebrowsing-disable-auto-update', '--disable-search-engine-choice-screen', '--no-service-autorun',
  '--disable-features=OptimizationHints,MediaRouter,Translate,CertificateTransparencyComponentUpdater,NetworkTimeServiceQuerying,AutofillServerCommunication,OptimizationGuideModelDownloading,SafeBrowsingEnhancedProtection',
  '--variations-server-url=http://127.0.0.1:9/', '--disable-field-trial-config',
  '--force-color-profile=srgb', '--disable-lcd-text',
];

// the page's counters, installed once per page: readText and read calls (the page's own readText calls go through them), the paste events, and the status
// elements that are visible (sampled every 50 ms; an episode is one time they are visible, from the first sample to the first sample where they are not)
const INSTRUMENT = `(() => {
  if (window.__clipProbe) return true;
  const api = navigator.clipboard;
  const p = { readText: 0, read: 0, pastes: [], episodes: [], open: null, maxVisible: 0 };
  window.__clipProbe = p;
  const readText = api.readText.bind(api), read = api.read.bind(api);
  api.readText = () => { p.readText++; return readText(); };
  api.read = () => { p.read++; return read(); };
  document.addEventListener('paste', (e) => {
    p.pastes.push({ at: performance.now(), target: (e.target && (e.target.id || e.target.tagName)) || null, text: e.clipboardData ? e.clipboardData.getData('text/plain') : null });
  }, true);
  const LIVE = '[role="status"],[role="alert"],[aria-live]';
  const shown = (el) => !el.hidden && el.getClientRects().length > 0;
  window.__liveBase = new Set([...document.querySelectorAll(LIVE)].filter(shown));
  const fresh = () => [...document.querySelectorAll(LIVE)].filter((el) => shown(el) && !window.__liveBase.has(el)).length;
  window.__liveFresh = fresh;
  setInterval(() => {
    const n = fresh(), t = performance.now();
    if (n > p.maxVisible) p.maxVisible = n;
    if (n > 0 && p.open === null) p.open = { shownAt: t };
    else if (n === 0 && p.open !== null) { p.episodes.push({ shownAt: p.open.shownAt, hiddenAt: t }); p.open = null; }
  }, 50);
  return true;
})()`;
const COUNTERS = `(() => { const p = window.__clipProbe; return { readText: p.readText, read: p.read, pastes: p.pastes.length, open: p.open !== null, maxVisible: p.maxVisible }; })()`;
const PERMISSION_OF = (name) => `navigator.permissions.query({ name: ${JSON.stringify(name)} }).then((p) => p.state, () => 'unknown')`;
const VISIBLE_ALERTS = `window.__liveFresh()`;
const LIVE_BASE = `[...window.__liveBase].map((el) => ({ tag: el.tagName, id: el.id, role: el.getAttribute('role'), live: el.getAttribute('aria-live'), text: el.textContent.trim().slice(0, 40) }))`;
const TOAST_SHOWN = `(() => { const p = window.__clipProbe; return p.open !== null || p.episodes.length > 0; })()`;
const TOAST_CLOSE = `!!document.querySelector('#clipboard-status button')`;
/** the picture of the guest in the rectangle r (guest px, clipped to the picture): RGBA bytes as base64, and the window table of the same picture */
const CAPTURE = (r) => `(async () => {
  const d = await window.__app.deskDumpBase64(false);
  const bin = atob(d.base64);
  const stride = bin.length / d.height;
  const x0 = Math.max(0, ${Math.round(r.x)}), y0 = Math.max(0, ${Math.round(r.y)});
  const w = Math.max(0, Math.min(d.width, ${Math.round(r.x + r.w)}) - x0), h = Math.max(0, Math.min(d.height, ${Math.round(r.y + r.h)}) - y0);
  const out = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++) {
    const o = (y0 + y) * stride + x0 * 4;
    for (let i = 0; i < w * 4; i++) out[y * w * 4 + i] = bin.charCodeAt(o + i);
  }
  let s = '';
  for (let i = 0; i < out.length; i += 0x8000) s += String.fromCharCode(...out.subarray(i, i + 0x8000));
  return { x0, y0, w, h, windows: d.windows, base64: btoa(s) };
})()`;

const scaleOf = (box) => box.w / PROBLEMS_W;
const at = (p, s) => ({ x: Math.round(p.x * s), y: Math.round(p.y * s) });
const round4 = (v) => Math.round(v * 10000) / 10000;
const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

/** the window table: the top-level windows (class, title) and the child windows (parent's lid, visible, class, text); a child's position is in its parent's client px */
export function parseTable(text) {
  const top = [], children = [];
  for (const l of String(text).split('\n')) {
    let m = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (m) {
      const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
      top.push({ lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' });
      continue;
    }
    m = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "([^"]*)"\s*$/.exec(l);
    if (m) children.push({ lid: Number(m[1]), parent: Number(m[2]), x: Number(m[3]), y: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9] });
  }
  return { top, children };
}

/** the pixels (RGB) that differ between two captures of the same rectangle: one byte per pixel, null when the sizes differ */
export function changedMask(a, b) {
  if (a.w !== b.w || a.h !== b.h || a.w * a.h === 0) return null;
  const A = a.pixels, B = b.pixels, m = new Uint8Array(a.w * a.h);
  for (let p = 0; p < m.length; p++) {
    const i = 4 * p;
    if (A[i] !== B[i] || A[i + 1] !== B[i + 1] || A[i + 2] !== B[i + 2]) m[p] = 1;
  }
  return m;
}

/** the bounding box of the set pixels of mask (w pixels a row), or null */
export function changedBox(mask, w) {
  let x0 = Infinity, y0 = Infinity, x1 = -1, y1 = -1, n = 0;
  for (let p = 0; p < mask.length; p++) {
    if (!mask[p]) continue;
    const x = p % w, y = (p - x) / w;
    n++;
    if (x < x0) x0 = x;
    if (x > x1) x1 = x;
    if (y < y0) y0 = y;
    if (y > y1) y1 = y;
  }
  return n ? { x0, y0, x1, y1, n } : null;
}

/** the bands of rows (contiguous rows with a set pixel): { y0, y1, n } with n the set pixels in the band */
export function changedRows(mask, w, h) {
  const bands = [];
  let cur = null;
  for (let y = 0; y < h; y++) {
    let n = 0;
    for (let x = 0; x < w; x++) n += mask[y * w + x];
    if (n > 0) {
      if (!cur) cur = { y0: y, y1: y, n: 0 };
      cur.y1 = y;
      cur.n += n;
    } else if (cur) {
      bands.push(cur);
      cur = null;
    }
  }
  if (cur) bands.push(cur);
  return bands;
}

/** the changed pixels that form one part (8-neighbours, at least 4 pixels) nearest the point at: the parts within 4 px of that part join it. { x0, y0, x1, y1, n } or null */
export function nearestPart(mask, w, h, at) {
  const seen = new Uint8Array(mask.length), parts = [];
  for (let p = 0; p < mask.length; p++) {
    if (!mask[p] || seen[p]) continue;
    const q = [p];
    seen[p] = 1;
    const part = { x0: Infinity, y0: Infinity, x1: -1, y1: -1, n: 0 };
    for (let i = 0; i < q.length; i++) {
      const x = q[i] % w, y = (q[i] - x) / w;
      part.n++;
      part.x0 = Math.min(part.x0, x); part.x1 = Math.max(part.x1, x);
      part.y0 = Math.min(part.y0, y); part.y1 = Math.max(part.y1, y);
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx, ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
          const j = ny * w + nx;
          if (mask[j] && !seen[j]) { seen[j] = 1; q.push(j); }
        }
      }
    }
    if (part.n >= 4) parts.push(part);
  }
  if (parts.length === 0) return null;
  const dist = (r) => Math.hypot((r.x0 + r.x1) / 2 - at.x, (r.y0 + r.y1) / 2 - at.y);
  const near = parts.reduce((a, b) => (dist(b) < dist(a) ? b : a));
  const joined = { ...near };
  for (const r of parts) {
    const gapX = Math.max(0, r.x0 - near.x1, near.x0 - r.x1), gapY = Math.max(0, r.y0 - near.y1, near.y0 - r.y1);
    if (r === near || Math.max(gapX, gapY) > 4) continue;
    joined.x0 = Math.min(joined.x0, r.x0); joined.y0 = Math.min(joined.y0, r.y0);
    joined.x1 = Math.max(joined.x1, r.x1); joined.y1 = Math.max(joined.y1, r.y1);
    joined.n += r.n;
  }
  return joined;
}

/** a PNG of RGBA pixels (w x h), each pixel drawn as a k x k block (a magnifier for the test pictures) */
export function pngOf(pixels, w, h, k = 4) {
  const W = w * k, H = h * k, raw = Buffer.alloc((W * 4 + 1) * H);
  for (let y = 0; y < H; y++) {
    raw[y * (W * 4 + 1)] = 0;
    for (let x = 0; x < W; x++) {
      const s = 4 * ((Math.floor(y / k) * w) + Math.floor(x / k));
      raw.set(pixels.subarray(s, s + 4), y * (W * 4 + 1) + 1 + x * 4);
    }
  }
  const chunk = (type, data) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const crc = Buffer.alloc(4); crc.writeUInt32BE(zlib.crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** the changed pixels inside the box, one string per row ('#' changed, '.' not): a picture for the trace */
export function asciiOf(mask, w, h, box) {
  if (!box) return [];
  const rows = [];
  for (let y = box.y0; y <= box.y1 && y < h; y++) {
    let s = '';
    for (let x = box.x0; x <= box.x1 && x < w; x++) s += mask[y * w + x] ? '#' : '.';
    rows.push(s);
  }
  return rows;
}

const popupAt = (top, c) => top.filter((w) => w.cls === POPUP).sort((a, b) => Math.hypot(a.x - c.x, a.y - c.y) - Math.hypot(b.x - c.x, b.y - c.y))[0] ?? null;
const formulaOk = (box, scale) => ({ dx: Math.round(OK.dx * scale), dy: Math.round(box.h - OK.dyFromBottom * scale), method: 'formula' });
async function waitFor(fn, ms) {
  const t0 = Date.now();
  for (;;) {
    const v = await fn().catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) return null;
    await sleep(POLL_MS);
  }
}

/** one page of the bundle in its Chrome: the keys and the mouse go to the engine as a person's would */
class Page {
  constructor(cdp, sessionId, browserContextId, origin, shots) {
    this.cdp = cdp;
    this.sessionId = sessionId;
    this.browserContextId = browserContextId;
    this.origin = origin;
    this.shots = shots;
  }
  send(method, params = {}) { return timed(this.cdp.send(method, params, this.sessionId), CDP_TIMEOUT_MS, method); }
  async ev(expression) {
    const r = await timed(this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, userGesture: true }), 30000, `evaluate ${expression.slice(0, 60)}`);
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  async table() {
    for (let i = 0; ; i++) {
      try {
        return parseTable(await this.ev('(async () => (await window.__app.deskDumpBase64(false)).windows)()'));
      } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || i > 150) throw e;
        await sleep(100);
      }
    }
  }
  async windows() { return (await this.table()).top; }
  async find(pred) { return (await this.windows()).find(pred) ?? null; }
  async capture(r) {
    for (let i = 0; ; i++) {
      try {
        const d = await this.ev(CAPTURE(r));
        return { x0: d.x0, y0: d.y0, w: d.w, h: d.h, pixels: Buffer.from(d.base64, 'base64'), table: parseTable(d.windows) };
      } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || i > 150) throw e;
        await sleep(100);
      }
    }
  }
  async geometry() { return this.ev('(() => { const g = window.__app.screen.guest; const r = document.getElementById("desktop").getBoundingClientRect(); return { gw: g.width, gh: g.height, x: r.x, y: r.y, w: r.width, h: r.height }; })()'); }
  async point(gx, gy) {
    const g = await this.geometry();
    return { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
  }
  async clickGuest(gx, gy, right = false) {
    const c = await this.point(gx, gy);
    const button = right ? 'right' : 'left';
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(80);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button, buttons: right ? 2 : 1, clickCount: 1, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button, buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(400);
  }
  /** the pointer over a guest pixel, and the wait for its hover look */
  async hover(gx, gy) {
    const c = await this.point(gx, gy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(300);
  }
  /** one wheel notch where the pointer is (no move) */
  async notch(gx, gy, deltaY) {
    const c = await this.point(gx, gy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
    await sleep(300);
  }
  /** wheel notches over a guest pixel (deltaY 120: one notch down, -120 up) */
  async wheel(gx, gy, notches, deltaY) {
    const c = await this.point(gx, gy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(80);
    for (let i = 0; i < notches; i++) {
      await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
      await sleep(300);
    }
  }
  async key(code, key, vk, modifiers = 0) {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
    await sleep(60);
  }
  async escape() { await this.key('Escape', 'Escape', 0x1b); await sleep(800); }
  /** Ctrl+V: Control goes down first (the engine reads the Control state from its own key messages), V, Control up (as tools/p3-audio-probe does for Ctrl+O) */
  async ctrlV() {
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 2 });
    await sleep(40);
    await this.key('KeyV', 'v', 86, 2);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 0xa2, nativeVirtualKeyCode: 0xa2, modifiers: 0 });
    await sleep(40);
  }
  async focusDesktop() { await this.ev("document.getElementById('desktop').focus({ preventScroll: true }); true"); await sleep(100); }
  async shot(name) {
    if (!this.shots) return;
    const s = await this.send('Page.captureScreenshot', { format: 'png' });
    fs.mkdirSync(this.shots, { recursive: true });
    fs.writeFileSync(path.join(this.shots, name), Buffer.from(s.data, 'base64'));
  }
  /** the engine's log lines that are clipboard calls (window-API trace), in order */
  clipLines() { return this.ev(`window.__app.engineLog().filter((l) => ${CLIP_LINE.toString()}.test(l)).map((l) => l.replace(/^\\[\\w+\\] /, ''))`); }
  /** the browser's permission state, set by the test (the page is told by the browser, not by the engine) */
  setPermission(name, setting) { return this.cdp.send('Browser.setPermission', { permission: { name }, setting, origin: this.origin, browserContextId: this.browserContextId }); }
  async setBoth(setting) { for (const name of PERMISSION_NAMES) await this.setPermission(name, setting); }
}

/** the room is 1836 x 1056 CSS px: the window's size is set until the stage has it (the device pixel ratio is the run's) */
async function fitRoom(page, dpr) {
  const W = ROOM_W, H = ROOM_H;
  for (let i = 0; i < 4; i++) {
    const r = await page.ev("(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()");
    if (Math.abs(r.sw - W) <= 0.5 && Math.abs(r.sh - H) <= 0.5) return;
    await page.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (W - r.sw)), height: Math.round(r.ih + (H - r.sh)), deviceScaleFactor: dpr, mobile: false });
    await sleep(1500);
  }
}

/** the engine is idle, the room is fitted, and the Welcome window is closed with Escape: the default project opens, and its "Problems loading the project" box comes */
async function startProject(page, dpr) {
  await page.send('Page.enable');
  await page.send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: dpr, mobile: false });
  await page.send('Page.navigate', { url: `${page.origin}/` });
  const idle = await waitFor(async () => ((await page.ev('window.__app ? window.__app.state : null')) === 'engine-idle' ? true : null), READY_LIMIT_MS);
  if (!idle) throw new Error('the engine did not reach its first idle');
  await fitRoom(page, dpr);
  await sleep(1500);
  log('engine idle', { dpr });
  await page.focusDesktop();
  await page.key('Escape', 'Escape', 0x1b);
  const problems = await waitFor(() => page.find((w) => PROBLEMS.test(w.title)), PROBLEMS_LIMIT_MS);
  if (!problems) throw new Error(`no "Problems loading the project" box within ${PROBLEMS_LIMIT_MS / 1000} s`);
  await sleep(500);
  await page.ev('window.__app.windowApiTrace(true); true');
  await sleep(500);
  return { problems, scale: scaleOf(problems) };
}

/** the box's Ok (the offset from the box's top-left corner) closes it (the knob and the channel are behind it); the offset is the child 'Ok' of the window table, else the formula */
async function okOffset(page, box, scale) {
  const { children } = await page.table();
  const ok = children.find((c) => c.parent === box.lid && c.vis && /^&?ok$/i.test(c.text.trim()));
  if (ok) return { dx: ok.x + Math.round(ok.w / 2), dy: ok.y + Math.round(ok.h / 2), method: 'child' };
  return formulaOk(box, scale);
}
async function closeProblems(page, ok) {
  const box = await page.find((w) => PROBLEMS.test(w.title));
  if (box) await page.clickGuest(box.x + ok.dx, box.y + ok.dy);
  await waitFor(async () => ((await page.find((w) => PROBLEMS.test(w.title))) ? null : true), 10000);
  await sleep(500);
}

/** the engine's Copy (copied, match): the Problems box's 'FLEX (FL)' row, then the Copy popup (DPR 1) */
async function copyCheck(page, out) {
  const { problems, scale } = await startProject(page, 1);
  await page.shot('clip-1-problems.png');
  const before = (await page.clipLines()).length;
  const flex = { x: Math.round(problems.x + FLEX_ROW.dx * scale), y: Math.round(problems.y + FLEX_ROW.dy * scale) };
  await page.clickGuest(flex.x, flex.y);
  const popup = await waitFor(async () => popupAt(await page.windows(), flex), 5000);
  if (!popup) throw new Error('no Copy popup after the click on the item');
  await page.shot('clip-2-popup.png');
  await page.clickGuest(popup.x + popup.w / 2, popup.y + popup.h / 2);
  await sleep(1000);
  await page.shot('clip-3-copied.png');
  const copyLines = (await page.clipLines()).slice(before);
  out.copyCalls = copyLines;
  const engineText = await page.ev('window.__app.clipboard.lastWritten');
  const back = await page.ev('navigator.clipboard.readText().then((t) => t, (e) => "ERROR " + (e && e.name) + ": " + (e && e.message))');
  out.text = typeof engineText === 'string' ? engineText.slice(0, 200) : null;
  out.textLength = typeof engineText === 'string' ? engineText.length : null;
  out.copied = typeof back === 'string' && back.length > 0 && !back.startsWith('ERROR') && back !== 'TIMEOUT';
  out.match = out.copied && typeof engineText === 'string' && back === engineText;
  log('copy', { copied: out.copied, match: out.match, text: out.text, lines: copyLines.length });
  await closeProblems(page, formulaOk(problems, scale));
}

/** a right click on c, and the knob's menu after it (its rectangle as the window table has it, and its picture); null when no menu comes */
async function rightMenu(page, c) {
  await page.clickGuest(c.x, c.y, true);
  const popup = await waitFor(async () => popupAt(await page.windows(), c), 3000);
  if (!popup) return null;
  await sleep(800);
  const rect = { x: popup.x, y: popup.y, w: popup.w, h: popup.h };
  const cap = await page.capture(rect);
  const again = popupAt(cap.table.top, c);
  if (!again || again.x !== popup.x || again.y !== popup.y || again.w !== popup.w || again.h !== popup.h) return null;
  return { cap, popup };
}

/** the knob's menu row 'Paste value': the menu with the permission denied (the row is grey) and with prompt (enabled) differ in the row; the row's band, from the menu's top-left */
async function menuRow(page, c, trace) {
  const t = { method: c.method, at: { x: c.x, y: c.y }, grey: null, enabled: null, bands: null, row: null };
  trace.menus.push(t);
  const rect = (p) => ({ x: p.x, y: p.y, w: p.w, h: p.h });
  await page.setBoth('denied');
  const grey = await rightMenu(page, c);
  if (grey) await page.shot(`${trace.tag}-${c.method}-grey.png`);
  await page.escape();
  if (!grey) return null;
  t.grey = rect(grey.popup);
  await page.setBoth('prompt');
  const enabled = await rightMenu(page, c);
  await page.escape();
  await page.setBoth('denied');
  if (!enabled) return null;
  t.enabled = rect(enabled.popup);
  if (enabled.popup.x !== grey.popup.x || enabled.popup.y !== grey.popup.y || enabled.popup.w !== grey.popup.w || enabled.popup.h !== grey.popup.h) return null;
  const mask = changedMask(grey.cap, enabled.cap);
  if (!mask) return null;
  const all = changedRows(mask, grey.cap.w, grey.cap.h);
  t.bands = all.slice(0, 8).map((b) => ({ y0: b.y0, y1: b.y1, n: b.n }));
  const bands = all.filter((b) => b.n >= ROW_MIN_PX && b.y1 - b.y0 + 1 <= ROW_MAX_H);
  if (bands.length === 0) return null;
  const band = bands.reduce((a, b) => (b.n > a.n ? b : a));
  const cy = grey.cap.y0 + (band.y0 + band.y1 + 1) / 2;
  t.row = { popupW: grey.popup.w, popupH: grey.popup.h, dx: Math.round(grey.popup.w / 2), dy: Math.round(cy - grey.popup.y), n: band.n, height: band.y1 - band.y0 + 1 };
  return t.row;
}

/** the Volume knob's centre from one wheel notch over the guess and back: the pixels that changed outside the windows the notch opened; null when none is a knob-sized change */
async function knobByWheel(page, K, trace) {
  const R = { x: K.x - KNOB_HALF, y: K.y - KNOB_HALF, w: 2 * KNOB_HALF, h: 2 * KNOB_HALF };
  const wt = { guess: K, changed: 0, opened: 0, box: null, reject: null };
  trace.wheel = wt;
  await page.hover(K.x, K.y);   // the pointer stays over the knob in both pictures: its hover look is the same in both, so only the value's change differs
  const a = await page.capture(R);
  await page.notch(K.x, K.y, -WHEEL_NOTCH);
  await sleep(600);
  const b = await page.capture(R);
  await page.shot(`${trace.tag}-wheel.png`);
  await page.notch(K.x, K.y, WHEEL_NOTCH);
  await sleep(1500);
  const opened = b.table.top.filter((w) => !a.table.top.some((v) => v.lid === w.lid));
  wt.opened = opened.length;
  const mask = changedMask(a, b);
  if (!mask) return (wt.reject = 'capture sizes differ'), null;
  const inR = (c) => {
    const p = b.table.top.find((w) => w.lid === c.parent) ?? a.table.top.find((w) => w.lid === c.parent);
    if (!p) return null;
    return { x: p.x + c.x, y: p.y + c.y, w: c.w, h: c.h, vis: c.vis, cls: c.cls, text: c.text.slice(0, 30) };
  };
  const near = (t) => t.children.map(inR).filter((c) => c && c.x < R.x + R.w && c.x + c.w > R.x && c.y < R.y + R.h && c.y + c.h > R.y);
  wt.childrenA = near(a.table);
  wt.childrenB = near(b.table);
  wt.ascii = asciiOf(mask, a.w, a.h, changedBox(mask, a.w));
  if (page.shots) {
    fs.mkdirSync(page.shots, { recursive: true });
    fs.writeFileSync(path.join(page.shots, `${trace.tag}-wheel-a.png`), pngOf(a.pixels, a.w, a.h, 4));
    fs.writeFileSync(path.join(page.shots, `${trace.tag}-wheel-b.png`), pngOf(b.pixels, b.w, b.h, 4));
  }
  for (const w of opened) {
    for (let y = Math.max(0, w.y - a.y0); y < Math.min(a.h, w.y + w.h - a.y0); y++) {
      for (let x = Math.max(0, w.x - a.x0); x < Math.min(a.w, w.x + w.w - a.x0); x++) mask[y * a.w + x] = 0;
    }
  }
  wt.changed = mask.reduce((n, v) => n + v, 0);
  const box = nearestPart(mask, a.w, a.h, { x: K.x - a.x0, y: K.y - a.y0 });
  if (!box) return (wt.reject = 'no change'), null;
  const bw = box.x1 - box.x0 + 1, bh = box.y1 - box.y0 + 1;
  wt.box = { x0: a.x0 + box.x0, y0: a.y0 + box.y0, w: bw, h: bh };
  if (bw > 1.5 * KNOB_HALF || bh > 1.5 * KNOB_HALF) return (wt.reject = 'box too big'), null;
  const x = a.x0 + Math.round((box.x0 + box.x1 + 1) / 2), y = a.y0 + Math.round((box.y0 + box.y1 + 1) / 2);
  if (Math.abs(x - K.x) > VOLUME_REACH || Math.abs(y - K.y) > VOLUME_REACH) return (wt.reject = `not the Volume knob (${x},${y})`), null;
  return { x, y, wheelBox: bw + 'x' + bh };
}

/** the channel name's centre, where F2 opens the name box: the formula (the Problems box's width ratio) first, else the same offset from the measured knob. The first that opens the box */
async function nameAt(page, scale, knob) {
  const formula = at(CHANNEL_NAME, scale);
  const off = { dx: Math.round((CHANNEL_NAME.x - VOLUME_KNOB.x) * scale), dy: Math.round((CHANNEL_NAME.y - VOLUME_KNOB.y) * scale) };
  const candidates = [{ x: formula.x, y: formula.y, method: 'formula' }, { x: knob.x + off.dx, y: knob.y + off.dy, method: 'knob' }];
  const tried = [];
  for (const c of candidates) {
    await page.focusDesktop();
    await page.clickGuest(c.x, c.y);
    await page.key('F2', 'F2', 0x71);
    const box = await waitFor(() => page.find((w) => w.cls === NAME_EDIT), 3000);
    tried.push({ method: c.method, x: c.x, y: c.y, opened: box !== null });
    if (box) {
      await page.escape();
      return { x: c.x, y: c.y, method: c.method, tried };
    }
  }
  throw new Error(`F2 opened no name box at DPR ${scale}: ${JSON.stringify(tried)}`);
}

/** the positions of one DPR, measured: the Problems box's Ok, the Volume knob (its centre), the knob menu's 'Paste value' row and the channel name (F2). The DPR's runs use them */
async function calibrate(bundle, dpr, trace) {
  const page = new Page(bundle.cdp, bundle.sessionId, bundle.browserContextId, bundle.origin, bundle.shots);
  await page.setBoth('denied');
  const { problems, scale } = await startProject(page, dpr);
  trace.scale = round4(scale);
  trace.problems = { x: problems.x, y: problems.y, w: problems.w, h: problems.h };
  const okAt = await okOffset(page, problems, scale);
  trace.okAt = okAt;
  await closeProblems(page, okAt);
  const guess = at(VOLUME_KNOB, scale);
  const wheel = await knobByWheel(page, guess, trace);
  log('knob', { dpr, guess, wheel });
  const candidates = [
    ...(wheel ? [{ x: wheel.x, y: wheel.y, method: 'wheel' }] : []),
    { x: guess.x, y: guess.y, method: 'guess' },
    ...NEAR_KNOB.map(([dx, dy]) => ({ x: guess.x + dx, y: guess.y + dy, method: `near ${dx},${dy}` })),
  ];
  for (const c of candidates) {
    const row = await menuRow(page, c, trace);
    if (row) {
      const name = await nameAt(page, scale, c);
      return { dpr, ok: true, scale: round4(scale), okAt, knob: { x: c.x, y: c.y, method: c.method }, row, name };
    }
  }
  throw new Error(`no knob menu with a 'Paste value' row at DPR ${dpr}`);
}

/** the positions of one DPR, measured, with up to CALIB_TRIES tries; one entry for the DPR: ok, or the last try's error and trace */
async function calibrateDpr(dpr, session1) {
  let last = null;
  for (let attempt = 1; attempt <= CALIB_TRIES; attempt++) {
    const trace = { dpr, attempt, tag: `cal-${dpr}-${attempt}`, menus: [] };
    try {
      const cal = await session1((b) => calibrate(b, dpr, trace));
      log('calibration', { dpr, attempt, ok: true, knob: cal.knob, row: cal.row });
      return { ...cal, attempt };
    } catch (e) {
      last = { dpr, ok: false, attempt, error: String(e?.message ?? e).slice(0, 300), trace };
      log('calibration', { dpr, attempt, ok: false, error: last.error });
    }
  }
  return last;
}

/** one run: the browser's permission state as given, the knob's right click, Ctrl+V, and the knob's 'Paste value' (once more after granted for 'prompt') */
async function runOne(permission, dpr, cal, bundle) {
  const { cdp, browserContextId, origin, shots } = bundle;
  const r = {
    permission, dpr, knobMenuShown: false, knobReadCalls: null, permissionAfterKnob: null, knobVisibleAlerts: null, pasteEnabledOnKnob: false,
    ctrlVMatch: false, ctrlVReadCalls: null, permissionAfterCtrlV: null, pasteEventOnCtrlV: 0, menuPasteReadCalls: null, menuPasteMatch: null, toastOnMenuPaste: false,
    toastCount: 0, toastHiddenAfter5s: true, toastHasClose: true, engineState: null, knobCalls: [], ctrlVCalls: [], menuPasteCalls: [],
  };
  const page = new Page(cdp, bundle.sessionId, browserContextId, origin, shots);
  const random = String(Math.floor(1e8 + Math.random() * 9e8));   // digits: FL takes a clipboard text as a value only when it is a number (its Paste value)
  await startProject(page, dpr);
  await page.ev(INSTRUMENT);
  r.liveBase = await page.ev(LIVE_BASE);
  const wrote = await page.ev(`navigator.clipboard.writeText(${JSON.stringify(random)}).then(() => true, (e) => 'ERROR ' + (e && e.message))`);
  if (wrote !== true) log('the test could not write the browser clipboard', wrote);
  await page.setBoth(permission);
  await closeProblems(page, cal.okAt);
  const counters = () => page.ev(COUNTERS);
  const reads = (a, b) => (b.readText - a.readText) + (b.read - a.read);

  // the knob: a right click on the Volume knob; the menu's 'Paste value' is asked of the clipboard (IsClipboardFormatAvailable, the engine's answer)
  const knob = { x: cal.knob.x, y: cal.knob.y };
  const k0 = await counters(), l0 = (await page.clipLines()).length;
  await page.clickGuest(knob.x, knob.y, true);
  const menu = await waitFor(async () => popupAt(await page.windows(), knob), 3000);
  await sleep(OP_WAIT_MS);
  const k1 = await counters();
  await page.shot(`clip-${permission}-${dpr}-1-knob.png`);
  r.knobMenuShown = !!menu;
  r.knobReadCalls = reads(k0, k1);
  r.permissionAfterKnob = await page.ev(PERMISSION_OF('clipboard-read'));
  r.knobVisibleAlerts = await page.ev(VISIBLE_ALERTS);
  r.knobCalls = (await page.clipLines()).slice(l0);
  r.pasteEnabledOnKnob = r.knobCalls.some((l) => AVAILABLE_TEXT.test(l));
  await page.escape();

  // Ctrl+V in the channel's name box
  await page.focusDesktop();
  const name = cal.name;
  await page.clickGuest(name.x, name.y);
  await page.key('F2', 'F2', 0x71);
  const box = await waitFor(() => page.find((w) => w.cls === NAME_EDIT), 5000);
  if (!box) throw new Error('F2 did not open the name box of the channel');
  await sleep(300);
  const c0 = await counters(), l2 = (await page.clipLines()).length;
  await page.ctrlV();
  await sleep(OP_WAIT_MS);
  const c1 = await counters();
  await page.shot(`clip-${permission}-${dpr}-2-ctrlv.png`);
  r.ctrlVReadCalls = reads(c0, c1);
  r.pasteEventOnCtrlV = c1.pastes - c0.pastes;
  const pastes = await page.ev('window.__clipProbe.pastes');
  r.pasteEvents = pastes.slice(c0.pastes).map((p) => ({ target: p.target, sameText: p.text === random }));
  r.permissionAfterCtrlV = await page.ev(PERMISSION_OF('clipboard-read'));
  r.ctrlVCalls = (await page.clipLines()).slice(l2);
  const got = r.ctrlVCalls.map((l) => OS_TEXT_GET.exec(l)).find((m) => m) ?? null;
  r.ctrlVMatch = got !== null && got[1] === random;
  await page.escape();

  // the knob's 'Paste value': the menu is opened again (a right click), then the row is chosen
  const pasteValue = async (label) => {
    let m = null;
    for (let attempt = 0; attempt < 2 && !m; attempt++) {
      await page.focusDesktop();
      await page.clickGuest(knob.x, knob.y, true);
      m = await waitFor(async () => popupAt(await page.windows(), knob), 3000);
      if (!m) await page.escape();
    }
    if (!m) throw new Error(`no knob menu (${label})`);
    await sleep(600);
    const s0 = await counters(), lines0 = (await page.clipLines()).length, t0 = await page.ev('performance.now()');
    await page.clickGuest(m.x + cal.row.dx, m.y + cal.row.dy);
    await sleep(OP_WAIT_MS);
    const s1 = await counters();
    await page.shot(`clip-${permission}-${dpr}-3-paste-${label}.png`);
    const lines = (await page.clipLines()).slice(lines0);
    const g = lines.map((l) => OS_TEXT_GET.exec(l)).find((x) => x) ?? null;
    const probe = await page.ev('({ episodes: window.__clipProbe.episodes, open: window.__clipProbe.open })');
    const toasts = probe.episodes.filter((e) => e.shownAt >= t0).length + (probe.open && probe.open.shownAt >= t0 ? 1 : 0);
    return { readCalls: reads(s0, s1), match: g !== null && g[1] === random, toast: toasts > 0, lines };
  };
  const first = await pasteValue('1');
  r.menuPasteCalls = first.lines;
  r.menuPasteReadCalls = first.readCalls;
  let last = first;
  if (permission === 'prompt') {
    await page.setBoth('granted');
    const second = await pasteValue('2');
    r.menuPasteCalls = [...first.lines, ...second.lines];
    r.menuPasteReadCalls = first.readCalls + second.readCalls;
    last = second;
  }
  r.menuPasteMatch = permission === 'denied' ? null : last.match;
  r.toastOnMenuPaste = last.toast;

  // every shown episode hides within the time limit (a shown one that is still up is waited for, up to the limit)
  await waitFor(async () => ((await page.ev('window.__clipProbe.open === null')) ? true : null), TOAST_MAX_MS + 1000);
  const final = await page.ev(COUNTERS);
  r.toastCount = final.maxVisible;
  const episodes = await page.ev('window.__clipProbe.episodes');
  r.toastHiddenAfter5s = !final.open && episodes.every((e) => e.hiddenAt - e.shownAt <= TOAST_MAX_MS + TOAST_SLOP_MS);
  r.toastHasClose = episodes.length === 0 || (await page.ev(TOAST_CLOSE));
  r.toastEpisodes = episodes.map((e) => Math.round(e.hiddenAt - e.shownAt));
  r.engineState = await page.ev('window.__app.state');
  r.permissionAfterPaste = await page.ev(PERMISSION_OF('clipboard-read'));
  await page.escape();
  return r;
}

/** the Copy that fails: the browser does not allow the page to write the clipboard (clipboard-write denied); the status element shows the reason, for 5 s at most */
async function copyFailureRun(bundle, out) {
  const page = new Page(bundle.cdp, bundle.sessionId, bundle.browserContextId, bundle.origin, bundle.shots);
  await page.setPermission('clipboard-write', 'denied');
  const { problems, scale } = await startProject(page, 1);
  await page.ev(INSTRUMENT);
  const flex = { x: Math.round(problems.x + FLEX_ROW.dx * scale), y: Math.round(problems.y + FLEX_ROW.dy * scale) };
  await page.clickGuest(flex.x, flex.y);
  const popup = await waitFor(async () => popupAt(await page.windows(), flex), 5000);
  if (!popup) throw new Error('no Copy popup after the click on the item (copyFailure)');
  await page.clickGuest(popup.x + popup.w / 2, popup.y + popup.h / 2);
  await waitFor(() => page.ev(TOAST_SHOWN), 4000);
  await waitFor(async () => ((await page.ev('window.__clipProbe.open === null')) ? true : null), TOAST_MAX_MS + 1000);
  const probe = await page.ev('({ episodes: window.__clipProbe.episodes, open: window.__clipProbe.open, maxVisible: window.__clipProbe.maxVisible })');
  const hasClose = await page.ev(TOAST_CLOSE);
  out.copyFailure = {
    toastShown: probe.episodes.length > 0,
    toastHiddenAfter5s: probe.open === null && probe.episodes.every((e) => e.hiddenAt - e.shownAt <= TOAST_MAX_MS + TOAST_SLOP_MS),
    toastHasClose: hasClose === true,
    toastCount: probe.maxVisible,
    toastEpisodes: probe.episodes.map((e) => Math.round(e.hiddenAt - e.shownAt)),
    message: await page.ev("document.querySelector('#clipboard-status')?.textContent ?? null"),
  };
  log('copyFailure', out.copyFailure);
  await page.escape();
}

async function session(bundleDir, port, shots, out, repoRoot, mode) {
  const root = fs.mkdtempSync(path.join(repoRoot, '.tmp-clip-'));
  const origin = `http://127.0.0.1:${port}`;
  const session1 = async (fn) => {
    let chrome = null, cdp = null;
    try {
      chrome = await launchChrome({ profile: path.join(root, `profile-${Math.random().toString(36).slice(2, 8)}`), extraArgs: ISOLATION });
      cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
      await cdp.opened;
      const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
      const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
      const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
      await cdp.send('Browser.grantPermissions', { browserContextId, origin, permissions: PERMISSIONS });
      return await fn({ cdp, sessionId, browserContextId, origin, shots });
    } finally {
      if (chrome && cdp) await shutdown(chrome, cdp);
    }
  };
  try {
    if (mode.calibrationOnly) {
      for (const dpr of mode.dprs) out.calibrations.push(await calibrateDpr(dpr, session1));
      return;
    }
    await session1(async (b) => copyCheck(new Page(b.cdp, b.sessionId, b.browserContextId, b.origin, b.shots), out));
    const dprs = [1];
    while (dprs.length < 4) {
      const d = Math.round((1.01 + Math.random() * 1.99) * 100) / 100;
      if (!dprs.includes(d)) dprs.push(d);
    }
    out.dprs = dprs;
    for (const dpr of dprs) {
      const cal = await calibrateDpr(dpr, session1);
      out.calibrations.push(cal);
      for (const permission of RUN_PERMISSIONS) {
        const run = cal.ok
          ? await session1((b) => runOne(permission, dpr, cal, b)).catch((e) => ({ permission, dpr, error: String(e?.message ?? e).slice(0, 300) }))
          : { permission, dpr, error: 'no calibration at this DPR' };
        out.runs.push(run);
        log('run', { permission, dpr, knobMenuShown: run.knobMenuShown, pasteEnabledOnKnob: run.pasteEnabledOnKnob, ctrlVMatch: run.ctrlVMatch, menuPasteMatch: run.menuPasteMatch, toastOnMenuPaste: run.toastOnMenuPaste, error: run.error });
      }
    }
    await session1((b) => copyFailureRun(b, out)).catch((e) => { out.copyFailure = { error: String(e?.message ?? e).slice(0, 300) }; });
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

/** the calibration sweep's DPRs: 1.00 to 3.00 in steps of 0.04 (51 points), or the list given by --dprs */
const SWEEP_DPRS = Array.from({ length: 51 }, (_, i) => Math.round((1 + 0.04 * i) * 100) / 100);

async function main() {
  const args = process.argv.slice(2);
  const valueOf = (name) => (args.indexOf(name) >= 0 ? args[args.indexOf(name) + 1] : null);
  const positional = args.filter((a, i) => !a.startsWith('--') && !['--shots', '--dprs'].includes(args[i - 1]));
  const shotsAt = valueOf('--shots');
  const shots = shotsAt ? path.resolve(shotsAt) : null;
  if (positional.length === 0) {
    process.stderr.write('usage: node tools/clipboard/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] [--calibration [--dprs a,b,...]]\n');
    process.exit(2);
  }
  const mode = {
    calibrationOnly: args.includes('--calibration'),
    dprs: valueOf('--dprs') ? valueOf('--dprs').split(',').map((d) => Math.round(Number(d) * 100) / 100) : SWEEP_DPRS,
  };
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const out = { error: null, copied: false, match: false, text: null, textLength: null, copyCalls: [], dprs: [], calibrations: [], runs: [], copyFailure: null };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-clip-assets-'));
  const assets = path.join(root, 'assets');
  let server = null;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    await session(bundleDir, port, shots, out, repo, mode);
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.stdout.write(`${JSON.stringify(out)}\n`);
  log('loadavg', os.loadavg(), 'nproc', os.availableParallelism());
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
