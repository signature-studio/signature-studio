// The file area's steps (Issue #1488): a project file given to the open box (Ctrl+O, File > Open, an Import), the files the engine wrote (Save, Save new version, Export:
// `window.__app.saved`, the page's list of the written files with their names and sizes), the trial window that a Save or an Export brings up (TInAppShopForm), a drop
// of a file onto the desktop, and the engine's log since a mark. The data files are the installer's, in the bundle's host resources (lift/out-app/host/resources);
// the names are the installer's (inputs/fl-installer-files.tsv). The save and the export are checked by what the engine wrote, not by the browser's download.
import path from 'node:path';
import { clearProblems, clickElement, clickRow, MISSING_PLUGINS, openProjectFile, sleep, uploadToWindow, WINDOW, waitListing } from './flow.mjs';
import { closeNew, MAIN_FORM, menuPath, openLids, newWindows, windowTable } from './common.mjs';

export { clickElement };
export const SHOP = 'TInAppShopForm';

/** the title of the main form in the window table (the project's title), or null */
export async function mainTitle(guest) {
  const t = await windowTable(guest).catch(() => null);
  const main = t ? t.top.find((w) => w.cls === MAIN_FORM) : null;
  return main ? main.title : null;
}

/** the missing-plugins form (TMissingPluginsForm, "Problems loading the project") that some demos bring: it takes the keys, so it is answered (flow.mjs clearProblems), and it is watched for `settleMs` more */
export function clearMissingPlugins(guest, settleMs = 3000) {
  return clearProblems(guest, { settleMs });
}
/** the absolute path of an installer file (`Data\Templates\...` as in inputs/fl-installer-files.tsv) in the host resources of the bundle */
export function dataPath(repo, rel) {
  return path.join(repo, 'lift', 'out-app', 'host', 'resources', ...String(rel).split('\\'));
}

/** the engine's log lines from `from` on (the page keeps them; a scenario marks its start with the length) */
export async function logSince(guest, from) {
  return guest.ev(`window.__app.engineLog().slice(${Number(from)})`).catch(() => []);
}

/** the page log lines that say the engine finished writing a file (the page's own log, '#log'): the sizes in them; the list (window.__app.saved) is the other read */
export async function pageWrites(guest) {
  const lines = (await guest.ev("(() => { const lg = document.getElementById('log'); return lg ? lg.textContent.split('\\n') : []; })()")) ?? [];
  return lines.filter((l) => /finished writing a file/.test(l)).map((l) => {
    const m = /\((\d+) bytes\)/.exec(l);
    return { size: m ? Number(m[1]) : 0, line: l.slice(0, 160) };
  });
}

/** the files the engine wrote so far (the page's list): { name, guestPath, size } */
export async function savedFiles(guest) {
  const list = await guest.ev("window.__app && window.__app.saved ? window.__app.saved.map((f) => ({ name: f.name, guestPath: f.guestPath, size: f.size })) : []");
  return list ?? [];
}

/** the engine's Open / Save window is up (the page's file manager in selection mode, Issue #1519): waits for it; returns 'explorer' | null */
export async function waitBox(guest, ms = 40000) {
  const t0 = Date.now();
  let box = null;
  while (!box && Date.now() - t0 < ms) {
    box = await guest.ev(WINDOW);
    if (!box) await sleep(500);
  }
  return box;
}

/**
 * the files (or a folder) given to the engine's Open window: each is uploaded into the window's list; the first `choose` of them (all by default) are chosen (the first is clicked, the others added with Ctrl),
 * and Open is pressed. `gone` is true when the window closed after the answer (the engine took it); false when the window stays (the window refused the choice, e.g. a folder).
 */
export async function giveFiles(guest, cdp, files, { choose = files.length } = {}) {
  const box = await waitBox(guest);
  if (box !== 'explorer') return { box, given: false };
  await waitListing(guest);
  await uploadToWindow(guest, cdp, files);
  for (let i = 0; i < choose; i++) await clickRow(guest, path.basename(files[i]), { ctrl: i > 0 });
  await clickElement(guest, '#fx-ok');
  const t0 = Date.now();
  while ((await guest.ev(WINDOW)) !== null && Date.now() - t0 < 15000) await sleep(500);
  return { box, given: true, gone: (await guest.ev(WINDOW)) === null, uploaded: files.length, chosen: choose };
}

/**
 * a project file given to the open box (Ctrl+O, the file answered in the box). The project is open when the engine is idle and the title has changed or the
 * engine logged the file's name. Returns the facts: `opened` is the judge's.
 */
