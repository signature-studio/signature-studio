// The visual area's steps (Issue #1496). The pictures are read from the page (common.picture: the guest's RGBA bytes); the actions are a person's (lib/guest.mjs:
// clicks, keys, drags). The golden hashes are the SHA-256 of a rectangle's RGB bytes (golden/visual/<kind>.tsv: key, hash, tolerance). Each golden also keeps a
// 16 x 16 cell signature (golden/visual/<kind>.cells.tsv: the mean R, G, B of each cell), so that a change of the hash is judged by the share of cells that moved
// (the tolerance of its row) and not only as "different". A key that has no row is recorded (the first run). Pure functions are exported for the tests.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { absoluteRect, subtree } from './dump.mjs';
import { clipRect, regionHash } from './picture.mjs';
import { closeNew, menuBarRect, newWindows, openLids, picture, startProject, waitWindow, windowTable, MAIN_FORM } from './common.mjs';
import { sleep } from './flow.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..', '..');
export const GOLDEN_DIR = path.resolve(here, '..', 'golden', 'visual');
/** the cells of a rectangle's signature (CELLS x CELLS) */
export const CELLS = 16;
/** a cell has moved when one of its mean channels changed by more than this (of 255) */
export const CELL_LIMIT = 8;
/** the share of cells that may move before a golden is a difference (0.02 = 5 of the 256 cells) */
export const TOLERANCE = 0.02;
export const DEFAULT_THEME = '(Default)';
/** the 13 themes (the file's name in Artwork\Themes\ is the name the Theme page shows) */
export const THEMES = [
  ['ash', 'Ash'], ['dedzii', 'DEDZII'], ['dedzii-light', 'DEDZII Light'], ['dark', 'Dark'], ['dark-orange', 'Dark orange'], ['light-cherry', 'Light cherry'],
  ['light-pear', 'Light pear'], ['light-tangerine', 'Light tangerine'], ['ppriime-dark', 'PPRIIME Dark'], ['ppriime-light', 'PPRIIME Light'], ['sytrus', 'Sytrus'],
  ['ultra-green', 'Ultra green'], ['veela', 'Veela'],
];
/** the 7 languages of the golden set (the item of the Language list by its native name); English is the start language */
export const LANGUAGES = {
  de: /Deutsch|German/i, es: /Español|Espanol|Spanish/i, fr: /Français|Francais|French/i, ja: /日本語|Japanese/i,
  ko: /한국어|Korean/i, vi: /Tiếng Việt|Tieng Viet|Vietnamese/i, zh: /中文|简体|Chinese/i,
};
const LANGUAGE_NAMES = { en: /English/i, ...LANGUAGES };
/** the windows of the golden set: the tail is the windows.tsv row's ID (windows/<kind>/<tail>) */
export const WINDOWS = ['right-click-channel-rack', 'right-click-mixer', 'right-click-playlist', 'tedgebrowser', 'tpluglistform', 'tmidiform', 'tmidiform-2', 'tpluglistform-2'];
/** the DPRs of the golden set: the fixed ones, and two drawn at a fixed seed (so that the keys are the same in every run) */
export const FIXED_DPRS = [['1', 1], ['1-25', 1.25], ['1-5', 1.5], ['1-75', 1.75], ['2', 2], ['2-5', 2.5], ['3', 3]];
const SETTINGS = 'TMIDIForm', BAR = 'TQuickSheetSelector', SHEET = 'TVectorSheet', GRID = 'TQuickStringGrid', SCROLLER = 'TQuickScrollBoxScroller';
const TOPBAR = 'TQuickControlBar';
const SLOTS = 10;
// the tiles of the Theme page's grid (three columns; a row is TILE_PITCH of a column's width; the image is TILE_IMAGE_DY below the grid's top) (tools/theme-switch)
const TILE_PITCH = 0.7, TILE_IMAGE_DY = 47, TILE_MARGIN = 24;
const TEXT_LIMIT = { colours: 8, range: 40 };
const lum = (pic, x, y) => { const i = y * pic.stride + 4 * x; return 0.299 * pic.bytes[i] + 0.587 * pic.bytes[i + 1] + 0.114 * pic.bytes[i + 2]; };
const median = (a) => { const s = [...a].sort((p, q) => p - q); return s[Math.floor(s.length / 2)]; };

/** the ratio's key form: the measured device pixel ratio to two decimals (1.25 is `1.25`, 1 is `1`) */
export const dprKey = (v) => String(Math.round(Number(v) * 100) / 100);

// ---- the golden tables -------------------------------------------------------------------------------------------------------------------------------

/** the cells of a rectangle as hex: CELLS x CELLS mean (R, G, B) bytes, row by row (a rectangle outside the picture gives zeros) */
export function cellsOf(pic, rect) {
  const out = Buffer.alloc(CELLS * CELLS * 3);
  const c = clipRect(pic, rect);
  if (c === null) return out.toString('hex');
  const [x0, y0, x1, y1] = c;
  let k = 0;
  for (let cy = 0; cy < CELLS; cy++) {
    const ay = y0 + Math.floor((cy * (y1 - y0)) / CELLS), by = y0 + Math.floor(((cy + 1) * (y1 - y0)) / CELLS);
    for (let cx = 0; cx < CELLS; cx++) {
      const ax = x0 + Math.floor((cx * (x1 - x0)) / CELLS), bx = x0 + Math.floor(((cx + 1) * (x1 - x0)) / CELLS);
      let r = 0, g = 0, b = 0, n = 0;
      for (let y = ay; y < by; y++) {
        for (let x = ax; x < bx; x++) {
          const i = y * pic.stride + 4 * x;
          r += pic.bytes[i]; g += pic.bytes[i + 1]; b += pic.bytes[i + 2]; n++;
        }
      }
      out[k++] = n ? Math.round(r / n) : 0; out[k++] = n ? Math.round(g / n) : 0; out[k++] = n ? Math.round(b / n) : 0;
    }
  }
  return out.toString('hex');
}

