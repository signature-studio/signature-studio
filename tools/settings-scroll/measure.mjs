// settings-scroll/measure (Issue #1384): the picture of the Settings window (F10, class TMIDIForm) on its General page, scrolled by the wheel over the page, at five positions of the scroll range
// (0, 25, 50, 75 and 100 %), as the picture is when the page is scrolled down from the top and when it is scrolled up from the bottom, and as it is when the whole window is drawn again.
// Boots the bundle in headless Chrome as the page is served (the room of 1836 x 1056 CSS px, as tools/smoke/welcome-flow.mjs does) at DPR 1 and at a random DPR (1.01 to 2.25), waits for the first engine-idle,
// closes the Welcome window and the dialog after it with Escape, opens the Settings window with F10, presses the General tab (its place from the window table's tab bar), and prints ONE JSON line per boot on stdout.
// The values (the acceptance reads them; the measure is the picture's pixels, the window table's positions):
//  - mismatchPath: the largest share of the window's pixels that differ between the picture reached from the top and the picture reached from the bottom, at the same scroll offset (the offset is read from the window table: a panel's y).
//  - mismatchRedraw: the largest share that differ between the picture as the scroll left it and the picture after the whole window was drawn again (window.__app.settingsRedraw: the measuring tool's request only).
//  - blankMax: the largest share of the window's area (16 x 16 blocks, all pixels black) that is not painted, over every picture taken during a scroll.
//  - positions: per position its target offset, the offset reached, the two shares above, and the blank share of its pictures.
//  - the scroll is measured on the scroll bar's thumb (its top is read from the picture: the thumb is the run of pixels that are not the track's colour): the thumb is dragged to each of the five tops from the top (down) and from the
//    bottom (up), so both pictures are at the same thumb top and offset. The wheel moves 96 px a notch and the offsets from the two ends are not the same (M = 1103 is not a multiple of 96), so the wheel route gives the blank share only.
//  - the pictures of the largest mismatch are written to pm-work/findings/settings-scroll-1384-*.png (half size: the down, up and redraw pictures).
// Usage: node tools/settings-scroll/measure.mjs <absolute path to the bundle directory raw/<name>> > scroll.jsonl   (needs app/node_modules and app/dist; the sign of a clean run is in the acceptance)
// Exit 0 when the tool ran (the two lines are on stdout), 1 when it failed, 2 when no bundle directory is given. The stderr is the log.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Cdp, freePort, launchChrome, shutdown, sleep } from '../boot-check/cdp.mjs';
import { stageAssets } from '../boot-check/stage-assets.mjs';
import { childWindows, topWindow, descendantsOf } from '../feat-c1-hidpi/measure-size.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const ROOM_W = 1836, ROOM_H = 1056;
const VIEW_START = { width: 2176, height: 1092 };
const ISSUE = 1384;
const SLOTS = 10;
const POSITIONS = [0, 0.25, 0.5, 0.75, 1];
const WHEEL_DELTA = 120;          // one notch
const NOTCH_PX = 96;              // measured (#1384): one notch moves the General page by 96 guest pixels
const SETTLE_MS = 350;
const READY_LIMIT_MS = 180000, CDP_TIMEOUT_MS = 30000, POLL_MS = 500;
const BLOCK = 16;
const BLANK_SUM = 24;             // a pixel is unpainted when its three colour bytes add up to less than this
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

