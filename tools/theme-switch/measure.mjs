// Theme switch (Issue #1382): boots the bundle in headless Chrome as the page is served, opens the Settings window (F10), shows its Theme page, and selects each
// .flstheme of the installer list (inputs/fl-installer-files.tsv, Artwork\Themes\) in turn. The Theme page's themes are tiles of one grid (TQuickStringGrid, no text in the window
// table): the tile of a theme is found by its order (themeOrder); the grid is scrolled by dragging its scroll bar's thumb to the top and to the bottom, and the largest offset is measured
// on the picture (gridProfile, scrollShift). Before each selection, and 2 s after it, the tool reads the presents (window.__app.deskStats.presents), the picture (window.__app.deskDumpBase64)
// and the colour of the main window.
// Usage: node tools/theme-switch/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] > theme.jsonl   (needs app/node_modules and app/dist, as boot-bundle.mjs does).
//   --shots DIR (a test aid): the Theme page and each tile before its press are written to DIR as PNG (the directory is made; nothing is removed).
//   --dpr R (a test aid): one run at the ratio R only (the output is one line, for that run).
// One stdout line per run: DPR 1, then a random ratio in 1.01..3.00 (the draw of tools/feat-c1-hidpi/measure-size.mjs). Fields: dpr, randomDprs, room, error (null when none), stopped,
// themes (the .flstheme files listed), switched (the themes whose row was pressed), items (per theme: name, file, found, presentsBefore, presentsAfter, screenChanged, pictureBefore,
// pictureAfter, bgBefore, bgAfter, bgChanged, state), elapsedMs (navigation to the first engine-idle), preBootBytes (the start-up assets' bytes), themePage (the Theme page's
// captions and its children: the discovery of the page), topWindows (the top-level windows after the last selection), stoppedAt and stopLog (on a stop: the engine's last 200 lines).
// sessions (one per engine start: from, to, error, stopped) and stops (one per stop: session, stoppedAt, stopLog). After a stop the engine is started again (a new profile) at the next theme.
// gl (at the first idle: the desktop canvas's size, the browser's WebGL2 limits, and whether a WebGL2 context is made at that size), items[].grid and items[].scroll (the grid's rectangle and the scroll measurement).
// Exit 0 when every run ran without error, 1 when a run failed or the tool failed, 2 on a usage error.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { childWindows, descendantsOf, randomDprs, topWindow } from '../feat-c1-hidpi/measure-size.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const ROOM_CSS = { w: 1836, h: 1056 };
const VIEW_START = { w: 2176, h: 1092 };
const RANDOM_RANGE = [1.01, 3.0];
const HOLD_MS = 2000;
const READY_LIMIT_MS = 180000, SETTLE_MS = 1500, POLL_MS = 500, CDP_TIMEOUT_MS = 60000, RETRY_LIMIT_MS = 15000;
const WIZARD = 'TWelcomeWizard', SETTINGS = 'TMIDIForm', BAR = 'TQuickSheetSelector', SHEET = 'TVectorSheet';
const PAGE_NAME = 'Theme';
const SLOTS = 10;
const LOG_TAIL = 200;
const KEY_F10 = 121, KEY_ESC = 27;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same launch as tools/feat-c1-hidpi/measure-size.mjs)
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

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 600) : null, exit: a.exit ? { reason: a.exit.reason, code: a.exit.code, message: String(a.exit.message ?? '').slice(0, 1400) } : null, presents: a.deskStats ? a.deskStats.presents : null }; })()`;
const READ_TIMES = `(() => { const a = window.__app; return { timeOrigin: performance.timeOrigin, idle: a.timeline.idle ?? null, bytes: (a.assetsReport()?.items ?? []).reduce((n, i) => n + i.bytes, 0) }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const CANVAS = `(() => { const c = document.querySelector('canvas[data-desk="1"]'); if (!c) return null; const r = c.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height, dpr: window.devicePixelRatio, cw: c.width, ch: c.height }; })()`;
const LOG_TAIL_JS = `window.__app.engineLog().slice(-${LOG_TAIL}).map((l) => String(l))`;
// the whole picture drawn again from the window table (a test aid of any display method: with WebGL the peek reads the GL canvas back, without WebGL it fails)
const DUMP = `(async () => { const d = await window.__app.deskDumpBase64(false); return { windows: d.windows, width: d.width, height: d.height, base64: d.base64 }; })()`;
// #1382: the desktop canvas's size and the browser's WebGL2 limits, and whether a WebGL2 context can be made at that size (the probe contexts are lost at once)
const GL_PROBE = `(() => { const c = document.querySelector('canvas[data-desk="1"]'); const w = c ? c.width : null, h = c ? c.height : null;
  const lose = (gl) => { if (gl) gl.getExtension('WEBGL_lose_context')?.loseContext(); };
  const probe = (pw, ph) => { const s = document.createElement('canvas'); s.width = pw; s.height = ph; let gl = null; try { gl = s.getContext('webgl2', { alpha: false, antialias: false, depth: false, stencil: false, preserveDrawingBuffer: true, powerPreference: 'high-performance' }); const ok = !!gl && !gl.isContextLost(); lose(gl); return ok; } catch (e) { lose(gl); return false; } };
  const g = document.createElement('canvas').getContext('webgl2');
  const limits = g ? { maxTexture: g.getParameter(g.MAX_TEXTURE_SIZE), maxRenderbuffer: g.getParameter(g.MAX_RENDERBUFFER_SIZE), maxViewport: Array.from(g.getParameter(g.MAX_VIEWPORT_DIMS)) } : null;
  lose(g);
  return { canvas: { width: w, height: h, area: w !== null && h !== null ? w * h : null, cssWidth: c ? c.clientWidth : null, cssHeight: c ? c.clientHeight : null }, devicePixelRatio: window.devicePixelRatio, limits,
    contextAtCanvasSize: w !== null && h !== null ? probe(w, h) : null, contextAt4096: probe(4096, 4096) }; })()`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const round = (v) => Math.round(v);
const norm = (s) => String(s ?? '').trim().toLowerCase();

/** the .flstheme files of the installer list: the path (as listed) and the display name (the file name without the extension) */
export function themeFiles(tsv) {
  return tsv.replace(/\r/g, '').split('\n').map((l) => l.split('\t')[0]).filter((p) => /^Artwork\\Themes\\.*\.flstheme$/i.test(p))
    .map((file) => ({ file, name: file.split('\\').pop().replace(/\.flstheme$/i, '') }));
}

/** the top-level windows of a window table: { lid, x, y, w, h, cls, title } */
export function topLevels(windows) {
  return windows.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const names = /class "([^"]*)" title "([^"]*)"\s*$/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' };
  });
}