/** the share of cells (0 to 1) whose mean colour moved by more than CELL_LIMIT in some channel; null when the signatures do not match in size */
export function cellsShare(a, b) {
  const A = Buffer.from(a, 'hex'), B = Buffer.from(b, 'hex');
  if (A.length !== B.length || A.length !== CELLS * CELLS * 3) return null;
  let moved = 0;
  for (let c = 0; c < CELLS * CELLS; c++) {
    let d = 0;
    for (let j = 0; j < 3; j++) d = Math.max(d, Math.abs(A[3 * c + j] - B[3 * c + j]));
    if (d > CELL_LIMIT) moved++;
  }
  return moved / (CELLS * CELLS);
}

/** the rows of a golden table: key -> { value, tol } (the cells table has no tolerance); '#' lines are the header */
function readRows(file) {
  const m = new Map();
  if (!fs.existsSync(file)) return m;
  for (const l of fs.readFileSync(file, 'utf8').split('\n')) {
    if (l.length === 0 || l.startsWith('#')) continue;
    const [k, v, t] = l.split('\t');
    m.set(k, { value: v, tol: t === undefined ? null : Number(t) });
  }
  return m;
}

function appendRow(file, header, row) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  if (!fs.existsSync(file)) fs.writeFileSync(file, `${header}\n`);
  fs.appendFileSync(file, `${row}\n`);
}

/**
 * judges a rectangle against the golden of its kind (golden/visual/<kind>.tsv) and key: status 'recorded' (no row yet: the picture is written as the golden, the first run),
 * 'same' (the hash is the golden's), 'within' (the hash differs, the cells that moved are within the row's tolerance) or 'differs'.
 * diffShare is the share of cells that moved (0 for 'same' and 'recorded'); the golden's hash and tolerance are in the result.
 */
export function judgeGolden(kind, key, pic, rect, tolerance = TOLERANCE) {
  const hash = regionHash(pic, rect);
  const cells = cellsOf(pic, rect);
  const hashFile = path.join(GOLDEN_DIR, `${kind}.tsv`), cellsFile = path.join(GOLDEN_DIR, `${kind}.cells.tsv`);
  const ref = readRows(hashFile).get(key);
  if (!ref) {
    appendRow(hashFile, '# key\tsha256 (RGB of the rectangle)\ttolerance (share of cells)', `${key}\t${hash}\t${tolerance}`);
    appendRow(cellsFile, '# key\tcells (16 x 16 mean R G B, hex)', `${key}\t${cells}`);
    return { key, status: 'recorded', hash, golden: null, diffShare: 0, tolerance, rect: roundRect(rect) };
  }
  if (ref.value === hash) return { key, status: 'same', hash, golden: ref.value, diffShare: 0, tolerance: ref.tol, rect: roundRect(rect) };
  const refCells = readRows(cellsFile).get(key)?.value ?? null;
  const diffShare = refCells ? cellsShare(refCells, cells) : null;
  const status = diffShare !== null && diffShare <= ref.tol ? 'within' : 'differs';
  return { key, status, hash, golden: ref.value, diffShare, tolerance: ref.tol, rect: roundRect(rect) };
}

const roundRect = (r) => (r ? { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.w), h: Math.round(r.h) } : null);

// ---- edges, text, cursor (judged by the picture alone) --------------------------------------------------------------------------------------------

/**
 * the 12 luminances across one side of a rectangle, from the outside (index 0) to the inside (index 11); the edge line is between indices 5 and 6 (the inside
 * begins at index 6). null when the side's outside is not in the picture (a window that fills the picture has no outside on that side).
 */
export function sideProfile(pic, rect, side) {
  const vert = side === 'left' || side === 'right';
  const mx = Math.round(rect.x + rect.w / 2), my = Math.round(rect.y + rect.h / 2);
  const E = { left: rect.x, right: rect.x + rect.w, top: rect.y, bottom: rect.y + rect.h }[side];
  const forward = side === 'left' || side === 'top';
  const pts = Array.from({ length: 12 }, (_, i) => {
    const d = forward ? E - 6 + i : E + 5 - i;
    return vert ? [d, my] : [mx, d];
  });
  if (pts.some(([x, y]) => x < 0 || y < 0 || x >= pic.w || y >= pic.h)) return null;
  return pts.map(([x, y]) => lum(pic, x, y));
}

/**
 * one side's reading: the contrast across the edge (inside minus outside, of the plateaus), the frame (pixels in the 6 px at the edge that differ from both plateaus:
 * the window's own line, its width in device px), the soft band (pixels between 10 % and 90 % of the step: a blurred edge), and sharp (band <= 2 and frame <= 2).
 */
export function sideReading(p) {
  const outer = median(p.slice(0, 4)), inner = median(p.slice(8, 12));
  const c = inner - outer;
  let frame = 0, band = 0;
  for (let i = 4; i <= 9; i++) {
    const v = p[i];
    if (Math.abs(v - outer) > 8 && Math.abs(v - inner) > 8) frame++;
    const t = c === 0 ? 0 : (v - outer) / c;
    if (t > 0.1 && t < 0.9) band++;
  }
  // the step's size (contrast) is recorded, not judged: the settings window's frame is a faint step in the default theme (about 9 of 255)
  return { contrast: Math.round(c), frame, band, sharp: band <= 2 && frame <= 2 && Math.abs(c) >= 1 };
}

