// The shared steps of the scenarios and of the inventory (Issue #1485). A person's actions are real mouse and key events (lib/guest.mjs); the reads are the page's test aids:
// the window table (deskDumpBase64: the windows, top-level and child), the picture (deskDumpBase64(true)), the guest's memory text (guestText) and the audio's numbers (audio.detail()).
// No text is read from the picture (no OCR): the names are the window table's texts, the guest's memory strings, or the item's place (`#n`).
// Playwright is not used here, so the tests and --list can import this file.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDump, absoluteRect, childrenOfClass } from './dump.mjs';
import { decodePicture, regionHash, blackTiles } from './picture.mjs';
import { clearProblems, closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from './flow.mjs';
import { armExpression, takeExpression, pcmReport } from '../../p3-audio-probe/waveform.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
export const FEATURES_DIR = path.resolve(here, '..', 'features');
export const POPUP = 'TQuickPopupMenuWindow';
export const MENU_BAR = 'TNewMenu';
export const MAIN_FORM = 'TFruityLoopsMainForm';
export const GDPR = 'TGDPRModule';
export const NEW_PROJECT_FORM = 'TNewProjForm';
export const SHOP_FORM = 'TInAppShopForm';
/** the menu bar's labels and their places as fractions of the bar's width (x) and height (y), the same as tools/crash-hunt (its LABELS) */
export const MENU_LABELS = [['File', 0.051], ['Edit', 0.147], ['Add', 0.235], ['Patterns', 0.375], ['View', 0.525], ['Options', 0.666], ['Tools', 0.819], ['Help', 0.94]];
export const LABEL_FY = 0.45;

export const slug = (s) => String(s).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'x';

/** the window table of the page as parseDump gives it (the table is asked again while the picture is being shown, as Guest.windows does) */
export async function windowTable(guest) {
  const t0 = Date.now();
  for (;;) {
    try {
      const text = await guest.ev('window.__app.deskDumpBase64(false)').then((d) => d.windows);
      return parseDump(text);
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
      guest.dumpRetries++;
      await sleep(100);
    }
  }
}

/** the picture as the page keeps it now (no compose): { w, h, stride, bytes } (lib/picture.mjs) */
export async function picture(guest) {
  const d = await guest.ev('window.__app.deskDumpBase64(true)');
  return decodePicture({ base64: d.base64, width: d.width, height: d.height });
}

/** the SHA-256 of a part of the picture (a guest px rectangle): the same pixels, the same hash (picture changes are seen by the hash) */
export async function pixelHashRegion(guest, rect) {
  return regionHash(await picture(guest), rect);
}

/** the number of black tiles (64 x 64 guest px, all pixels 0) over the whole picture: 0 when nothing is drawn black */
export async function noBlackRegions(guest) {
  return blackTiles(await picture(guest), 64);
}

/** the guest's memory text for the markers (the page's guestText: ASCII and UTF-16LE). `lines` is the best text (the most lines); `near` lists the runs near the places when radius > 0 */
export async function guestText(guest, markers, radius = 0) {
  const t = await guest.ev(`window.__app.guestText(${JSON.stringify(markers)}, ${Number(radius)})`);
  return { memBytes: t.memBytes, lines: t.best?.lines ?? [], best: t.best ? { marker: t.best.marker, enc: t.best.enc, offset: t.best.offset } : null, near: t.near ?? [] };
}

/** waits (on the window table, every 300 ms) until a top-level window of the class is there; returns it, or null at the limit */
export async function waitWindow(guest, cls, ms) {
  const t0 = Date.now();
  for (;;) {
    const w = (await guest.windows()).find((x) => x.cls === cls);
    if (w) return w;
    if (Date.now() - t0 >= ms) return null;
    await sleep(300);
  }
}

/** waits until the predicate on the window table holds, or the limit; returns the value of the predicate, or null */
export async function waitTable(guest, pred, ms) {
  const t0 = Date.now();
  for (;;) {
    const v = await pred(await windowTable(guest)).catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) return null;
    await sleep(300);
  }
}

/** the lids of the top-level windows now */
export async function openLids(guest) {
  return new Set((await guest.windows()).map((w) => w.lid));
}

