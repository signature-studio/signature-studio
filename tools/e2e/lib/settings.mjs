// The settings window (F10, the TMIDIForm window "Settings - ...") as the scenarios of the settings area use it (Issue #1492).
// A person opens it with F10, clicks its tabs (the selector) and the controls (real mouse events, lib/guest.mjs). The places are read from the window table
// (absolute rectangles, guest px), so no lid or offset is fixed in advance. The values are read as: the window table (the sheets, the titles), the picture (the
// SHA-256 of a rectangle), the engine's log lines (the theme file the engine opens), the guest's memory text (Debug, Info). Nothing is read by OCR.
import { absoluteRect, subtree, rootOf } from './dump.mjs';
import { windowTable, openLids, newWindows, closeNew, pixelHashRegion, guestText, waitTable, persist, picture } from './common.mjs';
import { regionHash } from './picture.mjs';
import { sleep } from './flow.mjs';

export const SETTINGS_CLS = 'TMIDIForm';
export const SELECTOR_CLS = 'TQuickSheetSelector';
/** the x offsets (guest px from the selector's left) of the tabs, as the inventory measured them (features/generated/settings.tsv) */
export const TAB_X = { MIDI: 6, Audio: 54, General: 114, File: 186, Theme: 234, Project: 306, Info: 378, Debug: 438, Account: 498, About: 570 };
export const MIDI_TITLE = 'Settings - MIDI input / output devices';

/** the settings window's top-level entry of a table (its lid is the settings' own), or null */
export function settingsWin(t) {
  return t.top.find((w) => w.cls === SETTINGS_CLS) ?? null;
}

/** the selector inside the settings window (visible), or null */
function selectorIn(t, lid) {
  return t.children.find((c) => c.cls === SELECTOR_CLS && c.vis && rootOf(t, c.lid)?.lid === lid) ?? null;
}

/** the visible sheets of the settings window: [{ lid, text }] */
export function visibleSheets(t, lid) {
  return t.children.filter((c) => c.cls === 'TVectorSheet' && c.vis && rootOf(t, c.lid)?.lid === lid).map((c) => ({ lid: c.lid, text: c.text }));
}

/** the visible sheet with the name (the page), or null */
export function sheetNamed(t, lid, name) {
  return visibleSheets(t, lid).find((s) => s.text === name) ?? null;
}

/** the children (any depth) of a sheet that are panels, with their titles: [{ lid, text, rect }] */
export function panelsOf(t, sheetLid) {
  return subtree(t, sheetLid).filter((c) => c.cls === 'TVectorPanel').map((c) => ({ lid: c.lid, text: c.text, rect: absoluteRect(t, c.lid) }));
}

/** a panel of the sheet by its title (the titles are the window table's; a regex when the title has `?` marks for non-ASCII letters) */
export function panelNamed(t, sheetLid, title) {
  const re = title instanceof RegExp ? title : null;
  return panelsOf(t, sheetLid).find((p) => (re ? re.test(p.text) : p.text === title)) ?? null;
}

/** the first child control of a class under a sheet, or of a panel (the grids, the memo) */
export function controlsOf(t, lid, cls) {
  return subtree(t, lid).filter((c) => c.cls === cls && c.vis).map((c) => ({ lid: c.lid, rect: absoluteRect(t, c.lid), text: c.text }));
}

/** the settings window's rectangle (guest px) */
export function rectOf(w) {
  return { x: w.x, y: w.y, w: w.w, h: w.h };
}

/** the picture hash of the settings window (guest px rectangle) */
export const hashWin = (guest, w) => pixelHashRegion(guest, rectOf(w));

/** the settings window's session: F10 opens it; close() removes it (and its drop-downs) so the next scenario starts with nothing open */
export async function openSettings(guest) {
  const base = await openLids(guest);
  await guest.focusDesktop();
  await guest.key('F10', 'F10', 121);
  const w = await settingsOpened(guest);
  const base2 = await openLids(guest);
  return { base, base2, w };
}