/** the four sides of a rectangle read: { sides: { left, right, top, bottom }, measured (the sides with an outside), width (the median frame), sharp (2 or more sides measured, all sharp) } */
export function edgeReadings(pic, rect) {
  const sides = {};
  for (const s of ['left', 'right', 'top', 'bottom']) {
    const p = sideProfile(pic, rect, s);
    sides[s] = p === null ? { measured: false } : { measured: true, ...sideReading(p) };
  }
  const got = Object.values(sides).filter((s) => s.measured);
  const frames = got.map((s) => s.frame).filter((v) => v !== null);
  return {
    sides, measured: got.length, width: frames.length ? median(frames) : null,
    sharp: got.length >= 2 && got.every((s) => s.sharp === true),
  };
}

/** the colours and the luminance range of a rectangle (every 2nd pixel): a drawn text has several colours and a range; a single colour has none */
export function textReading(pic, rect) {
  const c = clipRect(pic, rect);
  if (c === null) return { colours: 0, range: 0 };
  const [x0, y0, x1, y1] = c;
  const seen = new Set();
  let lo = 255, hi = 0;
  for (let y = y0; y < y1; y += 2) {
    for (let x = x0; x < x1; x += 2) {
      const i = y * pic.stride + 4 * x;
      seen.add(((pic.bytes[i] >> 4) << 8) | ((pic.bytes[i + 1] >> 4) << 4) | (pic.bytes[i + 2] >> 4));
      const l = lum(pic, x, y);
      if (l < lo) lo = l;
      if (l > hi) hi = l;
    }
  }
  return { colours: seen.size, range: Math.round(hi - lo) };
}

/** the reading of a computed cursor: whether it is an image-set and its declared resolutions (`x` or `dppx`); the data is not kept */
export function cursorReading(css) {
  const isImageSet = css.startsWith('image-set(');
  const shape = css.replace(/base64,[A-Za-z0-9+/=]+/g, 'base64,');
  const scales = [...shape.matchAll(/([0-9]*\.?[0-9]+)(?:dppx|x)\b/g)].map((m) => Number(m[1]));
  return { isImageSet, scales, sameRatio: (dpr) => isImageSet && scales.length === 1 && Math.abs(scales[0] - dpr) <= 0.01, shape: shape.slice(0, 160) };
}

// ---- the page: the settings window, the themes, the languages, the ratio --------------------------------------------------------------------------

/** the order of the themes on the Theme page (the sub-folders first, then the others; the list's order in each), as tools/theme-switch reads it */
export function themeOrderNames() {
  const tsv = fs.readFileSync(path.join(repo, 'inputs', 'fl-installer-files.tsv'), 'utf8');
  const files = tsv.replace(/\r/g, '').split('\n').map((l) => l.split('\t')[0])
    .filter((p) => /^Artwork\\Themes\\.*\.flstheme$/i.test(p))
    .map((file) => ({ file, name: file.split('\\').pop().replace(/\.flstheme$/i, '') }));
  const sub = (f) => (f.file.split('\\').length > 3 ? 0 : 1);
  return files.slice().sort((a, b) => sub(a) - sub(b)).map((f) => f.name);
}

/** the tile of a theme on the Theme page: 0 is the built-in (Default), theme i is tile i + 1 */
export function tileOfTheme(name) {
  if (name === DEFAULT_THEME) return 0;
  const i = themeOrderNames().indexOf(name);
  if (i < 0) throw new Error(`theme ${name} is not in inputs/fl-installer-files.tsv`);
  return i + 1;
}

/** the scroll shift between two profiles of one grid (tools/theme-switch: the least mean difference over the overlap) */
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

/** the state of a row's page (each row has its own page: a new Guest): the theme and the language now (null until a scenario sets them: the first one always sets its own), and the ratio at the start */
const STATE = new WeakMap();
const stateOf = (guest) => {
  if (!STATE.has(guest)) STATE.set(guest, { theme: null, language: null, dprBase: null });
  return STATE.get(guest);
};

/** the settings window (F10 when it is not open); its tab bar's slots are pressed in turn until the page's caption is `caption`. Returns { table, top, page } */
export async function openSettingsPage(guest, caption) {
  let t = await windowTable(guest);
  if (!t.top.some((w) => w.cls === SETTINGS)) {
    await guest.focusDesktop();
    await guest.key('F10', 'F10', 121);
    if (!(await waitWindow(guest, SETTINGS, 15000))) {
      const tops = (await windowTable(guest)).top.map((w) => `${w.cls} "${w.title}"`).join('; ');
      throw new Error(`F10 opened no TMIDIForm (the Settings window) within 15 s; top-level windows: ${tops.slice(0, 600)}`);
    }
    await sleep(1000);
    t = await windowTable(guest);
  }
  for (let s = 0; s <= SLOTS; s++) {
    const top = t.top.find((w) => w.cls === SETTINGS) ?? null;
    const page = top ? t.children.find((c) => c.cls === SHEET && c.parent === top.lid && c.vis) ?? null : null;
    if (page && page.text === caption) return { table: t, top, page };
    if (s === SLOTS || !top) break;
    const bar = t.children.find((c) => c.cls === BAR && c.parent === top.lid);
    if (!bar) throw new Error('no tab bar (TQuickSheetSelector) in the Settings window');
    const b = absoluteRect(t, bar.lid);
    await guest.click(b.x + ((s + 0.5) * b.w) / SLOTS, b.y + b.h / 2);
    await sleep(1000);
    t = await windowTable(guest);
  }
  throw new Error(`the ${caption} page was not reached by the tab bar's ${SLOTS} slots`);
}

