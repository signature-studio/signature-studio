#!/usr/bin/env node
// The inventory of the features (Issue #1485): what the running FL and the repository hold, written as TSV (one file per area) and summary.json.
// Usage: node tools/e2e/inventory.mjs --out <DIR> [--budget 20m]     (needs tools/e2e/node_modules (npm ci), Chromium, and the bundle lift/out-app/bundle/{raw,br}/fl-Oz)
// The FL side runs in the production delivery (lib/delivery.mjs), in one page: the start (lib/common.mjs startProject), then, read from the window table (lib/dump.mjs):
//   menus.tsv    the 8 labels of the menu bar (their places are the bar's fractions: MENU_LABELS) and each drop-down's items: the count is the number of Down presses until
//                the drop-down's picture repeats (its rectangle's hash); an item is `File > #n` (its number: the names are not read from the picture, no OCR). The memory names
//                (the page's guestText) are not tied to a drop-down's record here, so every item is named by its number (nameSource index).
//   windows.tsv  the top-level windows that the keys F5 to F12 and Alt+F8 open, and the drop-downs of the right-click menus of the Channel rack, Mixer, Playlist and Browser.
//   topbar.tsv   the top bar's parts: the TWPControl, TQuickControlBar, TQuickEditToolBar, TQuickEditToolbarPanel and TNewMenu windows under the main form (their texts are the names).
//   settings.tsv the F10 settings window's sheets (pages, by their texts), and the sections, lists, selectors and controls under them (by their classes).
// The repository side (lib/inventory.mjs): plugins.tsv, file.tsv, resources.tsv, page.tsv, delivery.tsv. summary.json: { total, perArea, unnamed, fl }.
// The inventory's IDs are `<領域>/<種類>/<名前の slug>`, unique over all files. Exit 0 when the files are written and the FL side ran; 1 otherwise (the reason is in stderr and summary.fl).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { launchBrowser, newPage } from './lib/browser.mjs';
import { Guest } from './lib/guest.mjs';
import { bundleObjects, startDelivery } from './lib/delivery.mjs';
import { parseArgs, parseBudget, budgetClock } from './lib/plan.mjs';
import { absoluteRect, subtree } from './lib/dump.mjs';
import { pixelHashRegion, startProject, windowTable, menuBarRect, menuLabelPlace, MENU_LABELS, MENU_BAR, MAIN_FORM, POPUP, newWindows, openLids, closeNew, slug } from './lib/common.mjs';
import { uniqueId, toTsv, pluginRows, fileRows, resourceRows, pageRows, deliveryRows, summarize } from './lib/inventory.mjs';
import { sleep } from './lib/flow.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const appDir = path.join(repo, 'app');
const BUNDLE = 'fl-Oz';
const bundleDir = path.join(repo, 'lift', 'out-app', 'bundle', 'raw', BUNDLE);
const brDir = path.join(repo, 'lift', 'out-app', 'bundle', 'br', BUNDLE);
const MAX_ITEMS = 80;
const DOWN_WAIT_MS = 200;
const OPEN_WAIT_MS = 1000;
const TOP_CLASSES = new Set(['TWPControl', 'TQuickControlBar', 'TQuickEditToolBar', 'TQuickEditToolbarPanel', 'TNewMenu']);
const SHEET = 'TVectorSheet';
const SHEET_CLASSES = new Set([SHEET, 'TVectorPanel', 'TQuickSheetSelector', 'TQuickTabSelector']);
const KEYS = [['F5', 116], ['F6', 117], ['F7', 118], ['F8', 119], ['F9', 120], ['F10', 121], ['F11', 122], ['F12', 123]];
const RIGHT_TARGETS = [['Channel rack', 'TStepSeqForm'], ['Mixer', 'TFXForm'], ['Playlist', 'TEventEditForm'], ['Browser', 'TSampleListForm']];
// the right-click spots inside a window, as offsets from its top-left (guest px): a spot that opens a drop-down is kept; the centre does not open one (measured)
const RIGHT_SPOTS = [[20, 60], [0.5, 60], [30, 100], [0.5, 0.5]];

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);