/** waits (up to 12 s) until the settings window and its selector are in the table; returns the settings window entry or null */
export async function settingsOpened(guest, ms = 12000) {
  return waitTable(guest, async (t) => {
    const w = settingsWin(t);
    return w && selectorIn(t, w.lid) ? w : null;
  }, ms);
}

/** the tab of a page is clicked (the selector's tab place); waits until its sheet is the visible one; returns the sheet's lid or null */
export async function goTab(guest, name) {
  const t0 = await windowTable(guest);
  const w = settingsWin(t0);
  const sel = w ? selectorIn(t0, w.lid) : null;
  if (!sel) return null;
  const r = absoluteRect(t0, sel.lid);
  await guest.click(r.x + TAB_X[name] + 8, r.y + r.h / 2);
  const sheet = await waitTable(guest, async (t) => {
    const ww = settingsWin(t);
    return ww ? sheetNamed(t, ww.lid, name) : null;
  }, 8000);
  await sleep(400);
  return sheet ? sheet.lid : null;
}

/** the current settings window entry and its visible page names (read now) */
export async function settingsNow(guest) {
  const t = await windowTable(guest);
  const w = settingsWin(t);
  return { t, w, pages: w ? visibleSheets(t, w.lid).map((s) => s.text) : [] };
}

/**
 * runs fn with an open settings window: { guest, w, base, base2, s.reopen(), s.now() }. The window is closed at the end (and its drop-downs).
 * reopen() reloads the page (lib/common.mjs persist) and opens the settings window again; it returns the new settings window entry.
 */
export async function withSettings(ctx, fn, { page = null } = {}) {
  const { guest } = ctx;
  const recovered = await ensureAlive(ctx);
  const s = { base: null, w: null, page };
  const opened = await openSettings(guest);
  s.base = opened.base;
  s.w = opened.w;
  if (!s.w) return { ok: false, notes: ['F10 did not open the settings window (TMIDIForm)'] };
  s.base2 = opened.base2;
  s.reopen = async () => {
    await persist(ctx.page, guest);
    const o = await openSettings(guest);
    s.base = o.base;
    s.base2 = o.base2;
    s.w = o.w;
    return o.w;
  };
  s.popups = async () => (await newWindows(guest, s.base2)).pops;
  s.others = async () => (await newWindows(guest, s.base2)).others;
  s.escape = async () => { await closeNew(guest, s.base2); };
  try {
    const out = await fn(s);
    if (recovered !== null) out.notes = [...(out.notes ?? []), `the engine was ${recovered} before this scenario: the saved registry of the test profile was removed and the page started again`];
    return out;
  } finally {
    await closeNew(guest, s.base).catch(() => []);
  }
}

/** the rectangle of the panel with the title, read now, on the visible sheet of the page; null when either is missing */
export async function panelOnPage(guest, page, title, s) {
  const t = await windowTable(guest);
  const w = settingsWin(t);
  if (!w) return null;
  const sheet = sheetNamed(t, w.lid, page);
  if (!sheet) return null;
  const p = panelNamed(t, sheet.lid, title);
  return p ? { panel: p, sheet, t, w } : null;
}

/** the popup (a TQuickPopupMenuWindow) that appeared since the base, or null */
export function onePopup(pops) {
  return pops.length === 1 ? pops[0] : null;
}

/** the centre of the i-th item (0-based) of a drop-down list of n items, as the popup window's rectangle gives it (guest px) */
export function itemPlace(pop, i, n) {
  return { x: pop.x + pop.w / 2, y: pop.y + (i + 0.5) * (pop.h / n) };
}

/** the picture hash of a rectangle around a point (guest px): used for a check box or a short label */
export function hashAround(guest, x, y, r = 12) {
  return pixelHashRegion(guest, { x: x - r, y: y - r, w: 2 * r, h: 2 * r });
}

/** the engine's log lines since the index (the lines that name a theme file: the engine opens it) */
export async function themeLines(guest, from) {
  return guest.ev(`window.__app.engineLog().slice(${from}).filter((l) => /\\.flstheme/i.test(l))`);
}