export async function openDataFile(ctx, abs, ms = 150000) {
  const { guest, cdp, log } = ctx;
  await clearMissingPlugins(guest);
  const before = { title: await mainTitle(guest) };
  const logStart = (await guest.ev('window.__app.engineLog().length').catch(() => 0)) ?? 0;
  let box = null;
  try {
    box = await openProjectFile(guest, cdp, abs, log);
  } catch (e) {
    const windows = (await guest.windows().catch(() => [])).map((w) => `${w.cls}|${w.title}`).slice(0, 10);
    return { box: null, opened: false, state: (await guest.state())?.state ?? null, title: null, titleBefore: before?.title ?? null, logHit: null, boxLeft: null, boxTitle: null, mainForm: null, problemsAnswers: 0, error: `${String(e?.message ?? e).slice(0, 200)}; windows ${JSON.stringify(windows)}` };
  }
  return waitOpened(guest, { box, name: path.basename(abs), before, logStart, ms });
}

/** waits until the project is open (see openDataFile), or the engine stopped, or the limit. The Problems box's answers (problemsAnswers) and what is left of it (boxLeft, boxTitle) and the main form (mainForm) are the facts of the judge */
export async function waitOpened(guest, { box, name, before, logStart, ms }) {
  const t0 = Date.now();
  let hit = null;
  let st = null;
  let answers = 0;
  while (Date.now() - t0 < ms) {
    st = await guest.state().catch(() => null);
    if (st && (st.state === 'error' || st.state === 'exited')) break;
    const lines = await logSince(guest, logStart);
    hit = lines.find((l) => l.includes(name)) ?? hit;
    if (st && st.state === 'engine-idle' && (st.title !== before?.title || hit !== null) && Date.now() - t0 > 2500) break;
    answers += await clearProblems(guest);
    await sleep(500);
  }
  await sleep(1500);
  answers += await clearProblems(guest);
  answers += await clearMissingPlugins(guest);
  const end = (await guest.state().catch(() => null)) ?? { state: 'unreadable', title: null };
  // the title is the main form's (the table's), not the page's: while the Problems box is up the page's title is the box's
  const table = await windowTable(guest).catch(() => null);
  const main = table ? table.top.find((w) => w.cls === MAIN_FORM) : null;
  const problems = table ? table.top.find((w) => w.cls === MISSING_PLUGINS) : null;
  const title = main ? main.title : end.title;
  const boxLeft = table ? !!problems : false;
  const changed = title !== (before?.title ?? null);
  return {
    box, opened: end.state === 'engine-idle' && !boxLeft && (changed || hit !== null), state: end.state, title, titleBefore: before?.title ?? null,
    titleChanged: changed, logHit: hit, boxLeft, boxTitle: problems ? problems.title : null, mainForm: table ? !!main : null, mainTitle: main ? main.title : null,
    pageTitle: end.title ?? null, problemsAnswers: answers, error: null, waitedMs: Date.now() - t0,
  };
}

/** the facts of an opening for a note: the Problems box (its answers, and whether it is left and its title), the main form (its title, or none), and the error when the opening threw */
export function openFacts(r) {
  const box = r.boxLeft === null ? 'box unread' : r.boxLeft ? `box left "${r.boxTitle}"` : 'no box left';
  const main = r.mainForm === null ? 'main form unread' : r.mainForm ? `main form title "${r.mainTitle}"` : 'no main form';
  return `problems answered ${r.problemsAnswers ?? 0}, ${box}, ${main}${r.error ? `; error ${r.error}` : ''}`;
}

/** Ctrl+O, then the files given to the box (several files, or a folder): the facts of what the box did */
export async function openFilesAtOnce(ctx, files, { choose = files.length } = {}) {
  const { guest, cdp } = ctx;
  await clearProblems(guest);
  await clearMissingPlugins(guest);
  const before = { title: await mainTitle(guest) };
  const logStart = (await guest.ev('window.__app.engineLog().length').catch(() => 0)) ?? 0;
  await cdp.send('Page.setInterceptFileChooserDialog', { enabled: true });
  await guest.focusDesktop();
  await guest.ctrl('KeyO', 'o', 79);
  let given = null;
  try {
    given = await giveFiles(guest, cdp, files, { choose });
  } catch (e) {
    return { box: null, given: false, opened: false, state: (await guest.state())?.state ?? null, title: null, error: String(e?.message ?? e).slice(0, 300) };
  }
  if (given.given && given.gone === false) {
    // the window stays: it refused the choice (a folder, or several files for one choice); it is left to the caller to close it
    const st = (await guest.state().catch(() => null)) ?? { state: null };
    return { box: given.box, given: true, gone: false, refused: true, opened: false, state: st.state, title: null, titleBefore: before.title, files: files.length };
  }
  const end = await waitOpened(guest, { box: given.box, name: path.basename(files[0]), before, logStart, ms: 120000 });
  return { ...end, given: given.given, gone: given.gone ?? null, files: files.length, chosen: choose };
}

