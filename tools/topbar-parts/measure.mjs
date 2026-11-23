// topbar-parts: C-6 (#1311). For the top bar's three parts (Tempo, Song Position, Pattern Selector): a right click shows FL's menu (a new top-level window, class TQuickPopupMenuWindow),
// one item is chosen from it, and the part's value is read before and after from the picture: the RGBA pixels of the part's display (its box) hashed with SHA-256 (the first 16 hex digits).
// Only the picture is compared: no engine text is read. The item of each part is given by its offset from the menu window's top-left (the menu opens at the part; the offset is the same on every start-up).
//   Tempo:            "120 BPM" (the tempo tapper column): the tempo changes from 140.000 to 120.000
//   Song Position:    "Minute : Second : Centisecond": the display changes unit (the position itself stays at 0)
//   Pattern Selector: "Clone": the pattern number changes from 1 to 2
// Order, on the engine's first idle (bootBundle's afterIdle): the usage-data dialog is closed with "No thanks" (guest (513, 308)), the Default row of the start page is double-clicked (guest (124, 81)) and the default project is waited for (the title has changed and the engine is idle).
// Then per part: the picture (before; 1 s with no input, to see whether the box is still), the right button (moved, pressed, 1 s, released, 1 s), the picture (open); a menu is shown when a new window (not the main form) holds the click point (4 px tolerance)
// and at least MENU_MIN_FRAC of its area changed; then the item is chosen (moved, pressed, 60 ms, released, 1.5 s); the windows that are left (a menu or a dialog) are closed with Escape; the picture (after).
// Usage: node tools/topbar-parts/measure.mjs <absolute path to the bundle directory raw/<name>> > parts.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does). Exit 1 when error is set, 2 when no bundle directory is given.
// stdout: ONE JSON line {state, standIn, error, parts: [{name, menuShown, valueBefore, valueAfter, changed}, ...]}. stderr: one line per part (the menu window, the changed fraction, the item, the pixels that changed in the box, and the control).
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';

const MENU_MIN_FRAC = 0.5;
const CONSENT_NO = { x: 513, y: 308 };
const DEFAULT_ROW = { x: 124, y: 81 };
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;
const TOLERANCE_PX = 4;
const MAIN_FORM = 'TFruityLoopsMainForm';
const PARTS = [
  { name: 'Tempo', at: { x: 462, y: 20 }, box: { x0: 437, y0: 8, x1: 491, y1: 33 }, item: { dx: 198, dy: 112 }, what: '"120 BPM"' },
  { name: 'Song Position', at: { x: 566, y: 20 }, box: { x0: 500, y0: 4, x1: 629, y1: 37 }, item: { dx: 84, dy: 46 }, what: '"Minute : Second : Centisecond"' },
  { name: 'Pattern Selector', at: { x: 616, y: 58 }, box: { x0: 592, y0: 48, x1: 639, y1: 76 }, item: { dx: 123, dy: 234 }, what: '"Clone"' },
];

const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title }; })()`;
const PICTURE = `window.__app.deskDumpBase64(true)`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`;

/** the windows of a window list: { lid, x, y, w, h, cls } (the page's own format: lid N at (x, y) WxH ... class "..." title "...") */
function parseWindows(text) {
  return text.split('\n').map((l) => /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l)).filter(Boolean).map((m) => {
    const cls = /class "([^"]*)"/.exec(m[6]);
    return { lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: cls ? cls[1] : '' };
  });
}

/** a picture: the RGBA bytes of a dump (its rows may be longer than width x 4) and the windows of the same dump */
function picture(d) {
  const bytes = Buffer.from(d.base64, 'base64');
  const stride = bytes.length / d.height;
  if (!Number.isInteger(stride) || stride < d.width * 4) throw new Error(`the picture has ${bytes.length} bytes for ${d.width}x${d.height}`);
  return { w: d.width, h: d.height, bytes, stride, windows: parseWindows(d.windows) };
}

/** the SHA-256 (16 hex digits) of the RGBA bytes of a box of the picture: the value as the picture shows it */
function boxHash(pic, b) {
  const h = crypto.createHash('sha256');
  for (let y = b.y0; y < b.y1; y++) h.update(pic.bytes.subarray(y * pic.stride + 4 * b.x0, y * pic.stride + 4 * b.x1));
  return h.digest('hex').slice(0, 16);
}

/** the pixels (R, G, B) of the box that differ between two pictures of the same size */
function boxChanged(a, b, box) {
  if (a.w !== b.w || a.h !== b.h) throw new Error(`the picture changed size: ${a.w}x${a.h} to ${b.w}x${b.h}`);
  let n = 0;
  for (let y = box.y0; y < box.y1; y++) for (let x = box.x0; x < box.x1; x++) {
    const i = y * a.stride + 4 * x, j = y * b.stride + 4 * x;
    if (a.bytes[i] !== b.bytes[j] || a.bytes[i + 1] !== b.bytes[j + 1] || a.bytes[i + 2] !== b.bytes[j + 2]) n++;
  }
  return n;
}