/** the guest's memory: the lines that hold the markers (Debug, Info, About texts), as { n, first } */
export async function memoryLines(guest, markers, radius = 0) {
  const g = await guestText(guest, markers, radius);
  return { n: g.lines.length, lines: g.lines, first: g.lines.slice(0, 3) };
}

/** escapes a string for a regular expression */
export const reEscape = (s) => String(s).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** the engine's state word now ('engine-idle' ...) */
export async function engineState(guest) {
  return (await guest.state())?.state ?? null;
}

/** the tab of a page is clicked: the page is the one visible sheet, and the settings title follows it (MIDI keeps its own title) */
export async function pageTabRun(ctx, page) {
  return withSettings(ctx, async (s) => {
    const sheet = await goTab(ctx.guest, page);
    const now = await settingsNow(ctx.guest);
    const title = now.w?.title ?? '';
    const ok = sheet !== null && now.pages.length === 1 && now.pages[0] === page && (page === 'MIDI' || title !== MIDI_TITLE);
    return { ok, engineState: await engineState(ctx.guest), notes: [`tab ${page}: visible ${JSON.stringify(now.pages)}; title "${title}"`], results: { page, visible: now.pages, title } };
  });
}

/** the picture of the settings window is the same after going to the page and back to MIDI, and it differs on the page (the page is drawn) */
export async function pageRoundtripRun(ctx, page) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    await goTab(g, 'MIDI');
    const h0 = await hashWin(g, (await settingsNow(g)).w);
    await goTab(g, page);
    const h1 = await hashWin(g, (await settingsNow(g)).w);
    await goTab(g, 'MIDI');
    const h2 = await hashWin(g, (await settingsNow(g)).w);
    await goTab(g, page);
    const h3 = await hashWin(g, (await settingsNow(g)).w);
    const ok = (page === 'MIDI' ? h1 === h2 : h1 !== h2) && (await engineState(g)) === 'engine-idle';
    return { ok, engineState: await engineState(g), notes: [`${page} drawn apart from MIDI ${h1 !== h2}; stable on the second visit ${h1 === h3} (not gated); MIDI back the same as the first MIDI picture ${h2 === h0} (not gated)`], results: { page, stable: h1 === h3, drawn: h1 !== h2, midiBackSame: h2 === h0 } };
  });
}

/** the settings window's picture changes from the MIDI page to the page (the sheet is drawn in its own place) */
export async function pagePictureRun(ctx, page) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    await goTab(g, 'MIDI');
    const h0 = await hashWin(g, (await settingsNow(g)).w);
    await goTab(g, page);
    const now = await settingsNow(g);
    const h1 = await hashWin(g, now.w);
    const ok = h1 !== h0 && now.pages.length === 1 && now.pages[0] === page;
    return { ok, engineState: await engineState(g), notes: [`${page} picture differs from MIDI: ${h1 !== h0}`], results: { page, visible: now.pages } };
  });
}

/** the panel is listed under its page, with a place inside the page's sheet (or, geometry: not overlapping its sibling panels) */
export async function sectionRun(ctx, page, title, mode) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const t = await windowTable(g);
    const w = settingsWin(t);
    const sh = sheet !== null && w ? sheetNamed(t, w.lid, page) : null;
    const p = sh ? panelNamed(t, sh.lid, title) : null;
    if (!p) return { ok: false, engineState: await engineState(g), notes: [`no panel "${title}" under ${page}`], results: {} };
    const sheetRect = absoluteRect(t, sh.lid);
    const r = p.rect;
    const inside = r.w > 0 && r.h > 0 && r.x >= sheetRect.x - 2 && r.x + r.w <= sheetRect.x + sheetRect.w + 2;
    const me = t.children.find((c) => c.lid === p.lid);
    const overlap = t.children.filter((c) => c.cls === 'TVectorPanel' && c.vis && c.parent === me.parent && c.lid !== me.lid && c.w > 4 && c.h > 4).filter((c) => {
      const ix = Math.min(c.x + c.w, me.x + me.w) - Math.max(c.x, me.x);
      const iy = Math.min(c.y + c.h, me.y + me.h) - Math.max(c.y, me.y);
      return ix > 4 && iy > 4;
    }).map((c) => c.text);
    const offScreen = r.y >= 2160;
    const share = mode === 'geometry' || offScreen ? null : await darkShare(g, r);
    const ok = mode === 'geometry' ? inside && overlap.length === 0 : inside && (offScreen || share < 0.95);
    return { ok, engineState: await engineState(g), notes: [`${title} under ${page}: rect ${r.x},${r.y} ${r.w}x${r.h}; inside ${inside}; overlaps ${JSON.stringify(overlap)}; dark share ${share === null ? 'n/a' : share.toFixed(3)}`], results: { rect: r, inside, overlaps: overlap, darkShare: share } };
  });
}

