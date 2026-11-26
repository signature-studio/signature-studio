// The page area's shared steps (Issue #1486): the canvas of the desktop, the device pixel ratio as a physical window (a move to another monitor, a browser zoom),
// the screenshot's pixels against the guest's picture, the beacon's request, the page's visibility and focus, and the visible elements of the page. The reads are the
// page's own test aids and the DOM; the actions are the browser's (CDP Emulation, the Playwright page) and real mouse and key events (lib/guest.mjs).
// Where headless Chromium has no such state (a tab hidden by another tab, a minimized window: both stay visible here), the page's own visibilityState and hasFocus are
// set for the test and the events are dispatched; the scenario's notes say so. Nothing here calls a product function to act.
import { sleep } from './flow.mjs';
import { picture, openMenu, closeNew, newWindows, windowTable } from './common.mjs';
import { regionHash } from './picture.mjs';

export const CANVAS = '#desktop canvas[data-desk="1"]';
export const SCREEN_LIMIT = { min: [640, 480], max: [7680, 4320] };
export const FACE = 'iframe[data-fl-webview2]';

/** the page's state: the device pixel ratio, the viewport (CSS px), the canvas (device px and CSS px), the room (CSS px), the guest's screen, the engine state, the presenter's mode and count */
export async function deskInfo(page) {
  return page.evaluate((sel) => {
    const c = document.querySelector(sel);
    const r = c ? c.getBoundingClientRect() : null;
    const stage = document.getElementById('stage').getBoundingClientRect();
    const a = window.__app;
    const snap = a ? a.perf.snapshot('periodic') : null;
    return {
      dpr: window.devicePixelRatio, inner: [window.innerWidth, window.innerHeight],
      canvas: c ? { w: c.width, h: c.height, left: r.left, top: r.top, cssW: r.width, cssH: r.height } : null,
      stage: { w: stage.width, h: stage.height, left: stage.left, top: stage.top },
      guest: a ? { w: a.screen.guest.width, h: a.screen.guest.height } : null,
      state: a ? a.state : null,
      mode: snap && snap.desk ? snap.desk.mode : null,
      presents: snap && snap.desk ? snap.desk.presents : null,
      info: document.getElementById('canvas-info')?.textContent ?? '',
    };
  }, CANVAS);
}

/** the expected guest screen for a room (CSS px) at a device pixel ratio: the room's device pixels, rounded, pulled inside the engine's limits (the rule, computed here from the numbers) */
export function expectedGuest(room, dpr) {
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  return { w: clamp(Math.round(room.w * dpr), SCREEN_LIMIT.min[0], SCREEN_LIMIT.max[0]), h: clamp(Math.round(room.h * dpr), SCREEN_LIMIT.min[1], SCREEN_LIMIT.max[1]) };
}

/**
 * the crispness of the canvas at its device pixel ratio: its CSS size times the ratio is its pixel size (within 0.5 px in each direction: no resampling), its pixels are the
 * guest's screen (the room in device pixels, within 1 px), the picture the engine keeps has the canvas's size, and the canvas starts on a device pixel.
 */
export function crispness(info, picW, picH) {
  const c = info.canvas;
  const dpr = info.dpr;
  const cssToDevW = c.cssW * dpr, cssToDevH = c.cssH * dpr;
  const exp = expectedGuest({ w: info.stage.w, h: info.stage.h }, dpr);
  const grid = Math.abs(c.left * dpr - Math.round(c.left * dpr)) < 0.01 && Math.abs(c.top * dpr - Math.round(c.top * dpr)) < 0.01;
  return {
    dpr,
    canvas: [c.w, c.h],
    cssToDevice: [Math.round(cssToDevW * 100) / 100, Math.round(cssToDevH * 100) / 100],
    expected: [exp.w, exp.h],
    picture: [picW, picH],
    sameAsPicture: picW === c.w && picH === c.h,
    matchesRoom: Math.abs(c.w - exp.w) <= 1 && Math.abs(c.h - exp.h) <= 1,
    noResample: Math.abs(cssToDevW - c.w) <= 0.5 && Math.abs(cssToDevH - c.h) <= 0.5,
    onDeviceGrid: grid,
  };
}

/** the WebView2 faces (the Browser window's content, a real iframe over the canvas): their rectangles in CSS px. They are not the canvas's pixels, so the screen comparison leaves them out */
export async function faceRects(page) {
  return page.evaluate((sel) => [...document.querySelectorAll(sel)].map((f) => { const r = f.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; }), FACE);
}