/** the guest position of a child window: its offsets are relative to its parent (the parents are the window table's), the top-level window's are absolute */
export function guestRect(children, top, lid) {
  const byLid = new Map(children.map((c) => [c.lid, c]));
  const at = (id) => {
    if (id === top.lid) return { x: top.x, y: top.y };
    const c = byLid.get(id);
    if (!c) return null;
    const p = at(c.parent);
    return p ? { x: p.x + c.x, y: p.y + c.y } : null;
  };
  const c = byLid.get(lid);
  const p = c ? at(c.parent) : null;
  return c && p ? { x: p.x + c.x, y: p.y + c.y, w: c.w, h: c.h } : null;
}

/** the picture: RGBA rows of `stride` bytes (deskDumpBase64) */
function picture(d) {
  const b = Buffer.from(d.base64, 'base64');
  return { b, stride: b.length / d.height, w: d.width, h: d.height };
}
const hashOf = (b) => crypto.createHash('sha256').update(b).digest('hex').slice(0, 16);

/** the most frequent colour (#rrggbb) of the picture inside the rectangle (every 4th pixel), null when the rectangle is outside. Black is not counted: the surface of a window
 * has pixels that are not drawn (0), and the background is what the theme sets */
export function modeColor(pic, rect) {
  const counts = new Map();
  const x0 = Math.max(0, Math.floor(rect.x)), y0 = Math.max(0, Math.floor(rect.y));
  const x1 = Math.min(pic.w, Math.floor(rect.x + rect.w)), y1 = Math.min(pic.h, Math.floor(rect.y + rect.h));
  for (let y = y0; y < y1; y += 4) {
    for (let x = x0; x < x1; x += 4) {
      const i = y * pic.stride + 4 * x;
      const k = (pic.b[i] << 16) | (pic.b[i + 1] << 8) | pic.b[i + 2];
      if (k !== 0) counts.set(k, (counts.get(k) ?? 0) + 1);
    }
  }
  let best = null, n = -1;
  for (const [k, c] of counts) if (c > n) { n = c; best = k; }
  return best === null ? null : `#${best.toString(16).padStart(6, '0')}`;
}