/** the share of the rectangle's pixels that are black (R, G and B 0) */
export async function darkShare(guest, rect) {
  const pic = await picture(guest);
  const x0 = Math.max(0, Math.floor(rect.x)), y0 = Math.max(0, Math.floor(rect.y));
  const x1 = Math.min(pic.w, Math.ceil(rect.x + rect.w)), y1 = Math.min(pic.h, Math.ceil(rect.y + rect.h));
  let dark = 0, n = 0;
  for (let y = y0; y < y1; y += 2) for (let x = x0; x < x1; x += 2) {
    const i = y * pic.stride + 4 * x;
    n++;
    if (!(pic.bytes[i] | pic.bytes[i + 1] | pic.bytes[i + 2])) dark++;
  }
  return n ? dark / n : 1;
}

/** a grid of the panel (the n-th, by table order) is clicked on its first row: its picture changes (the row is chosen) */
export async function gridRun(ctx, page, title, n) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const t = await windowTable(g);
    const w = settingsWin(t);
    const sh = sheet !== null && w ? sheetNamed(t, w.lid, page) : null;
    const p = sh ? panelNamed(t, sh.lid, title) : null;
    const grids = p ? controlsOf(t, p.lid, 'TQuickStringGrid') : [];
    const grid = grids[n];
    if (!grid) return { ok: false, engineState: await engineState(g), notes: [`no grid ${n} in "${title}" (${grids.length} grids)`], results: {} };
    const h0 = await pixelHashRegion(g, grid.rect);
    await g.click(grid.rect.x + 40, grid.rect.y + 12);
    await sleep(800);
    await goTab(g, page);
    const h1 = await pixelHashRegion(g, grid.rect);
    const pops = await s.popups();
    const ok = h1 !== h0 && pops.length === 0;
    return { ok, engineState: await engineState(g), notes: [`first row of ${title} grid ${n}: picture changed ${h1 !== h0}; popups ${pops.length}`], results: { grids: grids.length, changed: h1 !== h0 } };
  });
}

/** the Language drop-down: opened on the Language panel; the item of the index (0 English, 1 German ... 7 Chinese) is chosen; the panel's picture changes and the choice is kept after a reload */
export async function languageRun(ctx, index) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    await goTab(g, 'General');
    const pn = await panelOnPage(g, 'General', /^Language/, s);
    if (!pn) return { ok: false, engineState: await engineState(g), notes: ['no Language panel on General'], results: {} };
    const L = pn.panel.rect;
    const h0 = await pixelHashRegion(g, L);
    await g.click(L.x + 60, L.y + 30);
    await sleep(900);
    const pop = onePopup(await s.popups());
    if (!pop) return { ok: false, engineState: await engineState(g), notes: ['the Language drop-down did not open (one popup)'], results: {} };
    const at = itemPlace(pop, index, 8);
    await g.click(at.x, at.y);
    await sleep(1200);
    await goTab(g, 'General');
    const open = (await s.popups()).length;
    const h1 = await pixelHashRegion(g, (await panelOnPage(g, 'General', /^Language/, s))?.panel.rect ?? L);
    await s.reopen();
    await goTab(g, 'General');
    const pn2 = await panelOnPage(g, 'General', /^Language/, s);
    const h2 = pn2 ? await pixelHashRegion(g, pn2.panel.rect) : null;
    const state = await engineState(g);
    const ok = open === 0 && h1 !== h0 && h2 !== null && h2 !== h0 && state === 'engine-idle';
    return { ok, engineState: state, notes: [`item ${index}: drop-down closed ${open === 0}; panel changed ${h1 !== h0}; after reload the panel is drawn and differs from English ${h2 !== null && h2 !== h0}`], results: { index, popupsAfter: open, changed: h1 !== h0, afterReloadDiffers: h2 !== h0 } };
  });
}

