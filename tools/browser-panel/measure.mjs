// browser-panel: the measurement of the Browser panel on the left (#1355). It boots the bundle the way tools/boot-check/boot-bundle.mjs does (headless Chrome, a new profile, the page at "/",
// no page parameters, the same isolation flags), at device pixel ratio 1, with the room (#stage) fitted to 1836 x 1056 CSS px as tools/p2-frames/measure.mjs does. It records and judges nothing else.
// Usage: node tools/browser-panel/measure.mjs <absolute path to the bundle directory raw/<name>> [<png prefix>] > browser.json   (needs app/node_modules and app/dist). Exit 1 when error is set, 2 when no bundle directory is given.
//   Issue #1362: before the judgement the Welcome window is closed (Escape, and the project opens: flow B of tools/smoke/welcome-flow.mjs), the "Problems loading the project" box is closed (Escape),
//   the first tab (guest 35,125) is pressed and then the cloud tab (guest 267,125): the cloud tab is selected at the start, and its Navigate comes from a press that selects it. <png prefix>, when given, writes
//   <prefix>-before.png (before the cloud tab's press) and <prefix>-after.png (after the settle time): a test aid, nothing is removed.
// stdout: ONE JSON line {dpr, state, standIn, error, blackShare, browserComponents, elapsedMs, preBootBytes, browserWindow}. stderr: one line per step (the room, the windows, the rectangle used).
//   elapsedMs: from the navigation to the first poll that sees state 'engine-idle' (the poll is every 100 ms).
//   preBootBytes: the decoded bytes of the start-up assets (window.__app.assetsReport().items, bytes), as the page's report lists them: the engine wasm, the DLL pack, the seed pack and the engine glue.
//     The same count as the 314,326,602 B that #1345 wrote in the issue body. The lazy files come after the engine-idle and are not in the list.
//   browserWindow: {source: 'window-table' | 'fallback', x, y, w, h (guest px), className, title, cssRect (CSS px, from the room's origin)} or null. The window table is window.__app.summary.windows
//     (the guest's top-level windows). The Browser window is looked for by "browser" in its title or class (case-insensitive). The table does not hold it: the Browser panel is a child window
//     of the main form (TWPControl, lid 49, with TSampleListForm, lid 68, in it; #1355 probe: the window-API trace gives its rectangle as abs (-5, 76) 364 x 974 in the guest, the list pane
//     at (-5, 139) 364 x 905). So the fallback is that panel's rectangle, x 0, y 76, 364 x 974 CSS px from the room's origin (the room's origin is the guest's origin at DPR 1), clamped to the picture.
//     The earlier rectangle (x 0, y 580, 400 x 470) covered only the lower part of the list and 36 px of the Playlist, so it is not used.
//   blackShare (Issue #1362): the fraction (0..1) of the pixels whose R, G and B are all <= 16, in the PAGE's own picture (CDP Page.captureScreenshot of the Browser's area: x 0, y 76, 364 x 974 guest px at the desktop's scale).
//   guestBlackShare: the same fraction of the compositor's picture (window.__app.deskDumpBase64(true), guest px), the measure of #1355. Not judged.
//   cloudPressed: the cloud tab (guest 267,125) was pressed once before the judgement (probe-navigate.mjs's press).
//   iframeCount: the iframes of the page that carry the mark data-fl-webview2 (the WebView2 face: app/src/ui/webview2-surface.ts). iframeRect: the first one's rectangle, CSS px from the room's origin (#stage), or null.
//   boundsRect: the last put_Bounds of the engine's log (its bounds and origin=X,Y of the window: the guest's pixels) in the same frame, at the desktop's scale. A window whose origin is negative (the list pane: x -5) is placed at its negative origin, and the face is clipped to the desktop (its rectangle is not).
//   boundsMatch: true when iframeCount >= 1 and each of x, y, w, h of iframeRect and boundsRect differs by at most 2 CSS px.
//   browserComponents: the number of 8-connected groups of the pixels of the rectangle that are not the background colour (the most common of the four corners' colours), with at least 16 pixels.
//   webview2Lines: the number of the engine's log lines that begin with "webview2: " (fle_webview2.c: what FLEngine hands to the WebView2 model; the engine's stderr, window.__app.engineLog()).
//   webview2Methods: the names of the methods in those lines, in the order of their first appearance, each once (the fire lines are not methods: they are left out). The lines themselves go to stderr (the record of #1355). Judged: only that CreateCoreWebView2EnvironmentWithOptions is in it.
//   webview2Fired: the event names of the fire lines (`webview2: fire <event> ...`, the events the model called on the page's handlers), in the order of their first appearance, each once. Judged: NavigationCompleted is in it.
//   navigateCalls: the number of the lines of Navigate, NavigateToString, ExecuteScript, PostWebMessageAsJson and PostWebMessageAsString (what FL hands to the WebView2 for a page). Not judged (the record of #1355).
//   Issue #1376: states.<welcome|frontWindow|fileMenu|none> (in this order, one start): the front windows are window.__app.webview2Front() (the windows painted over the face, from the window table).
//     overlapped: a front rectangle meets the face's rectangle (not for none). frontRect: the front rectangle with the largest overlap (CSS px from #stage; null when none). frontCount: the front rectangles.
//     overlapPoints: the grid points (2 CSS px) of every front rectangle shrunk by 2 CSS px that elementFromPoint gives the face for (judged 0). bottomHit (fileMenu): the tagName 4 CSS px inside the bottom edge of the largest (judged CANVAS).
//     method: 'as-is' (overlapped without a move), 'drag' (the window that the list gives for the state was dragged from its top edge to BROWSER_TARGET, guest px), 'no window' (none was there to drag). window: {cls, title} of the dragged one.
//     startStalls: the starts that did not reach engine-idle within START_LIMIT_MS and were loaded again (START_TRIES = 4; the judgement uses the start that reached it).
//     none.clipMatchesBounds: boundsMatch and the face's clip-path is empty. The order: welcome (the start), Escape (the Welcome closes), frontWindow (the box, dragged over the Browser if it is not), Escape, the cloud tab's press, fileMenu (click at guest 18,20), Escape, none.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';