/** the screenshot's pixels at the points (device px of the screenshot): [[r, g, b], ...]; the screenshot's size too. The PNG is decoded in the page (createImageBitmap) */
export async function screenshotAt(cdp, page, points) {
  const s = await cdp.send('Page.captureScreenshot', { format: 'png' });
  return page.evaluate(async ({ b64, pts }) => {
    const bytes = Uint8Array.from(atob(b64), (ch) => ch.charCodeAt(0));
    const bmp = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const cv = new OffscreenCanvas(bmp.width, bmp.height);
    const ctx = cv.getContext('2d');
    ctx.drawImage(bmp, 0, 0);
    const d = ctx.getImageData(0, 0, bmp.width, bmp.height).data;
    return { w: bmp.width, h: bmp.height, px: pts.map(([x, y]) => { const i = (y * bmp.width + x) * 4; return [d[i], d[i + 1], d[i + 2]]; }) };
  }, { b64: s.data, pts: points });
}

/**
 * the guest's picture against the screenshot of the same device pixels: the canvas's pixel (x, y) is the screenshot's (x + ox, y + oy). Samples a grid of the picture
 * (outside the WebView2 faces). Returns { samples, mismatches, faces, shotSize, pictureSize }. A canvas that is resampled differs on its edges; the same pixels differ on none.
 */
export async function pictureMatchesScreen(guest, cdp, page, info, step = 29) {
  const pic = await picture(guest);
  const dpr = info.dpr;
  const ox = Math.round(info.canvas.left * dpr), oy = Math.round(info.canvas.top * dpr);
  const faces = await faceRects(page);
  const inFace = (sx, sy) => faces.some((f) => sx / dpr >= f.x && sx / dpr < f.x + f.w && sy / dpr >= f.y && sy / dpr < f.y + f.h);
  const pts = [];
  for (let y = 0; y < pic.h; y += step) for (let x = 0; x < pic.w; x += step) if (!inFace(x + ox, y + oy)) pts.push([x, y]);
  const shot = await screenshotAt(cdp, page, pts.map(([x, y]) => [x + ox, y + oy]));
  let mismatches = 0;
  pts.forEach(([x, y], k) => {
    const i = y * pic.stride + 4 * x;
    const s = shot.px[k];
    if (s[0] !== pic.bytes[i] || s[1] !== pic.bytes[i + 1] || s[2] !== pic.bytes[i + 2]) mismatches++;
  });
  return { samples: pts.length, mismatches, faces: faces.length, shotSize: [shot.w, shot.h], pictureSize: [pic.w, pic.h] };
}

/** the ratio changes and the physical window stays: the CSS viewport is the physical size over the new ratio (whole px). A move to another monitor, or a browser zoom (which changes the ratio and the CSS viewport the same way) */
export async function setRatio(cdp, page, dpr) {
  const d = await deskInfo(page);
  const pw = Math.round(d.inner[0] * d.dpr), ph = Math.round(d.inner[1] * d.dpr);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: Math.round(pw / dpr), height: Math.round(ph / dpr), deviceScaleFactor: dpr, mobile: false });
}

/** the ratio and the viewport the page has now, to put back when the scenario ends */
export async function baseOf(page) {
  const d = await deskInfo(page);
  return { dpr: d.dpr, w: d.inner[0], h: d.inner[1] };
}
export async function restoreBase(cdp, base) {
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: base.w, height: base.h, deviceScaleFactor: base.dpr, mobile: false });
}

/**
 * waits until the page shows the ratio `dpr`: the canvas's CSS size is its pixels over the ratio (within 1 CSS px), the guest's screen is the canvas, and the canvas is the room's
 * device pixels. Returns { ok, waitedMs, d } (ok false at the limit).
 */
export async function waitSettled(page, dpr, ms = 20000) {
  const t0 = Date.now();
  let d = null;
  for (;;) {
    d = await deskInfo(page);
    const c = d.canvas;
    if (c && Math.abs(d.dpr - dpr) < 1e-6 && Math.abs(c.cssW - c.w / d.dpr) < 1 && Math.abs(c.cssH - c.h / d.dpr) < 1 && d.guest && d.guest.w === c.w && d.guest.h === c.h) {
      const exp = expectedGuest({ w: d.stage.w, h: d.stage.h }, d.dpr);
      if (Math.abs(c.w - exp.w) <= 1 && Math.abs(c.h - exp.h) <= 1) return { ok: true, waitedMs: Date.now() - t0, d };
    }
    if (Date.now() - t0 >= ms) return { ok: false, waitedMs: Date.now() - t0, d };
    await sleep(250);
  }
}