/** the main window: the largest top-level window that is neither the Welcome window nor the Settings window */
export function mainWindowOf(windows) {
  let best = null;
  for (const t of topLevels(windows)) {
    if (t.cls === WIZARD || t.cls === SETTINGS || t.w <= 0 || t.h <= 0) continue;
    if (best === null || t.w * t.h > best.w * best.h) best = t;
  }
  return best;
}

/** the order of the themes on the Theme page's grid (this build): the themes of the sub-folders first (in the list's order), then the top-level ones (in the list's order).
 * The grid's first tile is the built-in "(Default)", so theme i is tile i + 1. The names of the tiles are read from the screenshots (--shots), not from the window table (the grid has no text there). */
export function themeOrder(files) {
  const sub = (f) => (f.file.split('\\').length > 3 ? 0 : 1);
  return files.slice().sort((a, b) => sub(a) - sub(b));
}

const GRID = 'TQuickStringGrid', SCROLLER = 'TQuickScrollBoxScroller';
// the tiles of the grid: three columns; a row is TILE_PITCH of a column's width (141 / 201 in this build); the tile's picture centre is TILE_IMAGE_DY below the grid's top
const TILE_PITCH = 0.7, TILE_IMAGE_DY = 47, TILE_MARGIN = 24;

/** the luminance of the grid's first column, row by row (guest pixels from the grid's top): the profile that the scroll is measured on */
export function gridProfile(pic, g) {
  const n = Math.max(0, Math.floor(g.h));
  const xs = [g.x + g.w / 6 - 24, g.x + g.w / 6, g.x + g.w / 6 + 24].map((x) => Math.round(x));
  const out = new Array(n).fill(0);
  for (let k = 0; k < n; k++) {
    const y = Math.round(g.y) + k;
    let s = 0, c = 0;
    for (const x of xs) {
      if (x < 0 || x >= pic.w || y < 0 || y >= pic.h) continue;
      const i = y * pic.stride + 4 * x;
      s += 0.299 * pic.b[i] + 0.587 * pic.b[i + 1] + 0.114 * pic.b[i + 2];
      c++;
    }
    out[k] = c ? s / c : 0;
  }
  return out;
}

/** how far the content moved up between two profiles of one grid: d with b[y] = a[y + d] (the least mean difference over the overlap); null when there is no overlap */
export function scrollShift(a, b, maxShift = 320, minOverlap = 0.5) {
  const n = a.length;
  let best = null;
  for (let d = -maxShift; d <= maxShift; d++) {
    let err = 0, c = 0;
    for (let y = 0; y < n; y++) {
      const yy = y + d;
      if (yy < 0 || yy >= n) continue;
      err += Math.abs(a[yy] - b[y]);
      c++;
    }
    if (c < n * minOverlap) continue;
    const m = err / c;
    if (best === null || m < best.err) best = { d, err: m };
  }
  return best ? best.d : null;
}

function openPage(chrome, dpr, viewport) {
  return (async () => {
    const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
    await cdp.opened;
    const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const S = (method, params = {}) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
      return r.result.value;
    };
    await S('Page.enable');
    await S('Emulation.setDeviceMetricsOverride', { width: viewport.w, height: viewport.h, deviceScaleFactor: dpr, mobile: false });
    return { cdp, S, ev };
  })();
}