const GEOMETRY = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN'), error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 600) : null }; })()`;
const READ_TIMES = `(() => { const a = window.__app; return { timeOrigin: performance.timeOrigin, idle: a.timeline.idle ?? null, bytes: (a.assetsReport()?.items ?? []).reduce((n, i) => n + i.bytes, 0) }; })()`;
const WINDOWS = `(async () => (await window.__app.deskDumpBase64(false)).windows)()`;
const TRACE_ON = `(function () { const t = setInterval(() => { if (window.__app) { clearInterval(t); window.__app.windowApiTrace(true); } }, 5); })();`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);
const timed = (p, ms, what) => {
  let t;
  return Promise.race([p, new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timeout ${ms} ms: ${what}`)), ms); })]).finally(() => clearTimeout(t));
};

/** one boot of the bundle at one DPR in a Chrome of its own: the page, the stage at the room, the calls of the measure */
class Boot {
  constructor(cdp, sessionId) {
    this.cdp = cdp;
    this.sessionId = sessionId;
  }
  send(method, params = {}) { return timed(this.cdp.send(method, params, this.sessionId), CDP_TIMEOUT_MS, method); }
  async ev(expression) {
    const r = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`.slice(0, 300));
    return r.result.value;
  }
  /** the window table (the picture's windows): the picture is being shown at times, and the page says ask again */
  async windows() {
    const t0 = Date.now();
    for (;;) {
      try { return String(await this.ev(WINDOWS)); } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 15000) throw e;
        await sleep(100);
      }
    }
  }
  async geometry() { return this.ev(GEOMETRY); }
  /** a wheel notch (deltaY) over the guest pixel (gx, gy), the pointer there */
  async wheel(gx, gy, deltaY) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
  }
  /** the pointer parked at a guest pixel that is on no window (the pictures are taken with the same pointer position) */
  async park(gx, gy) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(120);
  }
  /** a click (press and release) at a guest pixel */
  async click(gx, gy) {
    const g = await this.geometry();
    const c = { x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (gy + 0.5) * (g.h / g.gh) };
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
    await sleep(50);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(300);
  }
  /** the pointer pressed at guest (gx, gy), dragged by dy guest pixels in steps, released */
  async dragY(gx, gy, dy) {
    const g = await this.geometry();
    const at = (yy) => ({ x: g.x + (gx + 0.5) * (g.w / g.gw), y: g.y + (yy + 0.5) * (g.h / g.gh) });
    const p0 = at(gy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: p0.x, y: p0.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: p0.x, y: p0.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
    await sleep(40);
    for (let i = 1; i <= 12; i++) {
      const q = at(gy + Math.round((dy * i) / 12));
      await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: q.x, y: q.y, button: 'left', buttons: 1, pointerType: 'mouse' });
      await sleep(30);
    }
    const pe = at(gy + dy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: pe.x, y: pe.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(400);
  }
  async key(code, vk) {
    await this.ev("document.getElementById('desktop').focus({ preventScroll: true }); true");
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', code, key: code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await sleep(700);
  }
  /** the picture as RGBA: { w, h, data } (deskDumpBase64(true): the compositor's picture, as it is) */
  async picture() {
    for (;;) {
      try {
        const d = await this.ev('window.__app.deskDumpBase64(true)');
        const data = Buffer.from(d.base64, 'base64');
        if (data.length !== d.width * d.height * 4) throw new Error(`the picture is ${data.length} bytes for ${d.width}x${d.height}`);
        return { w: d.width, h: d.height, data, windows: d.windows };
      } catch (e) {
        if (!/ask again/.test(String(e?.message ?? e))) throw e;
        await sleep(100);
      }
    }
  }
}

/** the window table's top-level window and the General page: the page is the visible TVectorSheet of the Settings window; positions are guest pixels (top-level) and the parent's client (children) */
function settingsOf(windowsText) {
  const top = topWindow(windowsText, 'TMIDIForm');
  if (!top) return null;
  const kids = childWindows(windowsText);
  const page = kids.find((c) => c.parent === top.lid && c.cls === 'TVectorSheet' && c.vis) ?? null;
  const bar = kids.find((c) => c.parent === top.lid && c.cls === 'TQuickSheetSelector') ?? null;
  const panels = page ? descendantsOf(kids, page.lid).filter((c) => c.cls === 'TVectorPanel') : [];
  const scroller = page ? kids.find((c) => c.parent === page.lid && c.cls === 'TQuickScrollBoxScroller') ?? null : null;
  return { top, page, bar, panels, scroller, kids };
}

/** the content's reference: the panel with the least y at the top of the page (its offset is 0 there) */
function refPanel(panels) {
  let best = null;
  for (const p of panels) if (!best || p.y < best.y) best = p;
  return best;
}