/** the Settings window is closed (Escape while it is listed); returns true when it is gone */
export async function closeSettings(guest) {
  for (let i = 0; i < 4; i++) {
    const t = await windowTable(guest);
    if (!t.top.some((w) => w.cls === SETTINGS)) return true;
    await guest.focusDesktop();
    await guest.escape();
    await sleep(800);
  }
  return false;
}

const dragThumb = (guest, sr, toTop) => guest.drag({ x: sr.x + sr.w / 2, y: sr.y + sr.h / 2 }, { x: sr.x + sr.w / 2, y: toTop ? sr.y - 300 : sr.y + sr.h + 300 }, 10);

/** the grid's first column read row by row (the luminance), after the engine has drawn it again: the profile the scroll is measured on */
async function gridProfileNow(guest, g) {
  await sleep(1500);
  const pic = await picture(guest);
  const xs = [g.x + g.w / 6 - 24, g.x + g.w / 6, g.x + g.w / 6 + 24].map((x) => Math.round(x));
  const out = new Array(Math.max(0, Math.floor(g.h))).fill(0);
  for (let k = 0; k < out.length; k++) {
    const y = Math.round(g.y) + k;
    let s = 0, c = 0;
    for (const x of xs) {
      if (x < 0 || x >= pic.w || y < 0 || y >= pic.h) continue;
      s += lum(pic, x, y); c++;
    }
    out[k] = c ? s / c : 0;
  }
  return out;
}

/** the guest point of a tile: in the top view when it is shown there; else the grid is scrolled to the bottom and the tile is at its content minus the measured shift */
async function tilePoint(guest, g, sr, k) {
  const colW = g.w / 3;
  const cx = g.x + ((k % 3) + 0.5) * colW;
  const content = g.y + Math.floor(k / 3) * colW * TILE_PITCH + TILE_IMAGE_DY;
  await dragThumb(guest, sr, true);
  if (content <= g.y + g.h - TILE_MARGIN) return { x: cx, y: content };
  const top = await gridProfileNow(guest, g);
  await dragThumb(guest, sr, false);
  const bottom = await gridProfileNow(guest, g);
  const range = scrollShift(top, bottom, top.length, 0.2);
  const y = content - (range ?? 0);
  return y < g.y + TILE_MARGIN || y > g.y + g.h - TILE_MARGIN ? null : { x: cx, y };
}

/** the Theme page: the tile of the theme is pressed (the theme is applied at the press), then the Settings window is closed */
export async function pickTheme(guest, name) {
  const page = await openSettingsPage(guest, 'Theme');
  const kids = subtree(page.table, page.page.lid);
  const grid = kids.find((c) => c.cls === GRID), scroller = kids.find((c) => c.cls === SCROLLER);
  if (!grid || !scroller) throw new Error('the Theme page has no grid or no scroll bar');
  const pt = await tilePoint(guest, absoluteRect(page.table, grid.lid), absoluteRect(page.table, scroller.lid), tileOfTheme(name));
  if (!pt) throw new Error(`the tile of ${name} is not in view of the Theme grid`);
  await guest.click(pt.x, pt.y);
  await sleep(2500);
  await closeSettings(guest);
}

/** the General page: the Language list is opened, its item of the language is pressed (by its name; else the keys: Down and Enter), the restart notice is closed */
export async function pickLanguage(guest, code) {
  const page = await openSettingsPage(guest, 'General');
  const t = page.table;
  const panel = subtree(t, page.page.lid).find((c) => c.cls === 'TVectorPanel' && /^Language/.test(c.text));
  if (!panel) throw new Error('no Language panel on the General page');
  const a = absoluteRect(t, panel.lid);
  const before = new Set([...t.top.map((w) => w.lid), ...t.children.map((c) => c.lid)]);
  await guest.click(a.x + 12 + 87, a.y + 20 + 11);
  await sleep(1500);
  const t2 = await windowTable(guest);
  const items = t2.children.filter((c) => !before.has(c.lid) && c.vis && c.text);
  const target = items.find((c) => LANGUAGE_NAMES[code].test(c.text)) ?? null;
  if (target) {
    const r = absoluteRect(t2, target.lid);
    await guest.click(r.x + r.w / 2, r.y + r.h / 2);
  } else {
    const k = code === 'en' ? 0 : 1 + Object.keys(LANGUAGES).indexOf(code);
    for (let i = 0; i < k; i++) await guest.key('ArrowDown', 'ArrowDown', 40);
    await guest.key('Enter', 'Enter', 13);
  }
  await sleep(2500);
  let notice = false;
  for (let i = 0; i < 3; i++) {
    const t3 = await windowTable(guest);
    if (!t3.top.some((w) => w.title === 'Language change')) break;
    notice = true;
    await guest.escape();
    await sleep(800);
  }
  const t4 = await windowTable(guest);
  const shown = subtree(t4, page.page.lid).find((c) => c.cls === 'TVectorPanel' && /^Language/.test(c.text))?.text ?? null;
  await closeSettings(guest);
  return { matched: target !== null, label: target ? target.text : null, notice, shown };
}

/**
 * the engine has stopped (a scenario before this one ended it): the page is loaded again and the project started again, at the ratio the row started with.
 * The theme and the language are unknown after it (they are set again by ensure). Returns true when the page was loaded again.
 */
async function recover(ctx) {
  const s = await ctx.guest.state().catch(() => null);
  if (s && s.state === 'engine-idle') return false;
  await ctx.page.reload({ waitUntil: 'load', timeout: 120000 });
  const start = await startProject(ctx.guest);
  const st = stateOf(ctx.guest);
  st.theme = null;
  st.language = null;
  if (st.dprBase) await restoreDpr(ctx);
  if (!start.opened) throw new Error(`the page was loaded again but the project did not open (state ${start.state})`);
  return true;
}