if (!process.argv[2]) {
  process.stderr.write('usage: node tools/browser-panel/measure.mjs <absolute path to the bundle directory>\n');
  process.exit(2);
}
const bundleDir = path.resolve(process.argv[2]);
const BUNDLE = path.basename(bundleDir);
const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');

const DPR = 1;
const ROOM_W = 1836, ROOM_H = 1056;
// the window that gives the room at the first layout (the page's right panel is 340 px wide and its header 36 px high; the fit below corrects a difference)
const VIEW_START = { width: 2176, height: 1092 };
const FALLBACK_CSS = { x: 0, y: 76, w: 364, h: 974 };
const BLACK_MAX = 16;
const COMPONENT_MIN_PX = 16;
const WAIT_LIMIT_MS = 180000;
const POLL_MS = 100;
const SETTLE_MS = 5000;
const CDP_TIMEOUT_MS = 20000;

// the isolation flags of tools/boot-check/boot-bundle.mjs (the same launch)
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

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, standIn: document.body.innerText.includes('STAND-IN') }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); const r = document.getElementById('desktop').getBoundingClientRect(); const a = window.__app; return { sw: s.width, sh: s.height, x: r.x, y: r.y, w: r.width, h: r.height, iw: window.innerWidth, ih: window.innerHeight, dpr: window.devicePixelRatio, guest: a && a.screen ? a.screen.guest : null }; })()`;
const FINAL = `(() => { const a = window.__app ?? null; return {
  state: a ? a.state : null,
  error: a && a.error ? String(a.error.what ?? a.error) : null,
  standIn: document.body.innerText.includes('STAND-IN'),
  windows: a && a.summary ? a.summary.windows : null,
  assets: a && a.assetsReport ? (a.assetsReport() ?? null) : null,
  dpr: window.devicePixelRatio,
}; })()`;
const PICTURE = `window.__app.deskDumpBase64(true)`;
// Issue #1362: the cloud tab (guest 267,125; the press of probe-navigate.mjs), the face of the WebView2 (the iframe the page puts over the desktop), and the Browser's area in the page's own picture
const TAB_CLOUD = { x: 267, y: 125 };
const TAB_OTHER = { x: 35, y: 125 };
const TAB_SETTLE_MS = 3000;
const OPEN_WAIT_MS = 120000;
const DISMISS_MS = 2000;
const TITLE = `document.title`;
const FACE_WAIT_MS = 15000;
const BOUNDS_MATCH_CSS = 2;
const BROWSER_GUEST = { x: 0, y: 76, w: 364, h: 974 };
// Issue #1376: the front windows (window.__app.webview2Front(): l, t, r, b per window painted over the face, guest px on the guest's screen) and the window list (the describe text of the picture)
const FRONT = `window.__app.webview2Front()`;
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const WINDOW_LINE = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+).* class "([^"]*)" title "([^"]*)"$/;
const WELCOME_CLASS = 'TWelcomeWizard';
const MENU_CLASS = 'TQuickPopupMenuWindow';
const MENU_AT = { x: 18, y: 20 };                                  // the File menu: a click at guest (18, 20) (tools/verify-kit/tools/sdriveg2-export.mjs)
const BROWSER_TARGET = { x: BROWSER_GUEST.x + 10, y: BROWSER_GUEST.y + 250 };   // a window that is moved over the Browser: its top-left corner goes to this guest point
const MAIN_SHARE = 0.8;                                            // a top-level window of at least this share of the guest's screen in both directions is the main form
const STATE_SETTLE_MS = 1500;
const DRAG_SETTLE_MS = 1500;
const DRAG_STEPS = 12;
const CANDIDATE_WAIT_MS = 20000;
const MENU_WAIT_MS = 6000;
const WINDOW_POLL_MS = 500;
const START_LIMIT_MS = 40000;                                   // a start that has not reached engine-idle by then is loaded again (START_TRIES times; the last one has WAIT_LIMIT_MS)
const START_TRIES = 4;
/** the top-level windows of a describe text (lid, x, y, w, h in guest px, cls, title) */
export function parseWindows(text) {
  return String(text ?? '').split('\n').map((l) => WINDOW_LINE.exec(l)).filter((m) => m !== null).map((m) => ({ lid: +m[1], x: +m[2], y: +m[3], w: +m[4], h: +m[5], cls: m[6], title: m[7] }));
}
/** the area in which two rectangles {x, y, w, h} overlap (0 when they do not) */
export function overlapArea(a, b) {
  const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x), h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return w > 0 && h > 0 ? w * h : 0;
}
/** the grid points (2 CSS px) of each front rectangle, shrunk by 2 CSS px on each side: the number of them, and of those where elementFromPoint gives the face (the mark FACE_ATTR) */
const HIT_POINTS = (rects) => `(() => { const s = document.getElementById('stage').getBoundingClientRect(); let points = 0, hits = 0; for (const r of ${JSON.stringify(rects)}) { for (let x = r.x + 2; x <= r.x + r.w - 2; x += 2) for (let y = r.y + 2; y <= r.y + r.h - 2; y += 2) { points++; const e = document.elementFromPoint(Math.floor(s.x + x), Math.floor(s.y + y)); if (e && e.hasAttribute('data-fl-webview2')) hits++; } } return { points, hits }; })()`;
/** the tagName of the element 4 CSS px inside the bottom edge of the rectangle r (the middle of its width) */
const BOTTOM_HIT = (r) => `(() => { const s = document.getElementById('stage').getBoundingClientRect(); const e = document.elementFromPoint(Math.floor(s.x + ${r.x + r.w / 2}), Math.floor(s.y + ${r.y + r.h - 4})); return e ? e.tagName : null; })()`;
const round3 = (r) => ({ x: Math.round(r.x * 1000) / 1000, y: Math.round(r.y * 1000) / 1000, w: Math.round(r.w * 1000) / 1000, h: Math.round(r.h * 1000) / 1000 });
const FACE = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); const d = document.getElementById('desktop').getBoundingClientRect(); const a = window.__app; const g = a && a.screen && a.screen.guest ? a.screen.guest : null; const frames = Array.from(document.querySelectorAll('iframe[data-fl-webview2]')); const f = frames[0] ? frames[0].getBoundingClientRect() : null; return { stage: { x: s.x, y: s.y }, desk: { x: d.x, y: d.y, w: d.width, h: d.height }, guestW: g ? g.width : null, guestH: g ? g.height : null, clip: frames[0] ? frames[0].style.clipPath : null, count: frames.length, rect: f ? { x: f.x - s.x, y: f.y - s.y, w: f.width, h: f.height } : null, srcdocLength: frames[0] ? (frames[0].srcdoc || '').length : null, frameBackground: frames[0] ? getComputedStyle(frames[0]).backgroundColor : null }; })()`;
const PUT_BOUNDS = /put_Bounds \(\S+\) bounds=(-?\d+),(-?\d+),(-?\d+),(-?\d+)(?: origin=(-?\d+),(-?\d+))?/;
/** the number of pixels of a PNG (base64, decoded in the page) whose R, G and B are all <= BLACK_MAX, and the number of pixels */
const countBlack = (b64) => `(async () => { const img = new Image(); img.src = 'data:image/png;base64,${b64}'; await img.decode(); const c = document.createElement('canvas'); c.width = img.width; c.height = img.height; const g = c.getContext('2d'); g.drawImage(img, 0, 0); const px = g.getImageData(0, 0, c.width, c.height).data; let black = 0; for (let i = 0; i < px.length; i += 4) if (px[i] <= ${BLACK_MAX} && px[i + 1] <= ${BLACK_MAX} && px[i + 2] <= ${BLACK_MAX}) black++; return { black, n: c.width * c.height }; })()`;
const ENGINE_LOG = `window.__app.engineLog()`;
const WEBVIEW2_LINE = /^(?:\[err\] )?webview2: (\S+)/;
const WEBVIEW2_FIRE = /^(?:\[err\] )?webview2: fire (\S+)/;
const NAVIGATE_METHODS = new Set(['Navigate', 'NavigateToString', 'ExecuteScript', 'PostWebMessageAsJson', 'PostWebMessageAsString']);