/** the picture's share of pixels that differ (any of the colour bytes differs) between two crops of the same size */
function mismatch(a, b) {
  const x = a.data, y = b.data;
  if (x.length !== y.length || x.length === 0) return 1;
  let n = 0;
  const px = x.length / 4;
  for (let i = 0; i < px; i++) {
    const k = i * 4;
    if (x[k] !== y[k] || x[k + 1] !== y[k + 1] || x[k + 2] !== y[k + 2]) n++;
  }
  return n / px;
}

/** the crop of the picture inside the rectangle (guest pixels), as RGBA rows */
function crop(pic, r) {
  const x0 = Math.max(0, r.x), y0 = Math.max(0, r.y), x1 = Math.min(pic.w, r.x + r.w), y1 = Math.min(pic.h, r.y + r.h);
  const w = x1 - x0, h = y1 - y0;
  const out = Buffer.alloc(w * h * 4);
  for (let y = 0; y < h; y++) pic.data.copy(out, y * w * 4, ((y0 + y) * pic.w + x0) * 4, ((y0 + y) * pic.w + x1) * 4);
  return { w, h, data: out };
}

/** the scroll bar's thumb in the picture: the first run of the scroll bar column's pixels (x: the scroller's middle) whose colour is not the track's (the column's most common colour);
 * { T, len } with T the thumb's top relative to the page's top (guest pixels), or null */
function thumbOf(pic, s) {
  if (!s.scroller || !s.page) return null;
  const x = s.top.x + s.page.x + s.scroller.x + Math.floor(s.scroller.w / 2);
  const y0 = s.top.y + s.page.y, y1 = y0 + s.page.h;
  const colour = (y) => { const k = (y * pic.w + x) * 4; return (pic.data[k] << 16) | (pic.data[k + 1] << 8) | pic.data[k + 2]; };
  const counts = new Map();
  for (let y = y0; y < y1; y++) counts.set(colour(y), (counts.get(colour(y)) ?? 0) + 1);
  let track = -1, most = 0;
  for (const [c, n] of counts) if (n > most) { most = n; track = c; }
  let run = null;
  for (let y = y0; y <= y1; y++) {
    const inRun = y < y1 && colour(y) !== track;
    if (inRun && run === null) run = y;
    if (!inRun && run !== null) {
      if (y - run >= 8 && y - run <= 400) return { T: run - y0, len: y - run };
      run = null;
    }
  }
  return null;
}

/** the share of the crop's 16 x 16 blocks in which every pixel is unpainted (its colour bytes add up to less than BLANK_SUM) */
function blankShare(c) {
  let blocks = 0, blank = 0;
  for (let by = 0; by < c.h; by += BLOCK) {
    for (let bx = 0; bx < c.w; bx += BLOCK) {
      let all = true;
      for (let y = by; y < Math.min(by + BLOCK, c.h) && all; y++) {
        for (let x = bx; x < Math.min(bx + BLOCK, c.w); x++) {
          const k = (y * c.w + x) * 4;
          if (c.data[k] + c.data[k + 1] + c.data[k + 2] >= BLANK_SUM) { all = false; break; }
        }
      }
      blocks++;
      if (all) blank++;
    }
  }
  return blocks ? blank / blocks : 0;
}

/** a PNG (RGBA, 8 bit) of a crop, scaled down by `div` (nearest pixel) */
function pngOf(c, div = 2) {
  const w = Math.floor(c.w / div), h = Math.floor(c.h / div);
  const raw = Buffer.alloc(h * (w * 4 + 1));
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const s = ((y * div) * c.w + x * div) * 4;
      c.data.copy(raw, y * (w * 4 + 1) + 1 + x * 4, s, s + 4);
    }
  }
  const chunk = (type, body) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(body.length);
    const td = Buffer.concat([Buffer.from(type, 'latin1'), body]);
    const crc = Buffer.alloc(4); crc.writeUInt32BE(zlib.crc32(td) >>> 0);
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** the engine's log lines written after `marker` (the last line before the action); the log is a ring, so the marker is searched for */
function linesAfter(all, marker) {
  const i = marker === null ? -1 : all.lastIndexOf(marker);
  return all.slice(i + 1);
}

