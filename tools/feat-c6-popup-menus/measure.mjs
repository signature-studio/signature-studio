// feat-c6-popup-menus: the measurement of C-6 (#1310). For the top bar's parts (tempo, song-position, pattern-selector) and the mixer's Insert 1 name: what the engine logs for a right click (input:, window-api:, menu-api:) and whether the picture shows a menu-sized band after it.
// It records and measures only: no menu is drawn by this tool and no item is chosen. A menu that appears is closed with Escape before the next part.
// Usage: node tools/feat-c6-popup-menus/measure.mjs <absolute path to the bundle directory raw/<name>> > feat-c6.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does). Exit 1 when error is set, 2 when no bundle directory is given.
// Order, on the engine's first idle (bootBundle's afterIdle):
//   1. window.__app.windowApiTrace(true): the window-API lines (window-api:) are written from here on (off at the start of every start-up, so the start-up itself writes none);
//   2. the usage-data dialog ("Help us improve FL Studio by sending usage data") is closed with "No thanks" at guest (513, 308), then 2.5 s. The window list does not hold this dialog (it lists only the main form and the welcome
//      wizard), so the click is made without a check. It is this tool only: the product's start-up is not changed. While the dialog is up it takes every click (the Default row does nothing);
//   3. the Default row of the start page, guest (124, 81), is double-clicked; the default project is open when the title has changed and the engine is idle again (waited up to 120 s, then 2 s more);
//   4. per part, in this order: tempo (462, 20), song-position (566, 20), pattern-selector (616, 58), then F9 and mixer-insert1 (506, 174). The mixer is part of the main form, not a
//      top-level window, so F9 is judged by the picture: it must change MIXER_OPEN_MIN_PX pixels, else the note is 未確認 and menuBandChanged false;
//        a. the picture (c0), 1 s with no input, the picture again (c1): the pixels that change with no input (ctrl);
//        b. the right button: moved, pressed, 1 s, released, 1 s; the picture (after). The engine's log lines of that span are inputLines, windowApiCalls and menuApiCalls;
//        c. Escape (a menu, if one is open, is closed), then 1 s.
// A guest pixel (x, y) is mapped to the CSS point of #desktop with the picture's size. The picture is window.__app.deskDumpBase64(true): the compositor's picture as it is (no compose). Its windows
// (lid N at (x, y) WxH ... class "..." title "...") say where the windows are.
// Thresholds (fixed here, and reported with the result):
//   NEAR_PX = 200: controlNearClick and changedNearClick count the pixels of the square of 200 x 200 guest px centred on the click.
//   controlNearClick: the pixels of that square that change with no input (ctrl).
//   changedNearClick: the pixels of that square that change for the click and did not change with no input.
//   menuBandChanged: the pixels that change for the click and not with no input, over the whole picture, grouped into 4-connected regions; true when one region has a bounding box of at least 120 px wide and 40 px high, and fills at least 0.8 of it.
//   A point that no listed window holds gets the note "未確認" and menuBandChanged false (the clicks are still made and the counts still written).
// stdout: ONE JSON line {state, standIn, error, nproc, loadavg, parts: [{name, x, y, inputLines, windowApiCalls, menuApiCalls, controlNearClick, changedNearClick, menuBandChanged, note}], logAdded}.
//   logAdded: the input:, menu-api: and window-api: lines added after the default project opened. stderr: one line per part, the windows at each step, and the number of window-api: lines the measurement wrote.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';