/** the theme tile (row, col in the Presets grid) is clicked; the engine opens the theme's file (its log names it); the picture changes, and the theme is kept after a reload */
export async function themeRun(ctx, name, row, col) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    await goTab(g, 'Theme');
    const t0 = await windowTable(g);
    const w = settingsWin(t0);
    const sh = w ? sheetNamed(t0, w.lid, 'Theme') : null;
    const p = sh ? panelNamed(t0, sh.lid, 'Presets') : null;
    const grid = p ? controlsOf(t0, p.lid, 'TQuickStringGrid')[0] : null;
    if (!grid) return { ok: false, engineState: await engineState(g), notes: ['no Presets grid'], results: {} };
    const G = grid.rect;
    if (row >= 4) {
      await g.wheel(G.x + G.w / 2, G.y + G.h / 2, 12, 120);
      await sleep(600);
    }
    const y = row >= 4 ? G.y + 386 : G.y + Math.min(59 + 140 * row, 440);
    const x = G.x + 99 + 202 * col;
    const from = await g.ev('window.__app.engineLog().length');
    const h0 = await hashWin(g, w);
    await g.click(x, y);
    await sleep(1500);
    const lines = (await themeLines(g, from)).filter((l) => new RegExp(`[\\\\/]${reEscape(name)}\\.flstheme`, 'i').test(l));
    await goTab(g, 'Theme');
    const w1 = (await settingsNow(g)).w;
    const h1 = await hashWin(g, w1);
    const b1 = await bandHashes(g, rectOf(w1));
    await s.reopen();
    await goTab(g, 'Theme');
    const w2 = (await settingsNow(g)).w;
    const h2 = await hashWin(g, w2);
    const b2 = await bandHashes(g, rectOf(w2));
    const diffBands = differingBands(b1, b2);
    const state = await engineState(g);
    const named = lines.length > 0;
    const afterLines = (await themeLines(g, 0)).filter((l) => new RegExp(`[\\\\/]${reEscape(name)}\\.flstheme`, 'i').test(l));
    const kept = afterLines.length > 0;
    const ok = named && h1 !== h0 && kept && state === 'engine-idle';
    return { ok, engineState: state, notes: [`${name} at row ${row} col ${col}: engine names its file ${named}; picture changed ${h1 !== h0}; engine names the file after reload ${kept}; picture same after reload ${h2 === h1}; bands differing after reload ${JSON.stringify(diffBands)} of ${b1.length}`], results: { theme: name, namedLines: lines.length, changed: h1 !== h0, keptAfterReload: kept, pictureSameAfterReload: h2 === h1, namedAfterReload: afterLines.length, diffBands } };
  });
}

/** a drop-down (a combo) on the page's panel: the click opens exactly one popup; the Escape closes it */
export async function choiceOpenRun(ctx, page, title, dx, dy) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const pn = sheet !== null ? await panelOnPage(g, page, title, s) : null;
    if (!pn) return { ok: false, engineState: await engineState(g), notes: [`no panel "${title}" on ${page}`], results: {} };
    await g.click(pn.panel.rect.x + dx, pn.panel.rect.y + dy);
    await sleep(900);
    const pops = await s.popups();
    const pop = onePopup(pops);
    await s.escape();
    await sleep(300);
    const after = (await s.popups()).length;
    const ok = pop !== null && after === 0;
    return { ok, engineState: await engineState(g), notes: [`${title} at ${dx},${dy}: popups ${pops.length} (${pop ? `${pop.w}x${pop.h}` : 'none'}); closed by Escape ${after === 0}`], results: { popups: pops.length, popup: pop ? [pop.x, pop.y, pop.w, pop.h] : null, closed: after === 0 } };
  });
}