async function boot(dpr, port) {
  const root = fs.mkdtempSync(path.join(repo, '.tmp-settings-scroll-'));
  const chrome = await launchChrome({ profile: path.join(root, 'profile'), extraArgs: ISOLATION });
  const cdp = new Cdp(chrome.version.webSocketDebuggerUrl);
  await cdp.opened;
  const { browserContextId } = await cdp.send('Target.createBrowserContext', { disposeOnDetach: true });
  const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank', browserContextId });
  const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
  const b = new Boot(cdp, sessionId);
  await b.send('Page.enable');
  await b.send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: dpr, mobile: false });
  await b.send('Page.addScriptToEvaluateOnNewDocument', { source: TRACE_ON });
  const navSentAt = Date.now();
  const nav = await b.send('Page.navigate', { url: `http://127.0.0.1:${port}/` });
  if (nav.errorText) throw new Error(`navigate: ${nav.errorText}`);
  return { chrome, cdp, b, root, navSentAt };
}

async function closeBoot(h) {
  if (h.cdp) await shutdown(h.chrome, h.cdp).catch(() => {});
  else if (h.chrome) h.chrome.proc.kill('SIGKILL');
  fs.rmSync(h.root, { recursive: true, force: true });
}

/** one boot at one DPR: the first idle, the Settings window, the General page, the scroll routes, the comparisons */
async function runOne(dpr, port, pngDir) {
  const out = {
    dpr, room: { w: ROOM_W, h: ROOM_H }, state: null, standIn: null, error: null, elapsedMs: null, preBootBytes: null,
    mismatchPath: null, mismatchRedraw: null, blankMax: null, range: null, positions: [], routes: {}, trace: null, pngs: [],
  };
  let h = null;
  try {
    h = await boot(dpr, port);
    const b = h.b;
    for (;;) {
      await sleep(POLL_MS);
      const s = await b.ev(STATE).catch(() => null);
      if (s && (s.state === 'engine-idle' || s.state === 'error' || s.state === 'exited')) { out.state = s.state; break; }
      if (Date.now() - h.navSentAt >= READY_LIMIT_MS) throw new Error(`no engine-idle within ${READY_LIMIT_MS} ms`);
    }
    if (out.state !== 'engine-idle') throw new Error(`state ${out.state} before the first idle`);
    for (let i = 0; i < 4; i++) {
      const r = await b.ev(ROOM);
      if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) break;
      await b.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: dpr, mobile: false });
      await sleep(1500);
    }
    await sleep(1500);
    const st = await b.ev(STATE);
    out.state = st.state; out.standIn = st.standIn; out.error = st.error;
    const rt = await b.ev(READ_TIMES);
    if (rt.idle === null) throw new Error('engine-idle without the idle mark');
    out.elapsedMs = Math.round(rt.timeOrigin + rt.idle - h.navSentAt);
    out.preBootBytes = rt.bytes;
    log('idle', { dpr, elapsedMs: out.elapsedMs });

    for (let i = 0; i < 4; i++) {
      const w = await b.windows();
      if (!/TWelcomeWizard|TMissingPluginsForm/.test(w)) break;
      await b.key('Escape', 27);
      await sleep(2000);
    }
    await b.key('F10', 121);
    let set = null;
    for (let i = 0; i < 15 && !set; i++) {
      await sleep(1000);
      set = settingsOf(await b.windows());
    }
    if (!set) throw new Error('the Settings window (TMIDIForm) did not open within 15 s of F10');
    if (!set.bar) throw new Error('no tab bar (TQuickSheetSelector) in the Settings window');
    const pressSlot = async (s) => {
      const cur = settingsOf(await b.windows());
      const bar = cur.bar;
      await b.click(cur.top.x + bar.x + ((s + 0.5) * bar.w) / SLOTS, cur.top.y + bar.y + bar.h / 2);
    };
    let general = -1;
    for (let s = 0; s < SLOTS && general < 0; s++) {
      await pressSlot(s);
      const cur = settingsOf(await b.windows());
      if (cur.page && cur.page.text === 'General') general = s;
    }
    if (general < 0) throw new Error('the General page was not reached by the tab bar');
    log('General page at tab slot', general);

    let cur = settingsOf(await b.windows());
    const ref0 = refPanel(cur.panels);
    if (!ref0 || !cur.scroller) throw new Error('the General page has no panels or no scroll bar in the window table');
    const yTop = ref0.y;
    const offsetNow = (s) => { const r = s.panels.find((p) => p.lid === ref0.lid); return r ? yTop - r.y : null; };
    const pageRect = (s) => ({ x: s.top.x + s.page.x, y: s.top.y + s.page.y, w: s.page.w, h: s.page.h });
    const topRect = (s) => ({ x: s.top.x, y: s.top.y, w: s.top.w, h: s.top.h });
    const centre = (s) => ({ gx: s.top.x + s.page.x + Math.round(s.page.w / 2), gy: s.top.y + s.page.y + Math.round(s.page.h / 2) });
    const parkAt = (s) => ({ gx: s.top.x + s.top.w - 4, gy: s.top.y + s.top.h - 4 });
    const ctr = centre(cur);
    const park = parkAt(cur);
    const readOffset = async () => offsetNow(settingsOf(await b.windows()));
    const takePicture = async () => {
      const pic = await b.picture();
      const s = settingsOf(pic.windows);
      if (!s || !s.page) throw new Error('the Settings window or its page is not in the window table');
      const th = thumbOf(pic, s);
      return { off: offsetNow(s), T: th ? th.T : null, len: th ? th.len : null, img: crop(pic, topRect(s)), s, pic };
    };

    // the top: the wheel up until the offset stops
    await b.park(park.gx, park.gy);
    for (let i = 0; i < 40; i++) {
      await b.wheel(ctr.gx, ctr.gy, -WHEEL_DELTA);
      await sleep(SETTLE_MS);
    }
    await b.park(park.gx, park.gy);
    await sleep(600);
    const off0 = (await takePicture()).off;
    if (off0 !== 0) log('the top is not offset 0', off0);

    // route W (the wheel): notch by notch down to the bottom; its pictures are the blank share's and the offsets are its own (96 px a notch: not the same offsets as the other way)
    const wheelStates = [];
    let st0 = await takePicture();
    wheelStates.push({ off: st0.off, img: st0.img });
    let last = st0.off, stuck = 0;
    for (let i = 0; i < 40 && stuck < 2; i++) {
      await b.wheel(ctr.gx, ctr.gy, WHEEL_DELTA);
      await sleep(SETTLE_MS);
      await b.park(park.gx, park.gy);
      await sleep(300);
      const t = await takePicture();
      wheelStates.push({ off: t.off, img: t.img });
      if (t.off === last) stuck++; else stuck = 0;
      last = t.off;
    }
    out.range = { M: last, notchPx: NOTCH_PX, wheelNotches: wheelStates.length - 1, wheelOffsets: wheelStates.map((s) => s.off) };
    const now = settingsOf((await takePicture()).pic.windows);
    out.geometry = { top: [now.top.w, now.top.h], page: [now.page.w, now.page.h], panelBottom: Math.max(...now.panels.map((p) => p.y + p.h)), panelTop: Math.min(...now.panels.map((p) => p.y)) };
    log('wheel route', out.range, out.geometry);
    if (last <= 0) log(`the wheel does not move the General page at DPR ${dpr}: the scroll bar's thumb route decides the positions`);

    // route S (the scroll bar's thumb): the thumb is the picture's run of pixels that are not the track's colour, in the column of the page's scroll bar
    // its top is read from the picture; the drags move it by exact guest pixels until the thumb is at the wanted top (path-independent: the same top is the same offset)
    const sx = (s) => s.top.x + s.page.x + Math.round(s.scroller ? s.scroller.x + Math.floor(s.scroller.w / 2) : s.page.w - 9);
    const dragThumbTo = async (want) => {
      for (let k = 0; k < 10; k++) {
        const t = await takePicture();
        if (t.T === null) throw new Error('the thumb is not in the picture');
        if (t.T === want) return t;
        const mid = t.s.top.y + t.s.page.y + t.T + Math.floor(t.len / 2);
        const dy = want - t.T;
        const x = sx(t.s);
        await b.dragY(x, mid, dy);
        await sleep(300);
      }
      const t = await takePicture();
      return t.T === want ? t : null;
    };
    // grab: the thumb is pressed where it is and dragged by dy (a drag past the track's end stops at the end)
    const grab = async (dy) => {
      const t = await takePicture();
      if (t.T === null) throw new Error('the thumb is not found in the picture');
      await b.dragY(sx(t.s), t.s.top.y + t.s.page.y + t.T + Math.floor(t.len / 2), dy);
      return takePicture();
    };
    const topT0 = await takePicture();
    const bottomT = await grab(3000);
    const topT = await grab(-3000);
    if (topT.T === null || bottomT.T === null || topT0.T === null) throw new Error('the thumb is not found in the picture');
    const Tmin = topT.T, Tmax = bottomT.T;
    if (Tmax <= Tmin) throw new Error(`the scroll bar's thumb does not move at DPR ${dpr} (top ${Tmin}, bottom ${Tmax})`);
    out.thumb = { Tmin, Tmax, len: topT.len, offTop: topT.off, offBottom: bottomT.off };
    log('thumb', out.thumb);
    const targets = POSITIONS.map((p) => Math.round(Tmin + p * (Tmax - Tmin)));

    // route S down: the thumb from its top to each target (ascending)
    const downPics = [];
    for (let k = 0; k < POSITIONS.length; k++) {
      const t = await dragThumbTo(targets[k]);
      downPics.push(t ? { off: t.off, T: t.T, img: t.img } : null);
    }
    // route S up: the thumb from the bottom to each target (descending)
    const upPics = new Array(POSITIONS.length).fill(null);
    const upRedraw = new Array(POSITIONS.length).fill(null);
    // the redraw: the whole window is drawn again (window.__app.settingsRedraw: a measuring request) at each position; the picture after it is compared with the picture the scroll left
    const hasRedraw = (await b.ev("typeof window.__app.settingsRedraw === 'function'")) === true;
    await grab(3000);
    for (let k = POSITIONS.length - 1; k >= 0; k--) {
      const t = await dragThumbTo(targets[k]);
      upPics[k] = t ? { off: t.off, T: t.T, img: t.img, lid: t.s.top.lid } : null;
      if (!upPics[k] || !hasRedraw) continue;
      await b.ev(`window.__app.settingsRedraw(${upPics[k].lid})`);
      await sleep(1500);
      const r = await takePicture();
      upRedraw[k] = r.T === t.T && r.off === t.off ? { img: r.img } : null;
    }
    const blankAll = [...wheelStates.map((s) => s.img), ...downPics.filter(Boolean).map((q) => q.img), ...upPics.filter(Boolean).map((q) => q.img)].map((img) => blankShare(img));
    out.blankMax = Math.max(...blankAll);

    out.routes = { thumbTargets: targets, redrawHook: hasRedraw };
    out.positions = POSITIONS.map((p, k) => {
      const d = downPics[k], u = upPics[k];
      const aligned = !!d && !!u && d.off === u.off && d.T === u.T;
      const pathValue = aligned ? mismatch(d.img, u.img) : null;
      return { p, target: targets[k], downOff: d ? d.off : null, upOff: u ? u.off : null, aligned, mismatchPath: pathValue, mismatchRedraw: null, blank: { down: d ? blankShare(d.img) : null, up: u ? blankShare(u.img) : null } };
    });
    for (let k = 0; k < POSITIONS.length; k++) {
      if (upPics[k] && upRedraw[k]) out.positions[k].mismatchRedraw = mismatch(upPics[k].img, upRedraw[k].img);
    }
    const values = (key) => out.positions.map((q) => q[key]);
    out.mismatchPath = values('mismatchPath').every((v) => v !== null) ? Math.max(...values('mismatchPath')) : 1;
    out.mismatchRedraw = hasRedraw && values('mismatchRedraw').every((v) => v !== null) ? Math.max(...values('mismatchRedraw')) : null;
    if (!values('mismatchPath').every((v) => v !== null)) out.error = out.error ?? 'a position was not reached at the same thumb top from both ends';
    if (!hasRedraw) out.error = out.error ?? 'the redraw request is not in this build (window.__app.settingsRedraw)';
    // the largest mismatch's pictures, for the findings (half size)
    const worst = out.positions.map((q, k) => ({ q, d: downPics[k], u: upPics[k] })).filter((x) => x.q.mismatchPath !== null).sort((x, y) => y.q.mismatchPath - x.q.mismatchPath)[0];
    if (worst && worst.q.mismatchPath > 0.001) {
      const base = path.join(pngDir, `settings-scroll-${ISSUE}-dpr${String(dpr).replace('.', '_')}-p${String(Math.round(worst.q.p * 100))}`);
      fs.mkdirSync(pngDir, { recursive: true });
      fs.writeFileSync(`${base}-down.png`, pngOf(worst.d.img));
      fs.writeFileSync(`${base}-up.png`, pngOf(worst.u.img));
      const files = [`${base}-down.png`, `${base}-up.png`];
      const rd = upRedraw[worst.q.p === 1 ? POSITIONS.length - 1 : POSITIONS.indexOf(worst.q.p)];
      if (rd) { fs.writeFileSync(`${base}-redraw.png`, pngOf(rd.img)); files.push(`${base}-redraw.png`); }
      out.pngs = files.map((f) => path.relative(repo, f));
    }
    log('positions', out.positions.map((q) => ({ p: q.p, downOff: q.downOff, upOff: q.upOff, aligned: q.aligned, mismatchPath: q.mismatchPath, mismatchRedraw: q.mismatchRedraw })));
    log('blankMax', out.blankMax, 'mismatchPath', out.mismatchPath, 'mismatchRedraw', out.mismatchRedraw);
    return out;
  } catch (e) {
    out.error = String(e?.message ?? e).slice(0, 600);
    log('failed', out.error);
    return out;
  } finally {
    if (h) await closeBoot(h);
  }
}