/** the start state of a scenario: the theme and the language it needs (the default theme and English when none is given); changes only what differs */
export async function ensure(ctx, { theme = DEFAULT_THEME, language = 'en' } = {}) {
  const reloaded = await recover(ctx);
  const st = stateOf(ctx.guest);
  const did = reloaded ? ['page loaded again (the engine had stopped)'] : [];
  if (st.theme !== theme) { await pickTheme(ctx.guest, theme); st.theme = theme; did.push(`theme ${theme}`); }
  if (st.language !== language) {
    const l = await pickLanguage(ctx.guest, language);
    st.language = language;
    did.push(`language ${language} (${l.matched ? `item "${l.label}"` : 'by the keys'}; restart notice ${l.notice}; the Language panel reads ${JSON.stringify(l.shown)})`);
  }
  return did;
}

/** the device pixel ratio is set through the page's emulation: the physical size of the start is kept (the CSS size is the physical size / the ratio) */
export async function setDpr(ctx, value) {
  const st = stateOf(ctx.guest);
  if (!st.dprBase) st.dprBase = await ctx.guest.ev('({ w: window.innerWidth, h: window.innerHeight, d: window.devicePixelRatio })');
  const b = st.dprBase;
  await ctx.cdp.send('Emulation.setDeviceMetricsOverride', { width: Math.round((b.w * b.d) / value), height: Math.round((b.h * b.d) / value), deviceScaleFactor: value, mobile: false });
  await ctx.guest.waitFor(async () => (Math.abs((await ctx.guest.ev('window.devicePixelRatio')) - value) < 0.005 ? true : null), 20000, 'the ratio');
  await sleep(3000);
  return ctx.guest.ev('window.devicePixelRatio');
}

/** the ratio of the start again (setDpr keeps the start's CSS size and ratio in the row's state) */
export async function restoreDpr(ctx) {
  const b = stateOf(ctx.guest).dprBase;
  if (!b) return;
  await ctx.cdp.send('Emulation.setDeviceMetricsOverride', { width: b.w, height: b.h, deviceScaleFactor: b.d, mobile: false });
  await ctx.guest.waitFor(async () => (Math.abs((await ctx.guest.ev('window.devicePixelRatio')) - b.d) < 0.005 ? true : null), 20000, 'the ratio back');
  await sleep(3000);
}

export const currentDpr = async (guest) => dprKey(await guest.ev('window.devicePixelRatio'));

// ---- the windows of the set (windows.tsv) ----------------------------------------------------------------------------------------------------------

/** a rectangle of the window table: a child (its absolute place) or a top-level window */
const childRect = (t, cls) => {
  const c = t.children.find((x) => x.cls === cls && x.w > 0 && x.h > 0 && x.vis);
  return c ? absoluteRect(t, c.lid) : null;
};

/** opens the window of the set by its tail (the windows.tsv row's ID): returns { rect, cls, title, base } (base: the top-level windows before it, for closeNew) */
export async function openWindow(guest, tail) {
  const base = await openLids(guest);
  const t = await windowTable(guest);
  const popup = async (cls, dx, dy) => {
    const r = childRect(t, cls);
    if (!r) throw new Error(`no visible ${cls} to right-click`);
    await guest.rightClick(r.x + dx, r.y + dy, 60);
    const pop = await guest.waitFor(async () => (await newWindows(guest, base)).pops[0] ?? null, 6000, 'the drop-down');
    if (!pop) throw new Error(`no drop-down after the right-click on ${cls}`);
    return { rect: { x: pop.x, y: pop.y, w: pop.w, h: pop.h }, cls: pop.cls, title: pop.title };
  };
  const key = async (code, vk, mod, cls) => {
    await guest.focusDesktop();
    await guest.key(code, code, vk, mod);
    const w = await waitWindow(guest, cls, 8000);
    if (!w) throw new Error(`${code} opened no ${cls}`);
    return { rect: { x: w.x, y: w.y, w: w.w, h: w.h }, cls: w.cls, title: w.title };
  };
  let r;
  switch (tail) {
    case 'right-click-channel-rack': r = await popup('TStepSeqForm', 20, 60); break;
    case 'right-click-mixer': r = await popup('TFXForm', 20, 60); break;
    case 'right-click-playlist': r = await popup('TEventEditForm', 20, 60); break;
    case 'tedgebrowser': {
      const rect = childRect(t, 'TEdgeBrowser');
      if (!rect) throw new Error('no visible TEdgeBrowser (the Browser panel) in the window table');
      r = { rect, cls: 'TEdgeBrowser', title: '' };
      break;
    }
    case 'tpluglistform': r = await key('F8', 119, 0, 'TPlugListForm'); break;
    case 'tmidiform': r = await key('F10', 121, 0, SETTINGS); break;
    case 'tmidiform-2': r = await key('F11', 122, 0, SETTINGS); break;
    case 'tpluglistform-2': r = await key('F8', 119, 1, 'TPlugListForm'); break;
    default: throw new Error(`no window ${tail} in the set`);
  }
  return { ...r, base };
}

/** the main window's rectangle (TFruityLoopsMainForm, top-level) */
export async function mainRect(guest) {
  const t = await windowTable(guest);
  const m = t.top.find((w) => w.cls === MAIN_FORM);
  if (!m) throw new Error('the main form is not in the window table');
  return { x: m.x, y: m.y, w: m.w, h: m.h };
}

/** the top bar's rectangle: the visible TQuickControlBar with a height (the bar at the top of the picture) */
export async function topbarRect(guest) {
  const t = await windowTable(guest);
  const c = t.children.find((x) => x.cls === TOPBAR && x.vis && x.h > 0);
  if (!c) throw new Error('no visible TQuickControlBar (the top bar) in the window table');
  return absoluteRect(t, c.lid);
}