/** the page's visibility, as the page reads it (document.visibilityState). A test's own shim: the headless browser does not hide a tab; the change is announced by visibilitychange */
export async function setHidden(page, hidden) {
  return page.evaluate((h) => {
    if (h) Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' });
    else delete document.visibilityState;
    document.dispatchEvent(new Event('visibilitychange'));
    return document.visibilityState;
  }, hidden);
}

/** the window's focus, as the page reads it (document.hasFocus) and announced by the window's blur / focus events. A test's own shim: headless Chromium keeps the focus on another tab */
export async function setFocused(page, focused) {
  return page.evaluate((f) => {
    if (f) { delete document.hasFocus; window.dispatchEvent(new Event('focus')); }
    else { document.hasFocus = () => false; window.dispatchEvent(new Event('blur')); }
    return document.hasFocus();
  }, focused);
}

/** the beacon as it is sent now (window.__app.perf.sendNow): the request's path, body and headers, and the server's answer */
export async function beaconNow(page) {
  const req = page.waitForRequest((r) => r.url().includes('/__perf') && r.method() === 'POST', { timeout: 20000 });
  const sent = page.evaluate(() => window.__app.perf.sendNow('periodic'));
  const [r, result] = await Promise.all([req, sent]);
  return { url: r.url(), body: r.postData() ?? '', headers: await r.allHeaders(), result };
}

/** the SHA-256 of the whole picture (the guest's pixels as the engine keeps them): a change of the picture changes it */
export async function pictureHash(guest) {
  const p = await picture(guest);
  return regionHash(p, { x: 0, y: 0, w: p.w, h: p.h });
}

/** the canvas as it is now against the picture: { ok, c } where ok is the crispness (no resampling, the room's device pixels, the picture's size, on the device grid) */
export async function crispNow(guest, page) {
  const d = await deskInfo(page);
  const pic = await picture(guest);
  const c = crispness(d, pic.w, pic.h);
  return { ok: c.noResample && c.matchesRoom && c.sameAsPicture && c.onDeviceGrid, c, d };
}

/** the black 64 x 64 tiles (all pixels black) whose centre lies in a top-level window of the window table, and the picture's tiles: { black, total } */
export async function blackInWindows(guest) {
  const p = await picture(guest);
  const t = await windowTable(guest);
  const wins = t.top.filter((w) => w.w > 0 && w.h > 0);
  let n = 0, total = 0;
  for (let ty = 0; ty < p.h; ty += 64) {
    for (let tx = 0; tx < p.w; tx += 64) {
      const x1 = Math.min(p.w, tx + 64), y1 = Math.min(p.h, ty + 64);
      total++;
      const cx = (tx + x1) / 2, cy = (ty + y1) / 2;
      if (!wins.some((w) => cx >= w.x && cx < w.x + w.w && cy >= w.y && cy < w.y + w.h)) continue;
      let black = true;
      scan: for (let y = ty; y < y1; y++) for (let x = tx; x < x1; x++) { const i = y * p.stride + 4 * x; if (p.bytes[i] | p.bytes[i + 1] | p.bytes[i + 2]) { black = false; break scan; } }
      if (black) n++;
    }
  }
  return { black: n, total };
}

/**
 * a browser zoom of z (relative to the page's ratio at the start): the ratio is that times z, the physical window stays, and the page must show it crisp and keep drawing.
 * Returns { ok, notes, results }; the page's ratio and viewport are put back at the end.
 */
export async function zoomCase({ guest, page, cdp }, z) {
  const base = await baseOf(page);
  const target = Math.round(base.dpr * z * 10000) / 10000;
  try {
    await setRatio(cdp, page, target);
    const s = await waitSettled(page, target, 20000);
    const r = await crispNow(guest, page);
    const f = await menuChangesPicture(guest, () => pictureHash(guest));
    const ok = s.ok && r.ok && f.pops > 0 && f.shown !== f.after;
    return { ok, engineState: r.d.state, notes: [`zoom ${Math.round(z * 100)} %: ratio ${target} (from ${base.dpr}), settled ${s.ok} in ${s.waitedMs} ms; canvas ${r.c.canvas.join('x')} device px, CSS ${r.c.cssToDevice.join('x')} device px at the ratio, the room ${r.c.expected.join('x')}`, `crisp ${r.ok}; the File menu drew ${f.shown !== f.after ? 'a change' : 'no change'} (drop-downs ${f.pops})`], results: { zoom: z, ratio: target, canvas: r.c.canvas, expected: r.c.expected, crisp: r.ok, drawn: f.shown !== f.after } };
  } finally {
    await restoreBase(cdp, base);
    await waitSettled(page, base.dpr, 20000).catch(() => null);
  }
}