/** the trial window (TInAppShopForm) that a Save or an Export brings up is closed with Escape when it is there */
async function closeShop(guest) {
  const w = (await guest.windows()).find((x) => x.cls === SHOP);
  if (!w) return false;
  await guest.focusDesktop();
  await guest.escape();
  await sleep(500);
  return true;
}

/**
 * an action that makes the engine write a file (Save, Save new version, Export): the action runs; the trial window is closed when it comes; the scenario waits for
 * the page's list of written files to grow (the render or the save is done); then the windows the action left are closed. Returns the new files and what happened.
 */
export async function writeAndWait(ctx, action, ms = 40000) {
  const { guest } = ctx;
  const base = await openLids(guest);
  const before = await savedFiles(guest);
  let id = null;
  let error = null;
  try {
    id = await action();
  } catch (e) {
    error = String(e?.message ?? e).slice(0, 300);
  }
  const t0 = Date.now();
  let shop = false;
  let fresh = [];
  while (!error && Date.now() - t0 < ms) {
    if (await closeShop(guest)) shop = true;
    fresh = (await savedFiles(guest)).slice(before.length);
    if (fresh.length > 0) break;
    await sleep(500);
  }
  await sleep(800);
  if (await closeShop(guest)) shop = true;
  const left = await closeNew(guest, base);
  return { id, error, shop, saved: fresh, waitedMs: Date.now() - t0, leftOpen: left.length };
}

/** the Save of the project by its key (Ctrl+S) or by the menu item: the trial window closes and the engine's written file is listed */
export async function saveByKey(ctx) {
  const { guest } = ctx;
  return writeAndWait(ctx, async () => {
    await guest.focusDesktop();
    await guest.ctrl('KeyS', 's', 83);
    return 'ctrl-s';
  });
}

/** a drop of a file onto the desktop at a guest pixel: CDP's drag (dragEnter, dragOver, drop) with the file's path */
export async function dropFile(guest, abs, gx, gy) {
  const p = await guest.pointAt(gx, gy);
  const data = { items: [], files: [abs], dragOperationsMask: 1 };
  await guest.send('Input.setInterceptDrags', { enabled: true }).catch(() => {});
  await guest.send('Input.dispatchDragEvent', { type: 'dragEnter', x: p.x, y: p.y, data });
  await sleep(150);
  await guest.send('Input.dispatchDragEvent', { type: 'dragOver', x: p.x, y: p.y, data });
  await sleep(150);
  await guest.send('Input.dispatchDragEvent', { type: 'drop', x: p.x, y: p.y, data });
  await sleep(300);
  await guest.send('Input.setInterceptDrags', { enabled: false }).catch(() => {});
}

/** the engine's Open or Save dialog is on the page (the file manager in selection mode): its state, and the number of its dialogs so far */
export async function dialogState(guest) {
  return guest.ev("window.__app && window.__app.fileDlg ? { open: window.__app.fileDlg.open, asked: window.__app.fileDlg.history.length } : { open: false, asked: 0 }");
}

/** the new top-level windows since the baseline, as `cls|title` (for the notes) */
export async function freshTitles(guest, base) {
  const nw = await newWindows(guest, base);
  return nw.all.filter((w) => !base.has(w.lid)).map((w) => `${w.cls}|${w.title}`);
}


/** a save or an export by an action (the menu or the key): judged by the file the engine wrote; the file is listed with its size and its name has the extension (ext, or null) */
export async function writtenCase(ctx, action, ext) {
  const r = await writeAndWait(ctx, action);
  const pw = await pageWrites(ctx.guest);
  const f = r.saved[0] ?? (pw.length > 0 ? { name: pw[0].line, size: pw[0].size } : null);
  const ok = f !== null && f.size > 0 && (ext === null || r.saved.length === 0 || f.name.toLowerCase().endsWith(ext));
  const state = (await ctx.guest.state())?.state ?? null;
  const what = f ? `${f.name} ${f.size} B` : "no file listed";
  return { ok, state, file: f, notes: [`${what} (${r.id ?? r.error}; waited ${r.waitedMs} ms; trial window ${r.shop}; windows left open ${r.leftOpen}; listed ${r.saved.length})`] };
}

/** an item of File > Export by its index in the sub-menu (from 0: Wave file is 0) */
export function exportCase(ctx, index, ext) {
  return writtenCase(ctx, () => menuPath(ctx.guest, ['File', '#9', `#${index}`]), ext);
}