/** a drop-down is opened, an item (of n, from the popup's height: one item is 19.5 guest px at DPR 1) is chosen; the panel's picture changes and is the same after a reload */
export async function choicePickRun(ctx, page, title, dx, dy, pick, { gateKept = false } = {}) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const pn = sheet !== null ? await panelOnPage(g, page, title, s) : null;
    if (!pn) return { ok: false, engineState: await engineState(g), notes: [`no panel "${title}" on ${page}`], results: {} };
    const R = pn.panel.rect;
    const h0 = await pixelHashRegion(g, R);
    await g.click(R.x + dx, R.y + dy);
    await sleep(900);
    const pop = onePopup(await s.popups());
    if (!pop) return { ok: false, engineState: await engineState(g), notes: ['the drop-down did not open'], results: {} };
    const n = Math.max(2, Math.round(pop.h / 19.5));
    const k = Math.min(n - 1, pick);
    const at = itemPlace(pop, k, n);
    await g.click(at.x, at.y);
    await sleep(1000);
    await goTab(g, page);
    const open = (await s.popups()).length;
    const R1 = (await panelOnPage(g, page, title, s))?.panel.rect ?? R;
    const h1 = await pixelHashRegion(g, R1);
    const b1 = await bandHashes(g, R1);
    await s.reopen();
    await goTab(g, page);
    const pn2 = await panelOnPage(g, page, title, s);
    const R2 = pn2 ? pn2.panel.rect : null;
    const h2 = R2 ? await pixelHashRegion(g, R2) : null;
    const b2 = R2 ? await bandHashes(g, R2) : [];
    const diffBands = differingBands(b1, b2);
    const state = await engineState(g);
    const ok = open === 0 && h1 !== h0 && state === 'engine-idle' && (!gateKept || h2 === h1);
    return { ok, engineState: state, notes: [`${title}: ${n} items, item ${k + 1} chosen; popup closed ${open === 0}; changed ${h1 !== h0}; picture after reload same ${h2 === h1} (not gated: the Display values take effect at a restart; bands differing ${JSON.stringify(diffBands)} of ${b1.length})`], results: { items: n, item: k + 1, changed: h1 !== h0, pictureSameAfterReload: h2 === h1, diffBands } };
  });
}

/** a check box (a small square at dx, dy of the panel): one click changes its picture; the value is kept after a reload */
export async function checkRun(ctx, page, title, dx, dy) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const pn = sheet !== null ? await panelOnPage(g, page, title, s) : null;
    if (!pn) return { ok: false, engineState: await engineState(g), notes: [`no panel "${title}" on ${page}`], results: {} };
    const x = pn.panel.rect.x + dx, y = pn.panel.rect.y + dy;
    const h0 = await hashAround(g, x, y);
    await g.click(x, y);
    await sleep(700);
    await goTab(g, page);
    const h1 = await hashAround(g, x, y);
    await s.reopen();
    await goTab(g, page);
    const pn2 = await panelOnPage(g, page, title, s);
    const h2 = pn2 ? await hashAround(g, pn2.panel.rect.x + dx, pn2.panel.rect.y + dy) : null;
    const state = await engineState(g);
    const ok = h1 !== h0 && state === 'engine-idle';
    return { ok, engineState: state, notes: [`${title} box at ${dx},${dy}: changed ${h1 !== h0}; picture after reload same ${h2 === h1} (not gated: unexplained, see the report)`], results: { changed: h1 !== h0, pictureSameAfterReload: h2 === h1 } };
  });
}