const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};
const log = (...a) => process.stderr.write(a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ') + '\n');

/** a key press and release (probe-navigate.mjs's key) */
async function pressKey(S, name, vk) {
  await S('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  await sleep(40);
  await S('Input.dispatchKeyEvent', { type: 'keyUp', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}

/** a left click at the CSS point c (move, press, release: the press of probe-navigate.mjs) */
async function pressCss(S, c) {
  await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, clickCount: 0, pointerType: 'mouse' });
  await sleep(60);
  await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
  await sleep(50);
  await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
}

/** a picture: the RGBA bytes of a dump (its rows may be longer than width x 4) */
function picture(d) {
  const bytes = Buffer.from(d.base64, 'base64');
  const stride = bytes.length / d.height;
  if (!Number.isInteger(stride) || stride < d.width * 4) throw new Error(`the picture has ${bytes.length} bytes for ${d.width}x${d.height}`);
  return { w: d.width, h: d.height, bytes, stride };
}

/** the rectangle clamped to the picture: { x, y, w, h } in guest px */
function clampRect(pic, r) {
  const x0 = Math.max(0, Math.floor(r.x)), y0 = Math.max(0, Math.floor(r.y));
  const x1 = Math.min(pic.w, Math.ceil(r.x + r.w)), y1 = Math.min(pic.h, Math.ceil(r.y + r.h));
  return { x: x0, y: y0, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0) };
}

/** blackShare and browserComponents of the rectangle r (guest px) of pic */
export function judgePicture(pic, r) {
  const n = r.w * r.h;
  if (n === 0) return { blackShare: null, browserComponents: null, background: null, pixels: 0 };
  const px = new Uint32Array(n);
  let black = 0;
  for (let y = 0; y < r.h; y++) {
    for (let x = 0; x < r.w; x++) {
      const o = (r.y + y) * pic.stride + (r.x + x) * 4;
      const R = pic.bytes[o], G = pic.bytes[o + 1], B = pic.bytes[o + 2];
      if (R <= BLACK_MAX && G <= BLACK_MAX && B <= BLACK_MAX) black++;
      px[y * r.w + x] = (R << 16) | (G << 8) | B;
    }
  }
  const corners = [px[0], px[r.w - 1], px[(r.h - 1) * r.w], px[n - 1]];
  let bg = corners[0], most = 0;
  for (const c of corners) {
    const k = corners.filter((v) => v === c).length;
    if (k > most) { most = k; bg = c; }
  }
  const seen = new Uint8Array(n), stack = new Int32Array(n);
  let groups = 0;
  for (let s = 0; s < n; s++) {
    if (seen[s] || px[s] === bg) continue;
    let sp = 0, size = 0;
    stack[sp++] = s;
    seen[s] = 1;
    while (sp > 0) {
      const q = stack[--sp], x = q % r.w, y = (q - x) / r.w;
      size++;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx, ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= r.w || ny >= r.h) continue;
          const k = ny * r.w + nx;
          if (!seen[k] && px[k] !== bg) { seen[k] = 1; stack[sp++] = k; }
        }
      }
    }
    if (size >= COMPONENT_MIN_PX) groups++;
  }
  return { blackShare: black / n, browserComponents: groups, background: [(bg >>> 16) & 255, (bg >>> 8) & 255, bg & 255], pixels: n };
}