/** how many items a drop-down has: Down presses until the drop-down's picture repeats (seen[t] is the picture after t presses; the repeat of seen[i] gives the count
 *  n = k - i, and pre when i is 0: the picture before any press is the first item's). The rule and the waits are those of tools/crash-hunt (its item counts are the same) */
export async function countItems(guest, rect) {
  const seen = [await pixelHashRegion(guest, rect)];
  for (let k = 1; k <= MAX_ITEMS; k++) {
    await guest.key('ArrowDown', 'ArrowDown', 40);
    await sleep(DOWN_WAIT_MS);
    const h = await pixelHashRegion(guest, rect);
    const i = seen.indexOf(h);
    if (i < 0) { seen.push(h); continue; }
    return { n: k - i, pre: i === 0 };
  }
  return { n: null, pre: false };
}

/** a short right-button press and release at a guest pixel (the menu of FL opens on the release; tools/crash-hunt uses the same) */
async function rightTap(guest, gx, gy) {
  const c = await guest.pointAt(gx, gy);
  await guest.move(c);
  await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'right', buttons: 2, clickCount: 1, pointerType: 'mouse' });
  await sleep(50);
  await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'right', buttons: 0, clickCount: 1, pointerType: 'mouse' });
  await sleep(100);
}

/** the drop-down rows of the menu bar: one row per label, and one per item (`File > #n`) */
async function menuRows(guest) {
  const table = await windowTable(guest);
  const bar = menuBarRect(table);
  const barWin = table.children.find((c) => c.cls === MENU_BAR && c.vis);
  if (!bar || !barWin) throw new Error('the menu bar (TNewMenu) is not in the window table');
  const rows = [];
  for (const [label, fx] of MENU_LABELS) {
    const p = menuLabelPlace(bar, label);
    const base = await openLids(guest);
    await closeNew(guest, base);
    await guest.click(p.x, p.y);
    await sleep(OPEN_WAIT_MS);
    const { pops: all } = await newWindows(guest, base);
    const pops = all.filter((p) => Math.abs(p.y - (bar.y + bar.h)) <= 4);
    const head = { id: `menus/menu/${slug(label)}`, kind: 'menu', name: label, source: `TNewMenu lid ${barWin.lid} (bar ${bar.x},${bar.y} ${bar.w}x${bar.h})`, how: `label place: bar fraction x ${fx} y 0.45`, nameSource: 'index' };
    if (pops.length === 0) {
      rows.push({ ...head, how: `${head.how}; no drop-down after the press` });
      log(`${label}: no drop-down`);
      continue;
    }
    const pop = pops[pops.length - 1];
    const rect = { x: pop.x, y: pop.y, w: pop.w, h: pop.h };
    const { n, pre } = await countItems(guest, rect);
    rows.push({ ...head, how: `${head.how}; drop-down lid ${pop.lid} at ${pop.x},${pop.y} ${pop.w}x${pop.h}` });
    log(`${label}: ${n} items${pre ? ' (pre)' : ''}`);
    if (n === null) { await closeNew(guest, base); continue; }
    for (let j = 1; j <= n; j++) {
      rows.push({ id: `menus/item/${slug(label)}-${j}`, kind: 'item', name: `${label} > #${j}`, source: `drop-down lid ${pop.lid}`, how: `Down x ${j}${pre ? ' (pre: one less)' : ''} from the label; ${n} items`, nameSource: 'index' });
    }
    await closeNew(guest, base);
  }
  return rows;
}