/** the fraction of the window's area (clipped to the picture) whose pixels differ between two pictures */
function areaChangedFraction(a, b, win) {
  const x0 = Math.max(0, win.x), y0 = Math.max(0, win.y), x1 = Math.min(a.w, win.x + win.w), y1 = Math.min(a.h, win.y + win.h);
  let n = 0, total = 0;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    total++;
    const i = y * a.stride + 4 * x, j = y * b.stride + 4 * x;
    if (a.bytes[i] !== b.bytes[j] || a.bytes[i + 1] !== b.bytes[j + 1] || a.bytes[i + 2] !== b.bytes[j + 2]) n++;
  }
  return total === 0 ? 0 : n / total;
}

const holdsNear = (win, p) => p.x >= win.x - TOLERANCE_PX && p.y >= win.y - TOLERANCE_PX && p.x < win.x + win.w + TOLERANCE_PX && p.y < win.y + win.h + TOLERANCE_PX;
/** the windows that are not in the earlier list and are not the main form */
const newWindows = (before, now) => {
  const seen = new Set(before.windows.map((w) => w.lid));
  return now.windows.filter((w) => !seen.has(w.lid) && w.cls !== MAIN_FORM);
};

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

/** clicks the usage-data dialog's No thanks (shown at the first idle; no window of the list holds it), then 2.5 s */
async function closeConsent(page, pic) {
  const c = cssAt(await rectAt(page), pic, CONSENT_NO);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'left', 1, 1);
  await sleep(50);
  await mouse(page, 'mouseReleased', c, 'left', 0, 1);
  await sleep(2500);
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
  const t0 = Date.now();
  for (;;) {
    const s = await page.evaluate(STATE);
    if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} while the default project opens`);
    if (s && s.state === 'engine-idle' && s.title !== before.title) break;
    if (Date.now() - t0 >= OPEN_LIMIT_MS) throw new Error(`timeout ${OPEN_LIMIT_MS} ms: the default project (a new title, then engine-idle)`);
    await sleep(POLL_MS);
  }
  await sleep(2000);
  const after = await page.evaluate(STATE);
  if (after.state !== 'engine-idle') throw new Error(`state ${after.state} 2 s after the default project opened`);
  process.stderr.write(`default project open: title "${before.title}" -> "${after.title}"\n`);
}

/** Escape closes what is open (a menu, or a dialog) until no new window is left (at most 3 times), 1 s after each */
async function closeLeftovers(page, base) {
  let n = 0;
  for (;;) {
    const left = newWindows(base, await picAt(page));
    if (left.length === 0 || n === 3) return { left: left.length, escapes: n };
    await key(page, 'Escape', 27);
    n++;
    await sleep(1000);
  }
}

async function measurePart(page, p) {
  const base = await picAt(page);
  await sleep(1000);
  const still = await picAt(page);
  const control = boxChanged(base, still, p.box);
  const valueBefore = boxHash(still, p.box);
  const r = await rectAt(page);
  const c = cssAt(r, still, p.at);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  await mouse(page, 'mousePressed', c, 'right', 2, 1);
  await sleep(1000);
  await mouse(page, 'mouseReleased', c, 'right', 0, 1);
  await sleep(1000);
  const open = await picAt(page);
  const menu = newWindows(still, open).find((w) => holdsNear(w, p.at)) ?? null;
  const frac = menu ? areaChangedFraction(still, open, menu) : 0;
  const menuShown = menu !== null && frac >= MENU_MIN_FRAC;
  const item = menuShown ? { x: menu.x + p.item.dx, y: menu.y + p.item.dy } : null;
  if (item) {
    const ci = cssAt(r, open, item);
    await mouse(page, 'mouseMoved', ci, 'none', 0, 0);
    await sleep(60);
    await mouse(page, 'mousePressed', ci, 'left', 1, 1);
    await sleep(60);
    await mouse(page, 'mouseReleased', ci, 'left', 0, 1);
    await sleep(1500);
  }
  const leftover = await closeLeftovers(page, still);
  const after = await picAt(page);
  const valueAfter = boxHash(after, p.box);
  const changed = valueBefore !== valueAfter;
  const menuText = menu ? `menu lid ${menu.lid} at (${menu.x}, ${menu.y}) ${menu.w}x${menu.h} class "${menu.cls}" changed ${frac.toFixed(3)}` : 'no menu';
  process.stderr.write(`${p.name} at (${p.at.x}, ${p.at.y}): ${menuText}; item ${item ? `${p.what} at (${item.x}, ${item.y})` : 'none'}; box changed ${boxChanged(still, after, p.box)} px (control ${control} px); leftover ${leftover.left} escape ${leftover.escapes}; ${valueBefore} -> ${valueAfter}\n`);
  return { name: p.name, menuShown, valueBefore, valueAfter, changed };
}

async function session(page, result) {
  let pic = await picAt(page);
  await closeConsent(page, pic);
  pic = await picAt(page);
  await openDefault(page, pic);
  for (const p of PARTS) result.parts.push(await measurePart(page, p));
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/topbar-parts/measure.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  const result = { parts: [] };
  let ran = false;
  const { out } = await bootBundle(bundleDir, {
    afterIdle: async (page) => {
      ran = true;
      await session(page, result);
    },
  });
  if (!ran && !out.error) out.error = `the session did not run: state ${out.state}`;
  const final = { state: out.state, standIn: out.standIn, error: out.error, parts: result.parts };
  process.stdout.write(JSON.stringify(final) + '\n');
  process.exitCode = final.error ? 1 : 0;
}

// the measurement runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