/** a change that the picture shows: the File menu opens (a drop-down), then it closes; returns the picture's hash with the drop-down, and after it */
export async function menuChangesPicture(guest, pictureHash) {
  const base = new Set((await guest.windows()).map((w) => w.lid));
  await openMenu(guest, 'File');
  await sleep(600);
  const { pops } = await newWindows(guest, base);
  const shown = await pictureHash();
  await closeNew(guest, base);
  await sleep(400);
  return { pops: pops.length, shown, after: await pictureHash() };
}

/** the window table's main form (the engine's main window): its rectangle in guest px, or null */
export async function mainForm(guest) {
  const t = await windowTable(guest);
  return t.top.find((w) => w.cls === 'TFruityLoopsMainForm') ?? null;
}

/** the page's visible elements: other than the canvas and the containers that hold it (they count only when they paint), and other than the WebView2 faces (counted apart);
 *  the standing elements of the page are the allowed ones (#status, #audio-status, #taskbar) */
export const VISIBLE_CHECK = `(() => {
  const ALLOWED = ['status', 'audio-status', 'taskbar'];
  const shown = (el) => {
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  const paints = (el) => {
    const cs = getComputedStyle(el);
    const bg = cs.backgroundColor !== 'rgba(0, 0, 0, 0)' && cs.backgroundColor !== 'transparent';
    const border = ['Top', 'Right', 'Bottom', 'Left'].some((s) => parseFloat(cs['border' + s + 'Width']) > 0 && cs['border' + s + 'Style'] !== 'none');
    const text = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim() !== '');
    return bg || border || text;
  };
  const canvas = document.querySelector('#desktop canvas[data-desk="1"]');
  const holders = new Set();
  for (let e = canvas ? canvas.parentElement : null; e && e !== document.body; e = e.parentElement) holders.add(e);
  const faceParts = new Set();
  for (const f of document.querySelectorAll('iframe[data-fl-webview2]')) { faceParts.add(f); if (f.parentElement) faceParts.add(f.parentElement); }
  const others = [];
  const allowed = [];
  let faces = 0;
  for (const el of document.body.querySelectorAll('*')) {
    if (el === canvas || el.tagName === 'SCRIPT' || el.tagName === 'STYLE') continue;
    if (!shown(el)) continue;
    if (faceParts.has(el)) { if (el.tagName === 'IFRAME') faces++; continue; }
    if (holders.has(el) && !paints(el)) continue;
    const id = el.id || '';
    if (ALLOWED.includes(id)) { allowed.push('#' + id); continue; }
    others.push(id ? '#' + id : el.tagName.toLowerCase());
  }
  return { others, allowed, faces, dataImage: document.body.textContent.includes('data:image') };
})()`;

/**
 * the page moves to a monitor of another ratio (the physical window stays): the canvas is crisp at the new ratio, the picture keeps drawing (the File menu changes it),
 * and the page is put back to its ratio and viewport at the end.
 */
export async function dprMoveCase({ guest, page, cdp }, target) {
  const base = await baseOf(page);
  try {
    await setRatio(cdp, page, target);
    const s = await waitSettled(page, target, 20000);
    const r = await crispNow(guest, page);
    const f = await menuChangesPicture(guest, () => pictureHash(guest));
    const drawn = f.pops > 0 && f.shown !== f.after;
    return { ok: s.ok && r.ok && drawn, engineState: r.d.state, notes: [`ratio ${base.dpr} -> ${target}: settled ${s.ok} in ${s.waitedMs} ms; canvas ${r.c.canvas.join('x')} device px, CSS x ratio ${r.c.cssToDevice.join('x')}, the room ${r.c.expected.join('x')}`, `crisp ${r.ok}; the File menu drew ${drawn}`], results: { from: base.dpr, to: target, crisp: r.ok, drawn, canvas: r.c.canvas } };
  } finally {
    await restoreBase(cdp, base);
    await waitSettled(page, base.dpr, 20000).catch(() => null);
  }
}