/** one run: one boot at one ratio, then the Welcome window closed, the Settings window's Theme page, and each theme selected in turn */
/** one session: one boot of the engine, then the themes of `order` from the index `from` on, until the engine stops (the items of the stop are kept) */
async function runOnce(chrome, { dpr, id, port, shots, order, from }) {
  const out = { dpr, room: null, error: null, stopped: false, themes: order.length, switched: 0, items: [], elapsedMs: null, preBootBytes: null, gl: null,
    themePage: null, topWindows: null, stoppedAt: null, stopLog: null, state: null, from };
  let cdp = null;
  try {
    const p = await openPage(chrome, dpr, VIEW_START);
    cdp = p.cdp;
    const { S, ev } = p;
    const navAt = Date.now();
    const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
    if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
    const pressKey = async (code, vk) => {
      await ev("document.getElementById('desktop')?.focus({ preventScroll: true }); true");
      await sleep(100);
      await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
      await sleep(40);
      await S('Input.dispatchKeyEvent', { type: 'keyUp', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
      await sleep(700);
    };
    const dumpNow = async () => {
      const t0 = Date.now();
      for (;;) {
        try {
          return await ev(DUMP);
        } catch (e) {
          if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= RETRY_LIMIT_MS) throw e;
          await sleep(100);
        }
      }
    };
    const windowsText = async () => (await dumpNow()).windows;
    const pressGuest = async (gx, gy) => {
      const rect = await ev(CANVAS);
      const x = rect.x + (gx + 0.5) / rect.dpr, y = rect.y + (gy + 0.5) / rect.dpr;
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      await sleep(50);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      await sleep(600);
    };
    const stateNow = () => ev(STATE);
    // The grid is scrolled with its scroll bar's thumb, not with the wheel: a wheel notch sent at a ratio other than 1 did not reach the grid (measured at 1.23, the picture did not move),
    // while a drag of the thumb did. The thumb is grabbed at the middle of the track (a thumb is most of the track) and dragged past the top or the bottom, where it stops.
    const dragThumb = async (sr, toTop) => {
      const cv = await ev(CANVAS);
      const P = (gx, gy) => ({ x: cv.x + (gx + 0.5) / cv.dpr, y: cv.y + (gy + 0.5) / cv.dpr });
      const cx = sr.x + sr.w / 2, cy = sr.y + sr.h / 2;
      const end = toTop ? sr.y - 300 : sr.y + sr.h + 300;
      const a = P(cx, cy);
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: a.x, y: a.y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await sleep(80);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: a.x, y: a.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      for (let st = 1; st <= 10; st++) {
        const q = P(cx, cy + ((end - cy) * st) / 10);
        await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: q.x, y: q.y, button: 'left', buttons: 1, pointerType: 'mouse' });
        await sleep(120);
      }
      const b = P(cx, end);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: b.x, y: b.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    };
    // the grid's profile once the engine has drawn it again: read after 2 s, and again every 0.5 s until two readings are the same (10 s at most)
    const stableProfile = async (g) => {
      const t0 = Date.now();
      let last = null;
      await sleep(2000);
      for (;;) {
        const pic = picture(await dumpNow());
        const h = hashOf(pic.b);
        if (h === last || Date.now() - t0 >= 10000) return gridProfile(pic, g);
        last = h;
        await sleep(500);
      }
    };
    // the guest point of tile k (0 is the grid's first tile). The grid has two places that the tiles are seen from: the top (offset 0) and the bottom (the largest offset, measured
    // on the picture as the shift between the two). A tile in the top view is taken there; any other is taken at the bottom. null when the tile is not in view.
    const gridPoint = async (g, k, trace, sr) => {
      const colW = g.w / 3;
      const cx = g.x + ((k % 3) + 0.5) * colW;
      const content = g.y + Math.floor(k / 3) * colW * TILE_PITCH + TILE_IMAGE_DY;
      await dragThumb(sr, true);
      const profTop = await stableProfile(g);
      await dragThumb(sr, false);
      const profBottom = await stableProfile(g);
      const range = scrollShift(profTop, profBottom, profTop.length, 0.2);
      trace.push({ range, top: profTop.slice(0, 2).map(round), bottom: profBottom.slice(0, 2).map(round) });
      let y;
      if (content <= g.y + g.h - TILE_MARGIN) {
        await dragThumb(sr, true);
        y = content;
      } else {
        y = content - (range ?? 0);
      }
      trace.push({ content: round(content), y: round(y) });
      if (y < g.y + TILE_MARGIN || y > g.y + g.h - TILE_MARGIN) return null;
      return { x: cx, y };
    };

    // the first idle: the room is ROOM_CSS (the viewport is corrected as welcome-flow does: the layout outside the stage is a fixed offset)
    let state = null;
    for (;;) {
      state = await stateNow().catch(() => null);
      if (state && (state.state === 'engine-idle' || state.state === 'error' || state.state === 'exited')) break;
      if (Date.now() - navAt >= READY_LIMIT_MS) throw new Error(`no engine-idle within ${READY_LIMIT_MS} ms (state ${state ? state.state : 'window.__app not set'})`);
      await sleep(POLL_MS);
    }
    let room = null;
    for (let i = 0; i < 4; i++) {
      room = await ev(ROOM);
      if (Math.abs(room.sw - ROOM_CSS.w) <= 0.5 && Math.abs(room.sh - ROOM_CSS.h) <= 0.5) break;
      await S('Emulation.setDeviceMetricsOverride', { width: Math.round(room.iw + (ROOM_CSS.w - room.sw)), height: Math.round(room.ih + (ROOM_CSS.h - room.sh)), deviceScaleFactor: dpr, mobile: false });
      await sleep(1500);
    }
    out.room = { w: round(room.sw), h: round(room.sh) };
    await sleep(SETTLE_MS);
    state = await stateNow();
    if (state.state !== 'engine-idle') throw new Error(`state ${state.state} before the first idle: ${state.error ?? ''}`.trim());
    const t = await ev(READ_TIMES);
    if (t.idle === null) throw new Error('engine-idle without the idle mark');
    out.elapsedMs = round(t.timeOrigin + t.idle - navAt);
    out.preBootBytes = t.bytes;
    out.gl = await ev(GL_PROBE).catch((e) => `error: ${String(e?.message ?? e).slice(0, 200)}`);

    // the Welcome window and the dialog after it are closed with Escape (while listed, at most four times)
    for (let i = 0; i < 4; i++) {
      if (!/TWelcomeWizard|TMissingPluginsForm/.test(await windowsText())) break;
      await pressKey('Escape', KEY_ESC);
      await sleep(2000);
    }

    // the Settings window (F10), and its tab bar's press that shows the Theme page (the bar's ten slots are split evenly; the caption is the visible page's text)
    const reachTheme = async () => {
      await pressKey('F10', KEY_F10);
      let w = '', top = null;
      for (let i = 0; i < 15 && !top; i++) {
        await sleep(1000);
        w = await windowsText();
        top = topWindow(w, SETTINGS);
      }
      if (!top) throw new Error('the Settings window (TMIDIForm) did not open within 15 s of F10');
      out.settingsOpened = true;
      const captions = [];
      const logFrom = (await ev(`window.__app.engineLog().length`)) ?? 0;
      for (let s = 0; s < SLOTS; s++) {
        const cur = topWindow(w, SETTINGS) ?? top;
        const bar = childWindows(w).find((c) => c.parent === cur.lid && c.cls === BAR);
        if (!bar) throw new Error('no tab bar (TQuickSheetSelector) in the Settings window');
        await pressGuest(cur.x + bar.x + ((s + 0.5) * bar.w) / SLOTS, cur.y + bar.y + bar.h / 2);
        w = await windowsText();
        const kids = childWindows(w);
        const page = kids.find((c) => c.parent === (topWindow(w, SETTINGS) ?? cur).lid && c.cls === SHEET && c.vis) ?? null;
        captions.push(page ? page.text : null);
        if (page && page.text === PAGE_NAME) {
          const now = topWindow(w, SETTINGS) ?? cur;
          await sleep(1000);
          const lines = await ev(`window.__app.engineLog().slice(${logFrom}).map((l) => String(l))`);
          out.themeLog = lines.filter((l) => /Themes|enum-dir|flstheme|Settings/i.test(l)).slice(0, 80).map((l) => l.slice(0, 240));
          if (shots) {
            const sh = await S('Page.captureScreenshot', { format: 'png' });
            fs.mkdirSync(shots, { recursive: true });
            fs.writeFileSync(path.join(shots, `theme-page-${id}.png`), Buffer.from(sh.data, 'base64'));
          }
          out.themePage = { captions, children: descendantsOf(kids, page.lid).slice(0, 200).map((c) => ({ lid: c.lid, cls: c.cls, text: c.text, vis: c.vis, rect: guestRect(kids, now, c.lid) })), top: { x: now.x, y: now.y, w: now.w, h: now.h } };
          return { top: now, page, w, kids };
        }
      }
      out.themePage = { captions, children: [], top: null };
      throw new Error(`the Theme page was not reached (the presses showed: ${JSON.stringify(captions)})`);
    };

    let ctx = await reachTheme();
    for (let i = from; i < order.length; i++) {
      const th = order[i];
      const item = { index: i, name: th.name, file: th.file, tile: null, point: null, found: false, presentsBefore: null, presentsAfter: null, screenChanged: null, pictureBefore: null, pictureAfter: null, bgBefore: null, bgAfter: null, bgChanged: null, state: null };
      out.items.push(item);
      const w = await windowsText();
      const top = topWindow(w, SETTINGS);
      ctx = top ? { ...ctx, top, kids: childWindows(w) } : await reachTheme();
      const page = ctx.kids.find((c) => c.parent === ctx.top.lid && c.cls === SHEET && c.vis && c.text === PAGE_NAME);
      if (!page) throw new Error(`the Theme page is not shown before "${th.name}"`);
      const grid = descendantsOf(ctx.kids, page.lid).find((c) => c.cls === GRID);
      if (!grid) throw new Error('no theme grid (TQuickStringGrid) on the Theme page');
      const g = guestRect(ctx.kids, ctx.top, grid.lid);
      if (!g) throw new Error('the theme grid has no rectangle');
      item.tile = i + 1;
      const scroller = descendantsOf(ctx.kids, page.lid).find((c) => c.cls === SCROLLER);
      const sr = scroller ? guestRect(ctx.kids, ctx.top, scroller.lid) : null;
      if (!sr) throw new Error('no scroll bar (TQuickScrollBoxScroller) on the theme grid');
      const trace = [];
      const pt = await gridPoint(g, i + 1, trace, sr);
      item.point = pt ? { x: round(pt.x), y: round(pt.y) } : null;
      item.grid = { x: round(g.x), y: round(g.y), w: round(g.w), h: round(g.h) };
      item.scroll = trace;
      if (!pt) continue;
      item.found = true;
      if (shots) {
        fs.mkdirSync(shots, { recursive: true });
        const sh = await S('Page.captureScreenshot', { format: 'png' });
        fs.writeFileSync(path.join(shots, `theme-${id}-${i}-tile.png`), Buffer.from(sh.data, 'base64'));
      }
      const d0 = await dumpNow();
      const p0 = picture(d0), m0 = mainWindowOf(d0.windows);
      item.presentsBefore = (await stateNow()).presents;
      item.pictureBefore = hashOf(p0.b);
      item.bgBefore = m0 ? modeColor(p0, m0) : null;
      await pressGuest(pt.x, pt.y);
      out.switched++;
      await sleep(HOLD_MS);
      const s1 = await stateNow();
      item.presentsAfter = s1.presents;
      const d1 = await dumpNow();
      const p1 = picture(d1), m1 = mainWindowOf(d1.windows);
      item.pictureAfter = hashOf(p1.b);
      item.screenChanged = item.pictureBefore !== item.pictureAfter;
      item.bgAfter = m1 ? modeColor(p1, m1) : null;
      item.bgChanged = item.bgBefore !== null && item.bgAfter !== null && item.bgBefore !== item.bgAfter;
      item.state = s1.state;
      out.topWindows = topLevels(d1.windows).map((t) => ({ cls: t.cls, title: t.title, w: t.w, h: t.h }));
      log(`theme ${i} ${th.name}: presents ${item.presentsBefore} -> ${item.presentsAfter}, picture ${item.pictureBefore} -> ${item.pictureAfter}, bg ${item.bgBefore} -> ${item.bgAfter}, state ${s1.state}`);
      if (s1.exit !== null || s1.state === 'exited' || s1.state === 'error') {
        out.stopped = true;
        out.stoppedAt = { index: i, name: th.name, file: th.file, state: s1.state, exit: s1.exit, error: s1.error };
        out.stopLog = await ev(LOG_TAIL_JS);
        break;
      }
      for (let k = 0; k < 20; k++) {
        const s2 = await stateNow();
        if (s2.state === 'engine-idle' || s2.state === 'exited' || s2.state === 'error') break;
        await sleep(POLL_MS);
      }
    }
    if (!out.stopped) {
      const end = await stateNow().catch(() => null);
      if (end && (end.state === 'exited' || end.state === 'error' || end.exit !== null)) {
        out.stopped = true;
        out.stoppedAt = { index: null, name: null, file: null, state: end.state, exit: end.exit, error: end.error };
        out.stopLog = await ev(LOG_TAIL_JS).catch(() => null);
      }
    }
    out.state = (await stateNow().catch(() => null))?.state ?? null;
    if (out.error === null && out.stopped) out.error = out.stoppedAt.error ?? (out.stoppedAt.exit ? `${out.stoppedAt.exit.reason} ${out.stoppedAt.exit.code}` : 'stopped');
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 600);
  } finally {
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
  }
  log(`run ${id}: dpr ${dpr}, error ${out.error}, stopped ${out.stopped}, switched ${out.switched}/${out.themes}, elapsedMs ${out.elapsedMs}`);
  return out;
}