/** the top-level windows that the keys open (their visibility is switched by the keys: this runs after the other reads) */
async function keyWindowRows(guest) {
  const rows = [];
  const keyList = [...KEYS.map(([name, vk]) => ({ name, code: name, key: name, vk, mod: 0 })), { name: 'Alt+F8', code: 'F8', key: 'F8', vk: 119, mod: 1 }];
  for (const k of keyList) {
    const base = await openLids(guest);
    await guest.focusDesktop();
    await guest.key(k.code, k.key, k.vk, k.mod);
    await sleep(1500);
    const { pops, others } = await newWindows(guest, base);
    for (const w of others) {
      rows.push({ id: `windows/window/${slug(w.cls)}`, kind: 'window', name: w.title || w.cls, source: `key ${k.name} -> lid ${w.lid} class ${w.cls}`, how: `top-level, at ${w.x},${w.y} ${w.w}x${w.h}`, nameSource: w.title ? 'memory' : 'index' });
    }
    for (const p of pops) rows.push({ id: `windows/popup/key-${slug(k.name)}`, kind: 'popup', name: `key ${k.name}`, source: `lid ${p.lid} ${POPUP}`, how: `at ${p.x},${p.y} ${p.w}x${p.h}`, nameSource: 'index' });
    log(`key ${k.name}: ${others.length} windows, ${pops.length} drop-downs`);
    await closeNew(guest, base);
  }
  return rows;
}

/** the drop-downs of the right-click menus of the four windows (their rectangles are read from the table; the windows must be shown) */
async function rightClickRows(guest) {
  const rows = [];
  const table = await windowTable(guest);
  for (const [name, cls] of RIGHT_TARGETS) {
    const w = [...table.top, ...table.children].find((x) => x.cls === cls && x.vis !== false);
    if (!w) { log(`right-click ${name}: no shown ${cls} in the table`); continue; }
    const r = absoluteRect(table, w.lid);
    if (!r) continue;
    let found = null;
    for (const [ox, oy] of RIGHT_SPOTS) {
      const dx = ox <= 1 ? ox * r.w : ox, dy = oy <= 1 ? oy * r.h : oy;
      const base = await openLids(guest);
      await rightTap(guest, r.x + dx, r.y + dy);
      await sleep(800);
      const { pops } = await newWindows(guest, base);
      if (pops.length) { found = { pops, spot: `${Math.round(dx)},${Math.round(dy)}` }; await closeNew(guest, base); break; }
      await closeNew(guest, base);
    }
    if (!found) { log(`right-click ${name}: no drop-down at the spots`); continue; }
    for (const p of found.pops) rows.push({ id: `windows/popup/right-click-${slug(name)}`, kind: 'popup', name: `right-click ${name}`, source: `${cls} lid ${w.lid}, offset ${found.spot} in its rectangle`, how: `drop-down lid ${p.lid} at ${p.x},${p.y} ${p.w}x${p.h}`, nameSource: 'index' });
    log(`right-click ${name}: ${found.pops.length} drop-downs at ${found.spot}`);
  }
  return rows;
}

/** the top bar's parts under the main form */
async function topbarRows(guest) {
  const table = await windowTable(guest);
  const main = table.top.find((w) => w.cls === MAIN_FORM);
  if (!main) throw new Error(`no ${MAIN_FORM} in the window table`);
  const under = subtree(table, main.lid).filter((c) => TOP_CLASSES.has(c.cls));
  return under.map((c) => ({
    id: `topbar/part/${slug(c.text || c.cls)}`, kind: c.cls, name: c.text || `${c.cls} #${c.lid}`, source: `lid ${c.lid} parent lid ${c.parent} ${c.cls}`,
    how: `at ${c.x},${c.y} ${c.w}x${c.h} vis ${c.vis ? 1 : 0}`, nameSource: c.text ? 'memory' : 'index',
  }));
}

/**
 * the F10 settings window (opened by F10 and closed again). Its pages are the tabs of its top selector (TQuickSheetSelector): a click at each place of the
 * selector (12 px apart, the whole width) shows a page, and the page is named by its sheet's text (the window table). The rows: the selector, the pages (in the
 * order of the places), and under each page its sections (TVectorPanel with a text), lists (TQuickStringGrid), selectors and other child windows (by class).
 * A place where no page is shown is not counted (the trial's tab bar has blank places between the named tabs).
 */