/** the cursor of the page after it has settled (two readings in a row agree) */
async function settledCursor(guest) {
  let last = null;
  for (let i = 0; i < 10; i++) {
    await sleep(i === 0 ? 400 : 250);
    const v = await guest.ev("getComputedStyle(document.getElementById('desktop')).cursor");
    if (v === last) return v;
    last = v;
  }
  return last;
}

/** the cursor read at a guest pixel: moves the pointer there and reads the cursor of the desktop once it has settled */
export async function cursorAt(guest, x, y) {
  await guest.move(await guest.pointAt(x, y));
  return settledCursor(guest);
}

// ---- the runs of the scenarios (each returns { ok, engineState, notes, results }) -------------------------------------------------------------

const themeName = (slug) => {
  const t = THEMES.find(([s]) => s === slug);
  if (!t) throw new Error(`no theme ${slug} in the set`);
  return t[1];
};
const GOLDEN_OK = ['recorded', 'same', 'within'];
const judgedNote = (j) => `${j.key}: ${j.status}${j.diffShare === null ? ' (no cells to compare)' : ` (cells moved ${(100 * j.diffShare).toFixed(2)} %, tolerance ${(100 * j.tolerance).toFixed(2)} %)`}`;
const edgeNote = (e) => Object.entries(e.sides).map(([s, r]) => (r.measured ? `${s} contrast ${r.contrast} frame ${r.frame} band ${r.band}` : `${s} no outside`)).join('; ');

// the visual scenarios run at 4k-dpr1 only: the golden keys carry the ratio, and a random display would give keys that change in every run (the ratios of the
// set are put by the scenario itself: dpr-* scenarios)
const DISPLAY_NOTE = 'display 4k-dpr1 only (the golden keys carry the ratio; the random display would change the keys in every run)';
async function result(ctx, ok, notes, results = {}) {
  return { ok: ok === true, engineState: (await ctx.guest.state().catch(() => null))?.state ?? null, notes: [DISPLAY_NOTE, ...notes], results };
}

/** a window of the set: its picture is the golden of its kind (window.tsv) at the ratio */
export async function windowGolden(ctx, tail) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  const w = await openWindow(guest, tail);
  try {
    await sleep(1500);
    const dpr = await currentDpr(guest);
    const j = judgeGolden('window', `${tail}@dpr${dpr}`, await picture(guest), w.rect);
    return await result(ctx, GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), `${w.cls} ${JSON.stringify(roundRect(w.rect))}`, judgedNote(j)], { ...j, window: w.cls, title: w.title, dpr });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** a window of the set: its four edges are sharp (no blur) and the frame is 2 device px or less */
export async function windowEdge(ctx, tail) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  const w = await openWindow(guest, tail);
  try {
    await sleep(1500);
    const e = edgeReadings(await picture(guest), w.rect);
    const gap = e.measured < 2 ? ['the window fills the picture: its sides have no outside to read against, so this method cannot judge its edges'] : [];
    return await result(ctx, e.sharp, [...did.map((d) => `set ${d}`), `${w.cls} ${JSON.stringify(roundRect(w.rect))}; ${edgeNote(e)}`, ...gap], { edgeWidth: e.width, measuredSides: e.measured, sides: e.sides, dpr: await currentDpr(guest), window: w.cls });
  } finally {
    await closeNew(guest, w.base);
  }
}

/**
 * the all-black 64 x 64 tiles of the picture (every pixel R, G and B 0), counted apart: outside the Browser panel (TEdgeBrowser, the web view) and inside it.
 * `black` is the count the criterion reads (outside); `inBrowser` is the panel's own (reported, not judged here); bbox is the box of the outside tiles.
 */
export async function blackReport(guest) {
  const pic = await picture(guest);
  const t = await windowTable(guest);
  const browser = childRect(t, 'TEdgeBrowser');
  let black = 0, inBrowser = 0;
  let x0 = Infinity, y0 = Infinity, x1 = -1, y1 = -1;
  for (let ty = 0; ty < pic.h; ty += 64) {
    for (let tx = 0; tx < pic.w; tx += 64) {
      const ex = Math.min(pic.w, tx + 64), ey = Math.min(pic.h, ty + 64);
      let isBlack = true;
      for (let y = ty; y < ey && isBlack; y++) {
        for (let x = tx; x < ex; x++) {
          const i = y * pic.stride + 4 * x;
          if (pic.bytes[i] | pic.bytes[i + 1] | pic.bytes[i + 2]) { isBlack = false; break; }
        }
      }
      if (!isBlack) continue;
      const cx = (tx + ex) / 2, cy = (ty + ey) / 2;
      if (browser && cx >= browser.x && cx < browser.x + browser.w && cy >= browser.y && cy < browser.y + browser.h) { inBrowser++; continue; }
      black++;
      x0 = Math.min(x0, tx); y0 = Math.min(y0, ty); x1 = Math.max(x1, ex); y1 = Math.max(y1, ey);
    }
  }
  return { black, inBrowser, bbox: black ? [x0, y0, x1, y1] : null, browser: roundRect(browser) };
}

const blackNote = (r) => `black tiles outside the Browser panel ${r.black}${r.bbox ? ` (box ${r.bbox.join(',')})` : ''}; inside the Browser panel ${r.inBrowser}`;