const NEAR_PX = 200;
const BAND_MIN_W = 120, BAND_MIN_H = 40, BAND_MIN_FILL = 0.8;
const CONSENT_NO = { x: 513, y: 308 };
const DEFAULT_ROW = { x: 124, y: 81 };
const TOP_PARTS = [
  { name: 'tempo', x: 462, y: 20 },
  { name: 'song-position', x: 566, y: 20 },
  { name: 'pattern-selector', x: 616, y: 58 },
];
const MIXER_PART = { name: 'mixer-insert1', x: 506, y: 174 };
const MIXER_OPEN_MIN_PX = 1000;   // the pixels that F9 has to change for the mixer to count as open
const WAIT_MS = 1000;
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN') }; })()`;
const PICTURE = `window.__app.deskDumpBase64(true)`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`;
const LOG_LEN = `window.__app.engineLog().length`;
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;

/** the windows of a window list: { lid, x, y, w, h, cls, title } (the page's own format: lid N at (x, y) WxH ... class "..." title "...") */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const cls = /class "([^"]*)"/.exec(m[6]), title = /title "([^"]*)"\s*$/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: cls ? cls[1] : '', title: title ? title[1] : '' };
  });
}
const holds = (win, p) => p.x >= win.x && p.y >= win.y && p.x < win.x + win.w && p.y < win.y + win.h;

/** a picture: the RGBA bytes of a dump (its rows may be longer than width x 4) and the windows of the same dump */
function picture(d) {
  const bytes = Buffer.from(d.base64, 'base64');
  const stride = bytes.length / d.height;
  if (!Number.isInteger(stride) || stride < d.width * 4) throw new Error(`the picture has ${bytes.length} bytes for ${d.width}x${d.height}`);
  return { w: d.width, h: d.height, bytes, stride, windows: parseWindows(d.windows) };
}

/** the pixels that differ (in R, G, B) between two pictures of the same size: one byte per pixel */
function changedMask(a, b) {
  if (a.w !== b.w || a.h !== b.h) throw new Error(`the picture changed size: ${a.w}x${a.h} to ${b.w}x${b.h}`);
  const m = new Uint8Array(a.w * a.h);
  for (let y = 0; y < a.h; y++) {
    const ra = y * a.stride, rb = y * b.stride;
    for (let x = 0; x < a.w; x++) {
      const i = ra + 4 * x, j = rb + 4 * x;
      if (a.bytes[i] !== b.bytes[j] || a.bytes[i + 1] !== b.bytes[j + 1] || a.bytes[i + 2] !== b.bytes[j + 2]) m[y * a.w + x] = 1;
    }
  }
  return m;
}

const countAll = (mask) => { let n = 0; for (const v of mask) n += v; return n; };

/** the pixels of mask (and not of exclude) inside the square of NEAR_PX around the point p (guest px) */
function nearCount(mask, pic, p, exclude = null) {
  const x0 = Math.max(0, p.x - NEAR_PX / 2), x1 = Math.min(pic.w - 1, p.x + NEAR_PX / 2 - 1);
  const y0 = Math.max(0, p.y - NEAR_PX / 2), y1 = Math.min(pic.h - 1, p.y + NEAR_PX / 2 - 1);
  let n = 0;
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
    const i = y * pic.w + x;
    if (mask[i] && !(exclude && exclude[i])) n++;
  }
  return n;
}

/** true when a 4-connected region of mask has a bounding box of at least BAND_MIN_W x BAND_MIN_H and fills at least BAND_MIN_FILL of it */
function bandChanged(mask, w, h) {
  const seen = new Uint8Array(w * h), stack = new Int32Array(w * h);
  for (let s = 0; s < w * h; s++) {
    if (!mask[s] || seen[s]) continue;
    let sp = 0, n = 0, x0 = w, y0 = h, x1 = -1, y1 = -1;
    stack[sp++] = s;
    seen[s] = 1;
    while (sp > 0) {
      const q = stack[--sp], x = q % w, y = (q - x) / w;
      n++;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
      for (const k of [x > 0 ? q - 1 : -1, x < w - 1 ? q + 1 : -1, y > 0 ? q - w : -1, y < h - 1 ? q + w : -1]) {
        if (k >= 0 && mask[k] && !seen[k]) { seen[k] = 1; stack[sp++] = k; }
      }
    }
    const bw = x1 - x0 + 1, bh = y1 - y0 + 1;
    if (bw >= BAND_MIN_W && bh >= BAND_MIN_H && n / (bw * bh) >= BAND_MIN_FILL) return true;
  }
  return false;
}

const strip = (l) => l.replace(/^\[err\] /, '');
const kindOf = (l) => /^(input|menu-api|window-api): /.exec(strip(l))?.[1] ?? null;

async function waitFor(pred, ms, what) {
  const t0 = Date.now();
  for (;;) {
    const v = await pred();
    if (v) return v;
    if (Date.now() - t0 >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
  }
}

/** the CSS point of a guest pixel: its centre, on the box of #desktop (the page maps a CSS point back to the guest pixel floor(x * guest / css)) */
const cssAt = (r, pic, p) => ({ x: r.x + (p.x + 0.5) * (r.w / pic.w), y: r.y + (p.y + 0.5) * (r.h / pic.h) });

async function mouse(page, type, c, button, buttons, clickCount) {
  await page.send('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button, buttons, clickCount, pointerType: 'mouse' });
}
async function key(page, name, vk) {
  await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  await sleep(40);
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
}

const picAt = async (page) => picture(await page.evaluate(PICTURE));
const rectAt = (page) => page.evaluate(RECT);
const logLength = (page) => page.evaluate(LOG_LEN);
const logSince = (page, n) => page.evaluate(LOG_FROM(n));

function sayWindows(label, wins) {
  for (const w of wins) process.stderr.write(`windows at ${label}: lid ${w.lid} at (${w.x}, ${w.y}) ${w.w}x${w.h} class "${w.cls}" title "${w.title}"\n`);
}

/** clicks No thanks of the usage-data dialog (shown at the first idle; no window of the list holds it) */
async function closeConsent(page, pic) {
  const c = cssAt(await rectAt(page), pic, CONSENT_NO);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'left', 1, 1);
  await sleep(50);
  await mouse(page, 'mouseReleased', c, 'left', 0, 1);
  await sleep(2500);
  process.stderr.write(`No thanks clicked at guest (${CONSENT_NO.x}, ${CONSENT_NO.y})\n`);
}

/** double-clicks the Default row; then waits for the default project: the title has changed and the engine is idle */
async function openDefault(page, pic) {
  const before = await page.evaluate(STATE);
  const c = cssAt(await rectAt(page), pic, DEFAULT_ROW);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'left', 1, 1);
  await sleep(50);
  await mouse(page, 'mouseReleased', c, 'left', 0, 1);
  await sleep(100);
  await mouse(page, 'mousePressed', c, 'left', 1, 2);
  await sleep(50);
  await mouse(page, 'mouseReleased', c, 'left', 0, 2);
  await waitFor(async () => {
    const s = await page.evaluate(STATE);
    if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} while the default project opens`);
    return s && s.state === 'engine-idle' && s.title !== before.title;
  }, OPEN_LIMIT_MS, 'the default project (a new title, then engine-idle)');
  await sleep(2000);
  const after = await page.evaluate(STATE);
  if (after.state !== 'engine-idle') throw new Error(`state ${after.state} 2 s after the default project opened`);
  process.stderr.write(`default project open: title "${before.title}" -> "${after.title}"\n`);
}