/** the new drop-downs (TQuickPopupMenuWindow) and the other new top-level windows since the baseline */
export async function newWindows(guest, base) {
  const all = await guest.windows();
  const fresh = all.filter((w) => !base.has(w.lid));
  return { pops: fresh.filter((w) => w.cls === POPUP).sort((a, b) => a.lid - b.lid), others: fresh.filter((w) => w.cls !== POPUP), all };
}

/**
 * Escape until no top-level window is new since the baseline (8 rounds at most); returns the windows still open that are new.
 * After each Escape the close is awaited (waitFewer), not a fixed wait: the Plugin list closes later than 400 ms, and an Escape sent while it is still closing
 * reaches the main form with nothing open, which hides the Channel rack (Issue #1510).
 */
export async function closeNew(guest, base) {
  await guest.focusDesktop();
  for (let i = 0; i < 8; i++) {
    const { pops, others } = await newWindows(guest, base);
    const n = pops.length + others.length;
    if (n === 0) return [];
    await guest.escape();
    await waitFewer(guest, base, n, 5000);
  }
  const { pops, others } = await newWindows(guest, base);
  return [...pops, ...others];
}

/** waits (on the window table, every 150 ms) until fewer than `n` windows are new since the baseline, or `ms` pass; true when they are fewer */
export async function waitFewer(guest, base, n, ms) {
  const t0 = Date.now();
  for (;;) {
    const { pops, others } = await newWindows(guest, base);
    if (pops.length + others.length < n) return true;
    if (Date.now() - t0 >= ms) return false;
    await sleep(150);
  }
}

/** the menu bar's rectangle (guest px) of the table (its TNewMenu child), or null */
export function menuBarRect(table) {
  const bar = table.children.find((c) => c.cls === MENU_BAR && c.vis);
  return bar ? absoluteRect(table, bar.lid) : null;
}

/** the place of a menu label in guest px (the bar's fraction of its width and height) */
export function menuLabelPlace(bar, label) {
  const f = MENU_LABELS.find(([n]) => n === label);
  if (!f) throw new Error(`no menu label ${label} (${MENU_LABELS.map(([n]) => n).join(', ')})`);
  return { x: bar.x + f[1] * bar.w, y: bar.y + LABEL_FY * bar.h };
}

/** a click on a menu label (the bar is read from the window table); the drop-down opens on the press */
export async function openMenu(guest, label) {
  const table = await windowTable(guest);
  const bar = menuBarRect(table);
  if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
  const p = menuLabelPlace(bar, label);
  await guest.click(p.x, p.y);
  await sleep(1000);
}

/** the path of a menu item in menus.tsv: `menus/item/<label slug>-<j>[-<k>...]` (j: the item's number in its drop-down, k: in a submenu) */
export function itemIndices(id) {
  const m = /^menus\/item\/[a-z]+((?:-\d+)+)$/.exec(id);
  return m ? m[1].slice(1).split('-').map(Number) : null;
}

/**
 * chooses a menu item by its path: menuPath(guest, ['File', 'Open']) or ['File', '#9'] (the item's number) or ['File', '#9', '#2'] (a submenu's item).
 * The names are the menus.tsv's (a name read from the guest's memory) or `#n`. Each step is Down presses to the item and Enter (the Enter of an item that has a submenu
 * opens it); the last Enter chooses the item (its action runs). Returns the item's ID (menus/item/...).
 */