/** the Browser window in a window list (summary.windows: className, title, x, y, width, height, visible), or null */
export function findBrowser(windows) {
  const hit = (windows ?? []).filter((w) => /browser/i.test(w.title ?? '') || /browser/i.test(w.className ?? ''));
  return hit.find((w) => w.visible) ?? hit[0] ?? null;
}

async function main() {
  const t0 = Date.now();
  const out = { dpr: DPR, state: null, standIn: null, error: null, blackShare: null, guestBlackShare: null, browserComponents: null, elapsedMs: null, preBootBytes: null, browserWindow: null, webview2Lines: null, webview2Methods: null, webview2Fired: null, navigateCalls: null, cloudPressed: false, iframeCount: 0, iframeRect: null, boundsRect: null, boundsMatch: false, startStalls: 0, states: {} };
  const root = fs.mkdtempSync(path.join(repo, '.tmp-browser-panel-'));
  const assets = path.join(root, 'assets');
  const profile = path.join(root, 'profile');
  let server = null, chrome = null, cdp = null, navAt = null;
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
    const S = (method, params) => timed(cdp.send(method, params, sessionId), CDP_TIMEOUT_MS, method);
    const ev = async (expression) => {
      const r = await S('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
      return r.result.value;
    };
    await S('Page.enable');
    await S('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: DPR, mobile: false });

    const waitState = async (limitMs) => {
      const t = Date.now();
      for (;;) {
        const s = await ev(STATE).catch(() => null);
        if (s && (s.state === 'engine-idle' || s.state === 'error' || s.state === 'exited')) return { state: s.state, at: Date.now() };
        if (Date.now() - t >= limitMs) return { state: s ? s.state : null, at: null };
        await sleep(POLL_MS);
      }
    };
    // the first engine-idle: elapsedMs from the navigation. A start that is still 'running' after START_LIMIT_MS is loaded again (out.startStalls counts them: a stall of the engine's start-up
    // was seen in this runner, with the bundle that starts in about 8 s otherwise); the last start has the whole WAIT_LIMIT_MS
    let first = null;
    for (let attempt = 1; attempt <= START_TRIES; attempt++) {
      navAt = Date.now();
      const nav = await S('Page.navigate', { url: `http://127.0.0.1:${port}/` });
      if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
      first = await waitState(attempt < START_TRIES ? START_LIMIT_MS : WAIT_LIMIT_MS);
      if (first.at !== null) break;
      if (attempt < START_TRIES) out.startStalls++;
      log('start', { attempt, state: first.state, stalled: true });
    }
    out.state = first.state;
    out.elapsedMs = first.at !== null ? first.at - navAt : null;
    if (first.state !== 'engine-idle') throw new Error(`no engine-idle within ${WAIT_LIMIT_MS} ms (state ${first.state ?? 'window.__app not set'})`);

    // the room: #stage 1836 x 1056 CSS px (the window is changed by the difference, as p2-frames does)
    let room = null;
    for (let i = 0; i < 4 && room === null; i++) {
      const r = await ev(ROOM);
      if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) room = r;
      else {
        log('room', { sw: r.sw, sh: r.sh, iw: r.iw, ih: r.ih });
        await S('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: DPR, mobile: false });
        await sleep(1500);
        const again = await waitState(WAIT_LIMIT_MS);
        if (again.state !== 'engine-idle') throw new Error(`no engine-idle after the room fit (state ${again.state ?? 'window.__app not set'})`);
      }
    }
    if (room === null) throw new Error(`the room is not ${ROOM_W} x ${ROOM_H} CSS px`);
    out.dpr = room.dpr;
    log('room', { sw: room.sw, sh: room.sh, desktop: [room.x, room.y, room.w, room.h], dpr: room.dpr, guest: room.guest });

    // Issue #1376: the measurement of the four states of the front windows, in this order: welcome (the Welcome window, at the start), frontWindow (the "Problems loading the project" box, which the
    // project's opening brings: a window in front of the main form), then, after the cloud tab's press (below), fileMenu (the File menu open) and none (it closed again). A window that does not
    // overlap the Browser is moved over it by a drag from its top edge (method 'drag'); a state with no such window is 'no window' and not overlapped.
    // the window list is refused while the picture is being shown ("ask again"): asked again after a moment
    const windowsNow = async () => {
      for (let t = 0; ; t++) {
        try { return parseWindows(await ev(WINDOWS)); } catch (e) {
          if (t >= 20 || !/ask again/.test(String(e?.message ?? e))) throw e;
          await sleep(250);
        }
      }
    };
    const guestOf = async () => { const f = await ev(FACE); return { w: f.guestW, h: f.guestH }; };
    const waitFor = async (fn, ms) => { for (let t = 0; t <= ms; t += WINDOW_POLL_MS) { const v = await fn(); if (v) return v; await sleep(WINDOW_POLL_MS); } return null; };
    const measureState = async (name, bottom) => {
      const f = await ev(FACE);
      const k = f.desk.w / f.guestW;
      const quads = await ev(FRONT);
      const rects = [];
      for (let i = 0; i + 3 < quads.length; i += 4) rects.push({ x: f.desk.x - f.stage.x + quads[i] * k, y: f.desk.y - f.stage.y + quads[i + 1] * k, w: (quads[i + 2] - quads[i]) * k, h: (quads[i + 3] - quads[i + 1]) * k });
      const br = f.rect;
      const over = br ? rects.filter((r) => overlapArea(r, br) > 0) : [];
      const largest = over.length > 0 ? over.reduce((p, c) => (overlapArea(c, br) > overlapArea(p, br) ? c : p)) : null;
      const hits = rects.length > 0 ? await ev(HIT_POINTS(rects)) : { points: 0, hits: 0 };
      const st = { overlapped: over.length > 0, frontRect: largest ? round3(largest) : null, frontCount: rects.length, overlapPoints: hits.hits, overlapPointCount: hits.points, clip: f.clip };
      if (bottom && largest) st.bottomHit = await ev(BOTTOM_HIT(largest));
      return st;
    };
    const dragWindow = async (win, target) => {
      const f = await ev(FACE);
      const k = f.desk.w / f.guestW;
      const at = (gx, gy) => ({ x: f.desk.x + (gx + 0.5) * k, y: f.desk.y + (gy + 0.5) * k });
      const gx = win.x + Math.min(60, win.w / 2), gy = win.y + 6;
      const dx = target.x - win.x, dy = target.y - win.y;
      const p0 = at(gx, gy);
      await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p0.x, y: p0.y, button: 'none', buttons: 0, clickCount: 0, pointerType: 'mouse' });
      await sleep(60);
      await S('Input.dispatchMouseEvent', { type: 'mousePressed', x: p0.x, y: p0.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
      for (let i = 1; i <= DRAG_STEPS; i++) {
        const p = at(gx + (dx * i) / DRAG_STEPS, gy + (dy * i) / DRAG_STEPS);
        await S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p.x, y: p.y, button: 'left', buttons: 1, clickCount: 0, pointerType: 'mouse' });
        await sleep(40);
      }
      const pe = at(gx + dx, gy + dy);
      await S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: pe.x, y: pe.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      log('drag', { window: win.cls, from: [win.x, win.y], to: [target.x, target.y], grab: [gx, gy] });
    };
    const judgeState = async (name, pick, bottom = false) => {
      let st = await measureState(name, bottom);
      let method = 'as-is', win = null;
      if (pick && !st.overlapped) {
        const g = await guestOf();
        win = await waitFor(async () => pick(await windowsNow(), g), CANDIDATE_WAIT_MS);
        if (win) {
          await dragWindow(win, BROWSER_TARGET);
          await sleep(DRAG_SETTLE_MS);
          st = await measureState(name, bottom);
          method = 'drag';
        } else method = 'no window';
      }
      log('state', name, { method, window: win ? `${win.cls} | ${win.title}` : null, overlapped: st.overlapped, frontRect: st.frontRect, frontCount: st.frontCount, overlapPoints: st.overlapPoints, clip: st.clip, bottomHit: st.bottomHit ?? null });
      log('top-level', name, (await windowsNow()).map((w) => `${w.cls}|${w.title}|${w.x},${w.y} ${w.w}x${w.h}`));
      return { ...st, method, window: win ? { cls: win.cls, title: win.title } : null };
    };
    const titleBefore = await ev(TITLE);
    await sleep(STATE_SETTLE_MS);
    out.states.welcome = await judgeState('welcome', (ws) => ws.find((w) => w.cls === WELCOME_CLASS) ?? null);
    await pressKey(S, 'Escape', 27);
    for (let t = 0; t < OPEN_WAIT_MS; t += POLL_MS) {
      if ((await ev(TITLE)) !== titleBefore && (await ev(STATE))?.state === 'engine-idle') break;
      await sleep(POLL_MS);
    }
    await sleep(2000);
    await sleep(STATE_SETTLE_MS);
    out.states.frontWindow = await judgeState('frontWindow', (ws, g) => ws.find((w) => w.cls !== WELCOME_CLASS && w.cls !== MENU_CLASS && !(w.w >= MAIN_SHARE * g.w && w.h >= MAIN_SHARE * g.h)) ?? null);
    // the "Problems loading the project" box is closed (Escape, probe-navigate.mjs's second step): the cloud tab can be pressed
    await pressKey(S, 'Escape', 27);
    await sleep(DISMISS_MS);
    const face0 = await ev(FACE);
    if (!face0.guestW) throw new Error('the guest screen is not known to the page');
    const cssPerGuest0 = face0.desk.w / face0.guestW;
    const logFrom = (await ev(ENGINE_LOG)).length;
    if (process.argv[3]) fs.writeFileSync(`${process.argv[3]}-before.png`, Buffer.from((await S('Page.captureScreenshot', { format: 'png' })).data, 'base64'));
    // the cloud tab is selected at the start, so its press selects it again: the first tab (the waveform, guest 35,125) is pressed before it (probe-navigate.mjs's order)
    await pressCss(S, { x: face0.desk.x + (TAB_OTHER.x + 0.5) * cssPerGuest0, y: face0.desk.y + (TAB_OTHER.y + 0.5) * cssPerGuest0 });
    await sleep(TAB_SETTLE_MS);
    await pressCss(S, { x: face0.desk.x + (TAB_CLOUD.x + 0.5) * cssPerGuest0, y: face0.desk.y + (TAB_CLOUD.y + 0.5) * cssPerGuest0 });
    out.cloudPressed = true;
    for (let t = 0; t < FACE_WAIT_MS; t += POLL_MS) {
      const after = (await ev(ENGINE_LOG)).slice(logFrom);
      if (after.some((l) => /webview2: fire NavigationCompleted/.test(l))) break;
      await sleep(POLL_MS);
    }
    await sleep(TAB_SETTLE_MS);
    if (process.argv[3]) fs.writeFileSync(`${process.argv[3]}-after.png`, Buffer.from((await S('Page.captureScreenshot', { format: 'png' })).data, 'base64'));

    // Issue #1376: fileMenu (the File menu opened by a click at guest (18, 20), its window in the list), then none (the menu closed with Escape; nothing is in front of the Browser)
    await pressCss(S, { x: face0.desk.x + (MENU_AT.x + 0.5) * cssPerGuest0, y: face0.desk.y + (MENU_AT.y + 0.5) * cssPerGuest0 });
    await waitFor(async () => (await windowsNow()).some((w) => w.cls === MENU_CLASS), MENU_WAIT_MS);
    await sleep(STATE_SETTLE_MS);
    out.states.fileMenu = await judgeState('fileMenu', null, true);
    await pressKey(S, 'Escape', 27);
    await waitFor(async () => !(await windowsNow()).some((w) => w.cls === MENU_CLASS), MENU_WAIT_MS);
    await sleep(STATE_SETTLE_MS);
    out.states.none = await judgeState('none', null);
    delete out.states.none.overlapped;

    // the picture after the engine has been idle for a while (the lists are painted after the start-up)
    await sleep(SETTLE_MS);
    const fin = await ev(FINAL);
    out.standIn = fin.standIn;
    out.error = fin.error;
    out.state = fin.state;
    out.preBootBytes = fin.assets && Array.isArray(fin.assets.items) ? fin.assets.items.reduce((s, it) => s + it.bytes, 0) : null;
    log('windows', (fin.windows ?? []).map((w) => `${w.className}|${w.title}|${w.x},${w.y} ${w.width}x${w.height}${w.visible ? '' : ' hidden'}`));

    const wv = (await ev(ENGINE_LOG)).filter((l) => WEBVIEW2_LINE.test(l));
    const methods = wv.map((l) => WEBVIEW2_LINE.exec(l)[1]);
    out.webview2Lines = wv.length;
    out.webview2Methods = [...new Set(methods.filter((m) => m !== 'fire'))];
    out.webview2Fired = [...new Set(wv.filter((l) => WEBVIEW2_FIRE.test(l)).map((l) => WEBVIEW2_FIRE.exec(l)[1]))];
    out.navigateCalls = methods.filter((m) => NAVIGATE_METHODS.has(m)).length;
    for (const l of wv) process.stderr.write(l + '\n');
    for (const l of (await ev(ENGINE_LOG)).filter((x) => /^(\[err\] )?fle[-:]/.test(x) && !/fle-prot: TEMP commit/.test(x))) process.stderr.write(l + '\n');   // the engine's own lines (its stop message, the protection trace)

    // Issue #1362: the face (the iframe) against the last put_Bounds of the engine's log: its rectangle from the room's origin, and the same rectangle from the bounds
    const face = await ev(FACE);
    out.iframeCount = face.count;
    out.iframeRect = face.rect ? { x: face.rect.x, y: face.rect.y, w: face.rect.w, h: face.rect.h } : null;
    out.iframeSrcdocLength = face.srcdocLength;
    out.iframeBackground = face.frameBackground;
    const k = face.desk.w / face.guestW;
    const lastBounds = [...wv].reverse().map((l) => PUT_BOUNDS.exec(l)).find((m) => m !== null) ?? null;
    if (lastBounds) {
      const ox = lastBounds[5] !== undefined ? +lastBounds[5] : 0, oy = lastBounds[6] !== undefined ? +lastBounds[6] : 0;
      const [l, t, r, b] = lastBounds.slice(1, 5).map(Number);
      out.boundsRect = { x: face.desk.x - face.stage.x + (ox + l) * k, y: face.desk.y - face.stage.y + (oy + t) * k, w: (r - l) * k, h: (b - t) * k };
    }
    out.boundsMatch = out.iframeCount >= 1 && out.iframeRect !== null && out.boundsRect !== null && ['x', 'y', 'w', 'h'].every((key) => Math.abs(out.iframeRect[key] - out.boundsRect[key]) <= BOUNDS_MATCH_CSS);
    if (out.states.none) out.states.none.clipMatchesBounds = out.boundsMatch === true && out.states.none.clip === '';
    for (const st of Object.values(out.states)) delete st.clip;
    // the page's own picture of the Browser's area (CSS pixels of the room at DPR 1): what the person sees
    const shot = await S('Page.captureScreenshot', { format: 'png', clip: { x: face.desk.x, y: face.desk.y + BROWSER_GUEST.y * k, width: BROWSER_GUEST.w * k, height: BROWSER_GUEST.h * k, scale: 1 }, captureBeyondViewport: false });
    const shotBlack = await ev(countBlack(shot.data));
    out.blackShare = shotBlack.n > 0 ? shotBlack.black / shotBlack.n : null;
    const d = await ev(PICTURE);
    const pic = picture(d);
    const scale = pic.w / room.w;
    const found = findBrowser(fin.windows);
    let rect;
    if (found) {
      rect = { source: 'window-table', x: found.x, y: found.y, w: found.width, h: found.height, className: found.className, title: found.title };
    } else {
      rect = { source: 'fallback', x: FALLBACK_CSS.x * scale, y: FALLBACK_CSS.y * scale, w: FALLBACK_CSS.w * scale, h: FALLBACK_CSS.h * scale, className: '', title: '' };
    }
    const cr = clampRect(pic, rect);
    const j = judgePicture(pic, cr);
    out.guestBlackShare = j.blackShare;
    out.browserComponents = j.browserComponents;
    out.browserWindow = { ...rect, x: cr.x, y: cr.y, w: cr.w, h: cr.h, cssRect: { x: cr.x / scale, y: cr.y / scale, w: cr.w / scale, h: cr.h / scale }, background: j.background, pixels: j.pixels };
    log('picture', { w: pic.w, h: pic.h, scale, rect: cr, background: j.background, pixels: j.pixels });
    if (!found) log('no Browser window in the window table: the fallback rectangle is used');
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 500);
  } finally {
    if (navAt !== null && out.elapsedMs === null) out.elapsedMs = Date.now() - navAt;
    if (cdp) await shutdown(chrome, cdp).catch(() => {});
    else if (chrome) chrome.proc.kill('SIGKILL');
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  log('nproc', os.cpus().length, 'loadavg', os.loadavg().map((v) => v.toFixed(2)).join(' '), 'ms', Date.now() - t0);
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exitCode = out.error ? 1 : 0;
}

// the measurement runs when this file is the program; a tool that imports it gets judgePicture and findBrowser only
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