function summarizeTrace(lines) {
  const counts = {};
  for (const l of lines) {
    const m = /^window-api: (\w+)/.exec(l);
    if (m) counts[m[1]] = (counts[m[1]] ?? 0) + 1;
  }
  return counts;
}

async function main() {
  const argv = process.argv.slice(2);
  const positional = argv.filter((a) => !a.startsWith('--'));
  if (positional.length !== 1) {
    process.stderr.write('usage: node tools/settings-scroll/measure.mjs <absolute path to the bundle directory raw/<name>>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  const BUNDLE = path.basename(bundleDir);
  const brDir = path.resolve(bundleDir, '..', '..', 'br', BUNDLE);
  const root = fs.mkdtempSync(path.join(repo, '.tmp-settings-scroll-assets-'));
  const assets = path.join(root, 'assets');
  let server = null;
  const rows = [];
  try {
    stageAssets({ rawDir: bundleDir, brDir, assets, bundle: BUNDLE });
    process.env.FL_ASSETS_DIR = assets;
    const vite = await import(pathToFileURL(createRequire(path.join(appDir, 'package.json')).resolve('vite')).href);
    const port = await freePort();
    server = await vite.preview({ root: appDir, logLevel: 'silent', preview: { host: '127.0.0.1', port, strictPort: true } });
    const randomDpr = crypto.randomInt(101, 226) / 100;
    log('dprs', [1, randomDpr]);
    const pngDir = path.join(repo, 'pm-work', 'findings');
    rows.push(await runOne(1, port, pngDir));
    rows.push(await runOne(randomDpr, port, pngDir));
  } finally {
    if (server) await server.close().catch(() => {});
    fs.rmSync(root, { recursive: true, force: true });
  }
  for (const r of rows) process.stdout.write(JSON.stringify(r) + '\n');
}

// the boots run when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