export async function menuPath(guest, names, { rows = readMenuRows() } = {}) {
  const [label, ...rest] = names;
  if (!MENU_LABELS.some(([n]) => n === label)) throw new Error(`menuPath: ${label} is not a menu label`);
  if (rest.length === 0) throw new Error('menuPath: a path needs at least one item');
  const row = rows.find((r) => r.name === names.join(' > '));
  const indices = row ? itemIndices(row.id) : (rest.every((p) => /^#\d+$/.test(p)) ? rest.map((p) => Number(p.slice(1))) : null);
  if (!indices) throw new Error(`menuPath: no item ${names.join(' > ')} in menus.tsv (use #n for an item's number)`);
  await openMenu(guest, label);
  for (let d = 0; d < indices.length; d++) {
    for (let k = 0; k < indices[d]; k++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(120); }
    await guest.key('Enter', 'Enter', 13);
    await sleep(d < indices.length - 1 ? 900 : 300);
  }
  return `menus/item/${slug(label)}-${indices.join('-')}`;
}

/** the menus.tsv rows of the generated features (the inventory's menu items), read from the features directory; [] when there is none */
export function readMenuRows(dir = path.join(FEATURES_DIR, 'generated', 'menus.tsv')) {
  if (!fs.existsSync(dir)) return [];
  const lines = fs.readFileSync(dir, 'utf8').split(/\r?\n/).filter((l) => l.length > 0 && !l.startsWith('#'));
  if (lines.length < 2) return [];
  const head = lines[0].split('\t');
  return lines.slice(1).map((l) => Object.fromEntries(l.split('\t').map((c, i) => [head[i], c])));
}

/** the state of the page, the window table's main form and the Welcome/usage boxes: waits for idle, closes the start boxes and opens the default project */
export async function startProject(guest) {
  const first = await waitStable(guest, 180000);
  if (first.state !== 'engine-idle') return { opened: false, state: first.state, title: first.title, waitedMs: first.waitedMs };
  const t = await windowTable(guest);
  if (childrenOfClass(t, [GDPR]).some((c) => c.vis)) await noThanks(guest);
  if (t.top.some((w) => w.cls === WELCOME)) await closeWelcome(guest);
  const end = await waitProjectOpen(guest, first.title, 120000);
  await dismissProblems(guest);
  return { opened: end.opened, state: end.state, title: end.title, waitedMs: end.waitedMs };
}

/**
 * File > New by its key (Ctrl+N): an empty project. With `newProject` (the File area's scenarios: run.mjs) the New project form (TNewProjForm, titled "Save as": the
 * name and the place) takes Enter (its OK), which makes the new project (the title becomes Project_1.flp). The trial window that comes after it (TInAppShopForm) is
 * closed with Escape. Any other box the new project brings up is closed with Escape (its own windows only: the base is the windows at the start).
 * Without `newProject` the form is cancelled with Escape and the project stays: the trial window's web face stays over the desktop after its close (Issue #1531), and
 * it takes the clicks of the desktop's centre for the rest of the page, so the other areas do not make a new project.
 */
export async function resetProject(guest, { newProject = true, settleMs = 2000 } = {}) {
  await clearProblems(guest, { settleMs });
  // a drop-down left by the last scenario takes the Enter of the New form, so it is closed first
  for (let i = 0; i < 4 && (await guest.windows()).some((x) => x.cls === POPUP); i++) {
    await guest.escape();
    await sleep(400);
  }
  const base = await openLids(guest);
  await guest.focusDesktop();
  await guest.ctrl('KeyN', 'n', 78);
  const form = await waitWindow(guest, NEW_PROJECT_FORM, 8000);
  if (form && newProject) {
    await guest.key('Enter', 'Enter', 13);
    await sleep(1500);
    await waitWindow(guest, SHOP_FORM, 4000);
  }
  const left = await closeNew(guest, base);
  await sleep(500);
  return { form: form !== null, newProject, left: left.map((w) => ({ cls: w.cls, title: w.title })) };
}

/** the page reloaded: the start is run again (the usage box, the Welcome, the default project); returns the title before and after */
export async function persist(page, guest) {
  const before = await guest.state();
  await page.reload({ waitUntil: 'load', timeout: 120000 });
  const after = await startProject(guest);
  return { titleBefore: before?.title ?? null, titleAfter: after.title ?? null, opened: after.opened, state: after.state };
}

/** the audio's numbers over `ms`: the page's audio.detail() before and after, and a capture of the output (pcmEligible etc. as tools/p3-audio-probe reports them) */
export async function audioProbe(guest, ms) {
  const d0 = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
  if (d0 === null) return { available: false, before: null, after: null };
  const index = await guest.ev(armExpression());
  await sleep(ms);
  let summary = null;
  const t0 = Date.now();
  while (summary === null && Date.now() - t0 < ms + 5000) {
    summary = await guest.ev(takeExpression(index)).catch(() => null);
    if (summary === null) await sleep(250);
  }
  const d1 = await guest.ev('window.__app.audio ? window.__app.audio.detail() : null');
  return { available: true, before: d0, after: d1, captured: summary !== null, ...pcmReport(summary) };
}