async function measurePart(page, p) {
  const out = { name: p.name, x: p.x, y: p.y, inputLines: [], windowApiCalls: [], menuApiCalls: [], controlNearClick: 0, changedNearClick: 0, menuBandChanged: false, note: null };
  const c0 = await picAt(page);
  await sleep(WAIT_MS);
  const c1 = await picAt(page);
  const ctrl = changedMask(c0, c1);
  out.controlNearClick = nearCount(ctrl, c1, p);
  const inWindow = c1.windows.some((w) => holds(w, p));
  if (!inWindow) out.note = `未確認: no listed window holds guest (${p.x}, ${p.y})`;
  const c = cssAt(await rectAt(page), c1, p);
  const from = await logLength(page);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'right', 2, 1);
  await sleep(WAIT_MS);
  await mouse(page, 'mouseReleased', c, 'right', 0, 1);
  await sleep(WAIT_MS);
  const after = await picAt(page);
  const span = (await logSince(page, from)).map(strip);
  out.inputLines = span.filter((l) => kindOf(l) === 'input');
  out.windowApiCalls = span.filter((l) => kindOf(l) === 'window-api');
  out.menuApiCalls = span.filter((l) => kindOf(l) === 'menu-api');
  const moved = changedMask(c1, after);
  const residual = new Uint8Array(moved.length);
  for (let i = 0; i < moved.length; i++) residual[i] = moved[i] && !ctrl[i] ? 1 : 0;
  out.changedNearClick = nearCount(residual, c1, p);
  out.menuBandChanged = inWindow && bandChanged(residual, c1.w, c1.h);
  process.stderr.write(`${p.name} (${p.x}, ${p.y}): control ${out.controlNearClick} changed ${out.changedNearClick} band ${out.menuBandChanged} input ${out.inputLines.length} window-api ${out.windowApiCalls.length} menu-api ${out.menuApiCalls.length}${out.note ? ` ${out.note}` : ''}\n`);
  return out;
}

/** Escape closes what is open (a menu, or a dialog), then 1 s */
async function escape(page) {
  await key(page, 'Escape', 27);
  await sleep(WAIT_MS);
}

async function session(page, result) {
  const traceFrom = await logLength(page);
  await page.evaluate('window.__app.windowApiTrace(true)');
  let pic = await picAt(page);
  sayWindows('the start page', pic.windows);
  await closeConsent(page, pic);
  pic = await picAt(page);
  await openDefault(page, pic);
  sayWindows('the default project', (await picAt(page)).windows);
  const openedAt = await logLength(page);
  for (const p of TOP_PARTS) {
    result.parts.push(await measurePart(page, p));
    await escape(page);
  }
  // the mixer is part of the main form (the window list holds only top-level windows): F9 is seen as the picture that changes when it opens
  const beforeF9 = await picAt(page);
  await key(page, 'F9', 120);
  await sleep(2500);
  const f9 = countAll(changedMask(beforeF9, await picAt(page)));
  process.stderr.write(`F9 changed ${f9} pixels of the picture\n`);
  const part = await measurePart(page, MIXER_PART);
  if (f9 < MIXER_OPEN_MIN_PX) {
    part.note = `未確認: F9 changed ${f9} pixels (fewer than ${MIXER_OPEN_MIN_PX}): the mixer did not show (guest (${MIXER_PART.x}, ${MIXER_PART.y}))`;
    part.menuBandChanged = false;
  }
  result.parts.push(part);
  result.logAdded = (await logSince(page, openedAt)).filter((l) => kindOf(l) !== null).map(strip);
  process.stderr.write(`window-api lines written during the measurement: ${(await logSince(page, traceFrom)).filter((l) => kindOf(l) === 'window-api').length}\n`);
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/feat-c6-popup-menus/measure.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  const result = { parts: [], logAdded: [] };
  let ran = false;
  const { out } = await bootBundle(bundleDir, {
    afterIdle: async (page) => {
      ran = true;
      await session(page, result);
    },
  });
  if (!ran && !out.error) out.error = `the session did not run: state ${out.state}`;
  const final = { state: out.state, standIn: out.standIn, error: out.error, nproc: os.availableParallelism(), loadavg: os.loadavg(), parts: result.parts, logAdded: result.logAdded };
  process.stdout.write(JSON.stringify(final) + '\n');
  process.exitCode = final.error ? 1 : 0;
}

// the measurement runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