async function settingsRows(guest) {
  const base = await openLids(guest);
  await guest.focusDesktop();
  await guest.key('F10', 'F10', 121);
  await sleep(1500);
  let table = await windowTable(guest);
  const midi = table.children.find((c) => c.cls === SHEET && c.text === 'MIDI');
  if (!midi) throw new Error('no MIDI sheet in the window table (the settings window)');
  const all = new Map([...table.top, ...table.children].map((w) => [w.lid, w]));
  let lid = midi.parent, guard = 0;
  while (all.has(lid) && SHEET_CLASSES.has(all.get(lid).cls) && guard++ < 32) lid = all.get(lid).parent;
  const cur = all.get(lid) ?? { lid, cls: '(not in the table)' };
  const sel = subtree(table, cur.lid).find((c) => c.cls === 'TQuickSheetSelector' && c.vis);
  const pages = new Map();
  if (sel) {
    const r = absoluteRect(table, sel.lid);
    for (let x = r.x + 6; x < r.x + r.w - 6; x += 12) {
      await guest.click(x, r.y + r.h / 2);
      await sleep(350);
      table = await windowTable(guest);
      for (const c of subtree(table, cur.lid)) {
        if (c.cls === SHEET && c.vis && c.text && !pages.has(c.text)) pages.set(c.text, { lid: c.lid, x: x - r.x });
      }
    }
  }
  log(`settings: ${pages.size} pages (${[...pages.keys()].join(', ')})`);
  table = await windowTable(guest);
  const rows = [];
  if (sel) rows.push({ id: 'settings/selector/page-tabs', kind: 'selector', name: `TQuickSheetSelector #${sel.lid}`, source: `lid ${sel.lid} TQuickSheetSelector under lid ${cur.lid}`, how: `at ${sel.x},${sel.y} ${sel.w}x${sel.h}; the pages are the places clicked along it`, nameSource: 'index' });
  for (const [text, p] of pages) {
    rows.push({ id: `settings/page/${slug(text)}`, kind: 'page', name: text, source: `lid ${p.lid} TVectorSheet under lid ${cur.lid}`, how: `tab at x ${Math.round(p.x)} of the selector (guest px from its left)`, nameSource: 'memory' });
    const seq = new Map();
    for (const c of subtree(table, p.lid)) {
      if (c.cls === SHEET || (c.cls === 'TVectorPanel' && !c.text)) continue;
      const kind = c.cls === 'TVectorPanel' ? 'section' : c.cls === 'TQuickStringGrid' ? 'list' : (c.cls === 'TQuickSheetSelector' || c.cls === 'TQuickTabSelector') ? 'selector' : 'control';
      const key = `${kind}/${c.cls}`;
      const k = (seq.get(key) ?? 0) + 1;
      seq.set(key, k);
      const name = c.text || `${c.cls} #${k} in ${text}`;
      rows.push({ id: `settings/${kind}/${c.text ? slug(c.text) : `${slug(text)}-${slug(c.cls)}-${k}`}`, kind, name, source: `lid ${c.lid} ${c.cls}`, how: `under the page lid ${p.lid} (${text}), at ${c.x},${c.y} ${c.w}x${c.h} vis ${c.vis ? 1 : 0}`, nameSource: c.text ? 'memory' : 'index' });
    }
  }
  await closeNew(guest, base);
  return rows;
}