/** after a window of the set is open: the picture has no all-black 64 x 64 tile outside the Browser panel */
export async function windowBlack(ctx, tail) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  const w = await openWindow(guest, tail);
  try {
    await sleep(1000);
    const r = await blackReport(guest);
    return await result(ctx, r.black === 0, [...did.map((d) => `set ${d}`), `with ${w.cls} open: ${blackNote(r)}`], { black: r.black, browserBlack: r.inBrowser, bbox: r.bbox, browser: r.browser, window: w.cls });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** the main window's rectangle under the theme: its picture is the golden of the theme (theme.tsv) */
export async function themeGolden(ctx, slug) {
  const { guest } = ctx;
  const did = await ensure(ctx, { theme: themeName(slug) });
  const rect = await mainRect(guest);
  const dpr = await currentDpr(guest);
  const j = judgeGolden('theme', `${slug}@dpr${dpr}`, await picture(guest), rect);
  return result(ctx, GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), judgedNote(j)], { ...j, theme: themeName(slug), dpr });
}

/** after the theme is set: no all-black 64 x 64 tile */
export async function themeBlack(ctx, slug) {
  const did = await ensure(ctx, { theme: themeName(slug) });
  const r = await blackReport(ctx.guest);
  return result(ctx, r.black === 0, [...did.map((d) => `set ${d}`), `under ${themeName(slug)}: ${blackNote(r)}`], { black: r.black, browserBlack: r.inBrowser, bbox: r.bbox, theme: themeName(slug) });
}

/** the menu bar's text (the top bar's labels) is drawn under the theme: several colours and a luminance range */
export async function themeText(ctx, slug) {
  const did = await ensure(ctx, { theme: themeName(slug) });
  return textCheck(ctx, [...did.map((d) => `set ${d}`)], { theme: themeName(slug) });
}

async function textCheck(ctx, notes, extra) {
  const t = await windowTable(ctx.guest);
  const rect = menuBarRect(t);
  if (!rect) return result(ctx, false, [...notes, 'no menu bar (TNewMenu) in the window table'], extra);
  const r = textReading(await picture(ctx.guest), rect);
  const ok = r.colours >= TEXT_LIMIT.colours && r.range >= TEXT_LIMIT.range;
  return result(ctx, ok, [...notes, `menu bar ${JSON.stringify(roundRect(rect))}: ${r.colours} colours, luminance range ${r.range}`], { ...extra, text: r, rect: roundRect(rect) });
}

/** the Settings window's edges under the theme (the same window as the window set's tmidiform) */
export async function themeEdge(ctx, slug) {
  const { guest } = ctx;
  const did = await ensure(ctx, { theme: themeName(slug) });
  const w = await openWindow(guest, 'tmidiform');
  try {
    await sleep(1500);
    const e = edgeReadings(await picture(guest), w.rect);
    return await result(ctx, e.sharp, [...did.map((d) => `set ${d}`), `${w.cls} ${JSON.stringify(roundRect(w.rect))}; ${edgeNote(e)}`], { edgeWidth: e.width, measuredSides: e.measured, sides: e.sides, theme: themeName(slug) });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** the top bar under the language: its picture is the golden of the language (language.tsv) */
export async function languageTopbar(ctx, code) {
  const { guest } = ctx;
  const did = await ensure(ctx, { language: code });
  const rect = await topbarRect(guest);
  const dpr = await currentDpr(guest);
  const j = judgeGolden('language', `${code}/topbar@dpr${dpr}`, await picture(guest), rect);
  return result(ctx, GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), judgedNote(j)], { ...j, language: code, dpr });
}

/** the Settings window under the language: its picture is the golden of the language */
export async function languageSettings(ctx, code) {
  const { guest } = ctx;
  const did = await ensure(ctx, { language: code });
  const w = await openWindow(guest, 'tmidiform');
  try {
    await sleep(1500);
    const dpr = await currentDpr(guest);
    const j = judgeGolden('language', `${code}/settings@dpr${dpr}`, await picture(guest), w.rect);
    return await result(ctx, GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), judgedNote(j)], { ...j, language: code, dpr });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** after the language is set: no all-black 64 x 64 tile */
export async function languageBlack(ctx, code) {
  const did = await ensure(ctx, { language: code });
  const r = await blackReport(ctx.guest);
  return result(ctx, r.black === 0, [...did.map((d) => `set ${d}`), `under ${code}: ${blackNote(r)}`], { black: r.black, browserBlack: r.inBrowser, bbox: r.bbox, language: code });
}

/** the Settings window's edges under the language */
export async function languageEdge(ctx, code) {
  const { guest } = ctx;
  const did = await ensure(ctx, { language: code });
  const w = await openWindow(guest, 'tmidiform');
  try {
    await sleep(1500);
    const e = edgeReadings(await picture(guest), w.rect);
    return await result(ctx, e.sharp, [...did.map((d) => `set ${d}`), `${w.cls} ${JSON.stringify(roundRect(w.rect))}; ${edgeNote(e)}`], { edgeWidth: e.width, measuredSides: e.measured, sides: e.sides, language: code });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** the top bar at the ratio: its picture is the golden of the ratio (dpr.tsv, keyed by the ratio); the ratio is set by the page's emulation and put back */
export async function dprTopbar(ctx, value) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  try {
    const got = await setDpr(ctx, value);
    const dpr = dprKey(got);
    const j = judgeGolden('dpr', `topbar@dpr${dpr}`, await picture(guest), await topbarRect(guest));
    const set = Math.abs(got - value) < 0.01;
    return await result(ctx, set && GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), `ratio ${dpr} (asked ${value})`, judgedNote(j)], { ...j, dpr, asked: value });
  } finally {
    await restoreDpr(ctx);
  }
}