async function main() {
  const argv = process.argv.slice(2);
  const shotsAt = argv.indexOf('--shots');
  const shots = shotsAt >= 0 ? path.resolve(argv[shotsAt + 1] ?? '') : null;
  const dprAt = argv.indexOf('--dpr');
  const fixedDpr = dprAt >= 0 ? Number(argv[dprAt + 1]) : null;
  const positional = argv.filter((a, i) => !a.startsWith('--') && (shotsAt < 0 || i !== shotsAt + 1) && (dprAt < 0 || i !== dprAt + 1));
  if (positional.length !== 1 || (shotsAt >= 0 && !argv[shotsAt + 1]) || (dprAt >= 0 && !(fixedDpr >= 1))) {
    process.stderr.write('usage: node tools/theme-switch/measure.mjs <absolute path to the bundle directory raw/<name>> [--shots <directory>] [--dpr <ratio>, one run only (a test aid)]\n');
    process.exitCode = 2;
    return;
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-theme-'));
  const assets = path.join(root, 'assets');
  let server = null, failed = false;
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const order = themeOrder(themeFiles(fs.readFileSync(path.join(repo, 'inputs', 'fl-installer-files.tsv'), 'utf8')));
    // one DPR: the themes in order; after a stop the engine is started again (a new profile) at the theme after the one that stopped, so that every theme is tried
    const runDpr = async (id, dpr, rnd) => {
      const merged = { dpr, randomDprs: rnd ?? null, room: null, error: null, stopped: false, themes: order.length, switched: 0, items: [], elapsedMs: null, preBootBytes: null, gl: null,
        themePage: null, topWindows: null, stoppedAt: null, stopLog: null, state: null, tileOrder: order.map((t) => t.name), stops: [], sessions: [] };
      let from = 0;
      for (let s = 0; from < order.length; s++) {
        const chrome = await launchChrome({ profile: path.join(root, `profile-${id}-${s}`), extraArgs: ISOLATION });
        let r;
        try {
          r = await runOnce(chrome, { dpr, id: `${id}.${s}`, port, shots, order, from });
        } finally {
          chrome.proc.kill('SIGKILL');
        }
        merged.sessions.push({ session: s, from, to: from + r.items.length - 1, error: r.error, stopped: r.stopped, elapsedMs: r.elapsedMs });
        if (s === 0) Object.assign(merged, { room: r.room, elapsedMs: r.elapsedMs, preBootBytes: r.preBootBytes, gl: r.gl, themePage: r.themePage });
        merged.items.push(...r.items);
        merged.switched += r.switched;
        merged.topWindows = r.topWindows;
        merged.state = r.state;
        if (r.stopped) {
          merged.stopped = true;
          merged.stops.push({ session: s, stoppedAt: r.stoppedAt, stopLog: r.stopLog });
        }
        if (r.error !== null) merged.error = merged.error ?? r.error;
        if (r.items.length === 0) break;
        from += r.items.length;
      }
      merged.stoppedAt = merged.stops[0]?.stoppedAt ?? null;
      merged.stopLog = merged.stops[0]?.stopLog ?? null;
      return merged;
    };
    if (fixedDpr !== null) {
      const r = await runDpr('x', fixedDpr, null);
      failed = r.error !== null;
      process.stdout.write(JSON.stringify(r) + '\n');
      process.exitCode = failed ? 1 : 0;
      return;
    }
    const r1 = await runDpr(1, 1, null);
    failed = failed || r1.error !== null;
    process.stdout.write(JSON.stringify(r1) + '\n');
    const rnd = randomDprs(1, RANDOM_RANGE, undefined, [1]);
    const r2 = await runDpr(2, rnd.values[0], rnd);
    failed = failed || r2.error !== null;
    process.stdout.write(JSON.stringify(r2) + '\n');
  } catch (e) {
    failed = true;
    process.stderr.write(`theme-switch: ${String(e?.stack ?? e).slice(0, 800)}\n`);
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  process.exitCode = failed ? 1 : 0;
}

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
if (isMain) await main();