function writeAll(out, files) {
  fs.mkdirSync(out, { recursive: true });
  const seen = new Map();
  for (const rows of Object.values(files)) for (const r of rows) r.id = uniqueId(r.id, seen);
  for (const [name, rows] of Object.entries(files)) fs.writeFileSync(path.join(out, `${name}.tsv`), toTsv(rows));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.out) throw new Error('--out DIR is needed');
  const out = path.resolve(args.out);
  const budget = budgetClock(args.budget === undefined ? null : parseBudget(args.budget));
  const exists = (p) => fs.existsSync(path.join(repo, p));
  const read = (p) => fs.readFileSync(path.join(repo, p), 'utf8');
  const manifest = JSON.parse(fs.readFileSync(path.join(bundleDir, 'manifest.json'), 'utf8'));
  const repoRows = {
    plugins: pluginRows({ classes: read('inputs/plugin-classes.tsv'), external: read('inputs/external-plugins.tsv') }),
    delivery: deliveryRows({ manifest }),
    page: pageRows({ exists }),
    resources: resourceRows({ installer: read('inputs/fl-installer-files.tsv') }),
  };
  const fl = { ok: false, error: null, menus: 0, windows: 0, topbar: 0, settings: 0 };
  const flRows = { menus: [], windows: [], topbar: [], settings: [] };
  const workDir = fs.mkdtempSync(path.join(repo, '.tmp-e2e-inv-'));
  const wranglerDirs = [path.join(appDir, '.wrangler'), path.join(repo, '.wrangler')];
  const hadWrangler = wranglerDirs.map((d) => fs.existsSync(d));
  let browser = null, delivery = null;
  try {
    const objects = bundleObjects(bundleDir, brDir, BUNDLE);
    delivery = await startDelivery({ appDir, workDir, objects, log });
    browser = await launchBrowser('raf');
    const pg = await newPage(browser, { cssWidth: 3840, cssHeight: 2160, dpr: 1, origin: delivery.origin, token: delivery.newToken() });
    await pg.page.goto(`${delivery.origin}/`, { waitUntil: 'load', timeout: 120000 });
    const guest = new Guest(pg.page, pg.cdp, delivery.origin);
    const start = await startProject(guest);
    if (!start.opened) throw new Error(`the default project did not open (state ${start.state})`);
    log('the default project is open');
    if (!budget.expired()) { flRows.menus = await menuRows(guest); }
    if (!budget.expired()) { flRows.topbar = await topbarRows(guest); }
    // the right-click reads come before the settings window: its clicks and its Escape leave the Channel rack and the Playlist out of the table
    const rightRows = budget.expired() ? [] : await rightClickRows(guest);
    if (!budget.expired()) { flRows.settings = await settingsRows(guest); }
    if (!budget.expired()) { flRows.windows = [...rightRows, ...(await keyWindowRows(guest))]; }
    fl.menus = flRows.menus.length; fl.windows = flRows.windows.length; fl.topbar = flRows.topbar.length; fl.settings = flRows.settings.length;
    fl.ok = true;
    await pg.context.close().catch(() => {});
  } catch (e) {
    fl.error = String(e?.stack ?? e).slice(0, 800);
    log(`the FL side: ${fl.error}`);
  } finally {
    if (browser) await browser.close().catch(() => {});
    if (delivery) await delivery.close().catch(() => {});
    fs.rmSync(workDir, { recursive: true, force: true });
    wranglerDirs.forEach((d, i) => { if (!hadWrangler[i]) fs.rmSync(d, { recursive: true, force: true }); });
  }
  const menuItems = flRows.menus;
  const files = {
    menus: flRows.menus,
    windows: flRows.windows,
    topbar: flRows.topbar,
    settings: flRows.settings,
    plugins: repoRows.plugins,
    file: fileRows({ installer: read('inputs/fl-installer-files.tsv'), menuRows: menuItems }),
    resources: repoRows.resources,
    page: repoRows.page,
    delivery: repoRows.delivery,
  };
  writeAll(out, files);
  const sum = summarize(files);
  const summary = { ...sum, fl, perFile: Object.fromEntries(Object.entries(files).map(([k, v]) => [k, v.length])) };
  fs.writeFileSync(path.join(out, 'summary.json'), `${JSON.stringify(summary, null, 2)}\n`);
  log(`inventory: ${sum.total} features (${JSON.stringify(sum.perArea)}); FL side ${fl.ok ? 'ok' : 'NOT run'}`);
  return fl.ok ? 0 : 1;
}

const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  process.exitCode = await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    return 1;
  });
}