/** the Settings window's edges at the ratio (the edge's frame is 1 or 2 device px at every ratio) */
export async function dprEdge(ctx, value) {
  const { guest } = ctx;
  await ensure(ctx);
  try {
    const got = await setDpr(ctx, value);
    const w = await openWindow(guest, 'tmidiform');
    try {
      await sleep(1500);
      const e = edgeReadings(await picture(guest), w.rect);
      return await result(ctx, e.sharp && Math.abs(got - value) < 0.01, [`ratio ${dprKey(got)} (asked ${value})`, `${w.cls} ${JSON.stringify(roundRect(w.rect))}; ${edgeNote(e)}`], { edgeWidth: e.width, measuredSides: e.measured, sides: e.sides, dpr: dprKey(got) });
    } finally {
      await closeNew(guest, w.base);
    }
  } finally {
    await restoreDpr(ctx);
  }
}

/** the picture at the start: the whole picture has no all-black tile */
export async function blackInitial(ctx) {
  const did = await ensure(ctx);
  const r = await blackReport(ctx.guest);
  return result(ctx, r.black === 0, [...did.map((d) => `set ${d}`), blackNote(r)], { black: r.black, browserBlack: r.inBrowser, bbox: r.bbox, browser: r.browser });
}

/** the whole picture at the start: its golden (start.tsv) */
export async function pictureStart(ctx) {
  const did = await ensure(ctx);
  const pic = await picture(ctx.guest);
  const dpr = await currentDpr(ctx.guest);
  const j = judgeGolden('start', `picture@dpr${dpr}`, pic, { x: 0, y: 0, w: pic.w, h: pic.h });
  return result(ctx, GOLDEN_OK.includes(j.status), [...did.map((d) => `set ${d}`), judgedNote(j)], { ...j, dpr });
}

/** the menu bar's text at the start */
export async function textInitial(ctx) {
  const did = await ensure(ctx);
  return textCheck(ctx, did.map((d) => `set ${d}`), {});
}

/** the cursor over the window's edges: some point of its border shows the picture cursor (image-set), and its declared resolution is the ratio */
export async function cursorWindowEdge(ctx) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  const w = await openWindow(guest, 'tmidiform');
  try {
    await sleep(1000);
    const r = w.rect, dpr = Number(await currentDpr(guest));
    const pts = [[r.x + 1, r.y + r.h / 2], [r.x + r.w - 2, r.y + r.h / 2], [r.x + r.w / 2, r.y + r.h - 2], [r.x + r.w / 2, r.y + 1], [r.x + 1, r.y + 1], [r.x + r.w - 2, r.y + r.h - 2]].map(([x, y]) => [Math.round(x), Math.round(y)]);
    const seen = [];
    for (const [x, y] of pts) {
      const css = await cursorAt(guest, x, y);
      const cr = cursorReading(css);
      seen.push({ at: [x, y], isImageSet: cr.isImageSet, scales: cr.scales });
      if (cr.isImageSet) return await result(ctx, cr.sameRatio(dpr), [...did.map((d) => `set ${d}`), `picture cursor at ${x},${y}: declared ${JSON.stringify(cr.scales)} at ratio ${dpr}`], { dpr, cursor: seen });
    }
    return await result(ctx, false, [...did.map((d) => `set ${d}`), `no edge point shows a picture cursor (${JSON.stringify(seen.map((s) => s.isImageSet))})`], { dpr, cursor: seen });
  } finally {
    await closeNew(guest, w.base);
  }
}

/** the cursor over a text field (the Project page's edit): the picture cursor of the ratio */
export async function cursorTextField(ctx) {
  const { guest } = ctx;
  const did = await ensure(ctx);
  const page = await openSettingsPage(guest, 'Project');
  try {
    await sleep(800);
    const t = await windowTable(guest);
    const edit = subtree(t, page.page.lid).find((c) => c.cls === 'TQuickEdit' && c.w > 0 && c.h > 0);
    if (!edit) return await result(ctx, false, [...did.map((d) => `set ${d}`), 'no text field (TQuickEdit) on the Project page'], {});
    const a = absoluteRect(t, edit.lid);
    const dpr = Number(await currentDpr(guest));
    const css = await cursorAt(guest, Math.round(a.x + a.w / 2), Math.round(a.y + a.h / 2));
    const cr = cursorReading(css);
    return await result(ctx, cr.isImageSet && cr.sameRatio(dpr), [...did.map((d) => `set ${d}`), `text field ${JSON.stringify(roundRect(a))}: cursor ${JSON.stringify(css.slice(0, 60))}; image-set ${cr.isImageSet}, declared ${JSON.stringify(cr.scales)} at ratio ${dpr}`], { dpr, cursor: cr });
  } finally {
    await closeSettings(guest);
  }
}

/** the menu bar's text at the ratio: the ratio is set by the page's emulation and put back */
export async function textDpr(ctx, value) {
  await ensure(ctx);
  try {
    const got = await setDpr(ctx, value);
    return await textCheck(ctx, [`ratio ${dprKey(got)} (asked ${value})`], { dpr: dprKey(got), asked: value });
  } finally {
    await restoreDpr(ctx);
  }
}

/** the whole picture at the ratio: no all-black 64 x 64 tile outside the Browser panel */
export async function blackDpr(ctx, value) {
  await ensure(ctx);
  try {
    const got = await setDpr(ctx, value);
    const r = await blackReport(ctx.guest);
    return await result(ctx, r.black === 0 && Math.abs(got - value) < 0.01, [`ratio ${dprKey(got)} (asked ${value})`, blackNote(r)], { black: r.black, browserBlack: r.inBrowser, bbox: r.bbox, dpr: dprKey(got) });
  } finally {
    await restoreDpr(ctx);
  }
}

/** the menu bar's text under the language */
export async function textLanguage(ctx, code) {
  const did = await ensure(ctx, { language: code });
  return textCheck(ctx, did.map((d) => `set ${d}`), { language: code });
}