/** the page's memory text (the Debug log, the Info text, the About text) is read from the guest's memory: at least `min` lines, the first one matching `first` */
export async function memoryTextRun(ctx, page, markers, min, first, { reload = false } = {}) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    let m = await memoryLines(g, markers, 0);
    let reloaded = null;
    if (reload) {
      await s.reopen();
      await goTab(g, page);
      reloaded = await memoryLines(g, markers, 0);
    }
    const ok = sheet !== null && m.n >= min && (first ? first.test(m.first[0] ?? '') : true) && (reloaded === null || reloaded.n >= min) && (await engineState(g)) === 'engine-idle';
    return { ok, engineState: await engineState(g), notes: [`${page}: ${m.n} memory lines${reloaded ? `, after reload ${reloaded.n}` : ''}; first "${(m.first[0] ?? '').slice(0, 100)}"`], results: { lines: m.n, afterReload: reloaded ? reloaded.n : null, first: (m.first[0] ?? '').slice(0, 160) } };
  });
}

/** the number of the controls of a class under the page's panel (the Info page's fields, the About page's memo and control) */
export async function controlCountRun(ctx, page, cls, min) {
  return withSettings(ctx, async (s) => {
    const g = ctx.guest;
    const sheet = await goTab(g, page);
    const t = await windowTable(g);
    const w = settingsWin(t);
    const sh = sheet !== null && w ? sheetNamed(t, w.lid, page) : null;
    const n = sh ? controlsOf(t, sh.lid, cls).length : 0;
    return { ok: n >= min, engineState: await engineState(g), notes: [`${page}: ${n} ${cls} (at least ${min})`], results: { cls, n } };
  });
}

/** F10 opens the settings window and Escape closes it (the window leaves the table); with `reload`, F10 opens it again after a reload of the page */
export async function f10ToggleRun(ctx, { reload = false } = {}) {
  const g = ctx.guest;
  const first = await openSettings(g);
  const opened1 = first.w !== null;
  await closeNew(g, first.base);
  const closed = (await settingsNow(g)).w === null;
  if (reload) await persist(ctx.page, g);
  const second = await openSettings(g);
  const opened2 = second.w !== null;
  await closeNew(g, second.base).catch(() => []);
  const state = await engineState(g);
  const ok = opened1 && closed && opened2 && state === 'engine-idle';
  return { ok, engineState: state, notes: [`F10 opened ${opened1}; Escape closed ${closed}; F10 again after ${reload ? 'a reload' : 'the close'} opened ${opened2}`], results: { opened1, closed, opened2, reload } };
}

/**
 * the engine is running again: when a scenario before this one stopped it (a saved language that stops the start, Issue-less defects of the start), the
 * saved registry of the test profile (OPFS fl/state/hkcu.log) is removed and the page is loaded again, so that the rest of the run is not lost. Returns the
 * engine's state before the recovery (null when it was running).
 */
export async function ensureAlive(ctx) {
  const st = await engineState(ctx.guest);
  if (st === 'engine-idle') return null;
  await ctx.page.evaluate(async () => {
    const root = await navigator.storage.getDirectory();
    const state = await root.getDirectoryHandle('fl').then((d) => d.getDirectoryHandle('state')).catch(() => null);
    if (state) await state.removeEntry('hkcu.log').catch(() => {});
  });
  await persist(ctx.page, ctx.guest);
  return st;
}

/** the picture hash of each of n horizontal bands of a rectangle (one picture read): the bands that differ show where a change is */
export async function bandHashes(guest, rect, n = 8) {
  const pic = await picture(guest);
  const out = [];
  for (let i = 0; i < n; i++) out.push(regionHash(pic, { x: rect.x, y: rect.y + (i * rect.h) / n, w: rect.w, h: rect.h / n }));
  return out;
}

/** the indices of the bands that differ between two band lists */
export const differingBands = (a, b) => a.map((h, i) => (h === b[i] ? -1 : i)).filter((i) => i >= 0);
