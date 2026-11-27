// The menus' scenarios (Issue #1489): what a menu item does, read from the window table (the new windows, the submenus still open, the windows that changed) and the page's title.
// The pixels are not the judge: a picture change is only written to the notes. The items are chosen by their place in the drop-down (`#n`), with real keys and mouse clicks (lib/common.mjs).
// A scenario starts with ensureEngine: when the engine has stopped (exited or error), the page is loaded again, so that one item's stop does not fail the next scenarios of the shard.
import { sleep, waitStable } from './flow.mjs';
import { windowTable, closeNew, menuPath, openMenu, menuBarRect, pixelHashRegion, readMenuRows, startProject, POPUP, MENU_BAR } from './common.mjs';
import { rootOf } from './dump.mjs';

const ESC = { code: 'Escape', key: 'Escape', vk: 27 };

/** the menus can be used now: the File drop-down opens from its label, and Escape closes it (no drop-down is left) */
async function menusUsable(guest) {
  const t = await windowTable(guest);
  if (!menuBarRect(t)) return false;
  const base = new Set(t.top.map((w) => w.lid));
  await openMenu(guest, 'File');
  await sleep(600);
  const open = (await windowTable(guest)).top.some((w) => w.cls === POPUP && !base.has(w.lid));
  await closeNew(guest, base);
  return open;
}

/**
 * the engine is idle, the menus are usable and no page box waits for a person; otherwise the page is loaded again (and the project started, as common.mjs persist).
 * A stopped engine (exited or error), the Open box that Ctrl+O leaves up, or a bar that a View item hid stop every later choice of the shared page; the new load clears them.
 */
export async function ensureEngine({ guest, page }) {
  const s = await waitStable(guest, 60000);
  const boxUp = await guest.ev("!!document.getElementById('fx-input-files')").catch(() => true);
  const usable = s.state === 'engine-idle' && !boxUp && (await menusUsable(guest).catch(() => false));
  if (usable) return { restarted: false, state: s.state };
  await page.reload({ waitUntil: 'load', timeout: 120000 });
  const st = await startProject(guest);
  return { restarted: true, state: st.state, opened: st.opened };
}

/** the window table, the lids of its top-level windows, the page's title and engine state, and whether the page's Open box is up (it is not a window of the table) */
async function snapshot(guest) {
  const table = await windowTable(guest);
  const st = await guest.state();
  const box = await guest.ev("!!document.getElementById('fx-input-files')").catch(() => false);
  return { table, lids: new Set(table.top.map((w) => w.lid)), title: st?.title ?? null, state: st?.state ?? null, box };
}

/** the windows that the menus do not count as an effect: the drop-downs (TQuickPopupMenuWindow) and the menu bar (TNewMenu) and what is in them */
function sig(table) {
  const m = new Map();
  for (const w of table.top) if (w.cls !== POPUP && w.cls !== MENU_BAR) m.set(w.lid, `${w.cls}|${w.title}|${w.x},${w.y},${w.w},${w.h}`);
  for (const c of table.children) {
    if (c.cls === MENU_BAR) continue;
    const root = rootOf(table, c.lid);
    if (!root || root.cls === POPUP || root.cls === MENU_BAR) continue;
    m.set(c.lid, `${c.cls}|${c.text}|${c.vis ? 1 : 0}|${c.x},${c.y},${c.w},${c.h}`);
  }
  return m;
}

/**
 * the windows that change with no input (a meter, a clock): two snapshots a second apart with no key or click between them. Their changes are not an effect of a choice.
 */
const quietByPage = new WeakMap();
export async function quietWindows(guest) {
  if (quietByPage.has(guest)) return quietByPage.get(guest);
  const a = sig((await snapshot(guest)).table);
  await sleep(1000);
  const b = sig((await snapshot(guest)).table);
  const quiet = new Set([...a.keys()].filter((k) => b.has(k) && a.get(k) !== b.get(k)));
  quietByPage.set(guest, quiet);
  return quiet;
}

/**
 * what changed between two snapshots: `dialogs` (new top-level windows that are not drop-downs), `popups` (new drop-downs still open: a submenu),
 * `changed` (windows that were there and whose class, text, visibility or place changed, except the `quiet` ones), `titleChanged`. hasEffect: any of them.
 */
export function observe(before, after, quiet = new Set()) {
  const baseTop = new Set(before.table.top.map((w) => w.lid));
  const fresh = after.table.top.filter((w) => !baseTop.has(w.lid));
  const popups = fresh.filter((w) => w.cls === POPUP).sort((a, b) => a.lid - b.lid);
  const dialogs = fresh.filter((w) => w.cls !== POPUP && w.cls !== MENU_BAR);
  const sb = sig(before.table), sa = sig(after.table);
  const changed = [...sa.keys()].filter((k) => sb.has(k) && sb.get(k) !== sa.get(k) && !quiet.has(k));
  const titleChanged = before.title !== after.title;
  const box = !before.box && after.box;
  const byLid = new Map([...after.table.top, ...after.table.children].map((w) => [w.lid, w]));
  return {
    dialogs,
    popups,
    changed: changed.map((k) => ({ lid: k, cls: byLid.get(k)?.cls ?? '', text: byLid.get(k)?.title || byLid.get(k)?.text || '' })),
    titleChanged,
    box,
    hasEffect: dialogs.length > 0 || popups.length > 0 || changed.length > 0 || titleChanged || box,
  };
}

const winName = (w) => `${w.cls}${w.title ? `[${w.title}]` : ''}`;

/** the notes of an observation, one line */
export function describe(obs) {
  const parts = [
    `windows: ${obs.dialogs.length ? obs.dialogs.map(winName).join(', ') : 'none'}`,
    `submenus open: ${obs.popups.length}`,
    `changed: ${obs.changed.length ? obs.changed.map((c) => `${c.cls}${c.text ? `[${c.text}]` : ''}`).slice(0, 6).join(', ') : 'none'}`,
    `title: ${obs.titleChanged ? 'changed' : 'same'}`,
    `page box (Open): ${obs.box ? 'up' : 'no'}`,
  ];
  return parts.join('; ');
}

const engineState = async (guest) => (await guest.state())?.state ?? null;

/** the sizes of the drop-downs that are open (a submenu is one more than its parent); null when none. The items are not counted here (each count is a Down press per item, with a whole picture read each time) */
const popSizes = (pops) => (pops.length ? pops.map((p) => `${p.w}x${p.h}`) : null);

/** a scenario's result with one more note (the display the set runs it on, for example) */
export function withNote(r, note) {
  return { ...r, notes: [...(r.notes ?? []), note] };
}

/** the E0 list's item count of a label (the rows `menus/item/<label>-<n>`), for the notes */
function e0Count(label) {
  const slug = label.toLowerCase();
  return readMenuRows().filter((r) => new RegExp(`^menus/item/${slug}-\\d+$`).test(r.id)).length;
}

/**
 * chooses a menu item by its path (menuPath: the keys Down and Enter, as a person does). ok: the choice shows an effect (a window, a submenu or a changed window or title)
 * and the engine is idle afterwards. The new windows are closed afterwards (Escape), and their number left is in the notes.
 */
export async function chooseItem(ctx, names) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const quiet = await quietWindows(guest);
  const before = await snapshot(guest);
  const id = await menuPath(guest, names);
  await sleep(700);
  const after = await snapshot(guest);
  const obs = observe(before, after, quiet);
  const sub = popSizes(obs.popups);
  const left = await closeNew(guest, before.lids);
  const notes = [`${names.join(' > ')} is ${id}`, describe(obs)];
  if (sub !== null) notes.push(`drop-downs open: ${sub.join(', ')}`);
  if (left.length) notes.push(`still open after Escape: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  if (obs.changed.length && !obs.dialogs.length && !obs.popups.length) {
    // a toggle (a panel shown or hidden): the same choice again puts it back, so the next scenario starts from the same window table
    await menuPath(guest, names);
    await sleep(700);
    await closeNew(guest, before.lids);
    notes.push('the change was a toggle: the same item was chosen again to put it back');
  }
  const state = await engineState(guest);
  return {
    ok: obs.hasEffect && state === 'engine-idle',
    engineState: state,
    notes,
    results: { id, dialogs: obs.dialogs.map(winName), submenus: obs.popups.length, submenuSizes: sub, changed: obs.changed.length, titleChanged: obs.titleChanged },
  };
}

/**
 * chooses a dialog's item and closes the dialog by `how`: ok (Enter, the dialog's default button), cancel (Escape), esc (a click on the dialog's caption, then Escape).
 * ok: the choice opens a window, and the window is gone from the table after the answer, and the engine is idle.
 */
export async function dialogAnswer(ctx, names, how) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  const id = await menuPath(guest, names);
  await sleep(700);
  const mid = await snapshot(guest);
  const obs = observe(before, mid);
  const notes = [`${names.join(' > ')} is ${id}`, `answer: ${how}`];
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  const dlg = obs.dialogs.find((w) => w.cls !== 'TNewCaption') ?? obs.dialogs[0] ?? null;
  if (!dlg) {
    await closeNew(guest, before.lids);
    return { ok: false, engineState: await engineState(guest), notes: [...notes, describe(obs), 'no window opened: no dialog to answer'], results: { id, dialogs: [], answer: how } };
  }
  if (how === 'ok') await guest.key('Enter', 'Enter', 13);
  else if (how === 'cancel') await guest.key(ESC.code, ESC.key, ESC.vk);
  else {
    await guest.click(dlg.x + dlg.w / 2, dlg.y + 8);
    await sleep(300);
    await guest.key(ESC.code, ESC.key, ESC.vk);
  }
  await sleep(900);
  const after = await snapshot(guest);
  const gone = obs.dialogs.every((w) => !after.table.top.some((t) => t.lid === w.lid));
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  notes.push(`dialog ${winName(dlg)} ${gone ? 'is gone' : 'is still there'} after ${how}`);
  if (left.length) notes.push(`still open after the answer: ${left.map(winName).join(', ')}`);
  return { ok: gone && state === 'engine-idle', engineState: state, notes, results: { id, dialogs: obs.dialogs.map(winName), answer: how, gone } };
}

/** the same item twice: the effect of the first choice, the window closed by Escape, the effect of the second choice (the same kind) */
export async function chooseTwice(ctx, names) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const quiet = await quietWindows(guest);
  const b1 = await snapshot(guest);
  const id = await menuPath(guest, names);
  await sleep(700);
  const o1 = observe(b1, await snapshot(guest), quiet);
  const left1 = await closeNew(guest, b1.lids);
  await sleep(500);
  const b2 = await snapshot(guest);
  await menuPath(guest, names);
  await sleep(700);
  const o2 = observe(b2, await snapshot(guest), quiet);
  const left2 = await closeNew(guest, b2.lids);
  const state = await engineState(guest);
  // the same kind: the same window classes (a title can differ: a new pattern is named with its number), and a change in both or in neither
  const same = o1.dialogs.map((w) => w.cls).join('|') === o2.dialogs.map((w) => w.cls).join('|') && o1.changed.length > 0 === o2.changed.length > 0;
  const notes = [`${names.join(' > ')} is ${id}`, `first: ${describe(o1)}`, `second: ${describe(o2)}`];
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  if (left1.length || left2.length) notes.push(`still open after Escape: ${[...left1, ...left2].map(winName).join(', ')}`);
  return { ok: o1.hasEffect && o2.hasEffect && same && state === 'engine-idle', engineState: state, notes, results: { id, first: o1.dialogs.map(winName), second: o2.dialogs.map(winName), same } };
}

/** Alt with a key, as a person presses it: Alt goes down first (the engine reads Alt from its own key messages), then the key, then Alt comes up */
export async function altChord(guest, code, key, vk) {
  await guest.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'AltLeft', key: 'Alt', windowsVirtualKeyCode: 0x12, nativeVirtualKeyCode: 0x12, modifiers: 1 });
  await sleep(40);
  await guest.key(code, key, vk, 1);
  await guest.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'AltLeft', key: 'Alt', windowsVirtualKeyCode: 0x12, nativeVirtualKeyCode: 0x12, modifiers: 0 });
  await sleep(40);
}

/** the kind of effect of an observation, for comparing two paths: the dialogs (class and title), a changed window, a changed title (a drop-down the item leaves open is not compared) */
const effectOf = (obs) => `dialogs=${obs.dialogs.map(winName).join('|') || 'none'};changed=${obs.changed.length ? 1 : 0};title=${obs.titleChanged ? 1 : 0}`;

/**
 * a shortcut and the menu item that it duplicates, each as a person chooses it, on the same page: the key first (press), then the item (names).
 * ok: both have an effect, and it is the same kind of effect (effectOf). A toggle (a change with no window) is put back by the same path, so the second path starts from the same state.
 */
export async function keyAndMenu(ctx, label, press, names) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const quiet = await quietWindows(guest);
  const b1 = await snapshot(guest);
  await press(guest);
  await sleep(900);
  const o1 = observe(b1, await snapshot(guest), quiet);
  if (o1.changed.length && !o1.dialogs.length && !o1.popups.length) {
    await press(guest);
    await sleep(700);
  }
  const left1 = await closeNew(guest, b1.lids);
  await sleep(500);
  const b2 = await snapshot(guest);
  const id = await menuPath(guest, names);
  await sleep(700);
  const o2 = observe(b2, await snapshot(guest), quiet);
  if (o2.changed.length && !o2.dialogs.length && !o2.popups.length) {
    await menuPath(guest, names);
    await sleep(700);
  }
  const left2 = await closeNew(guest, b2.lids);
  const state = await engineState(guest);
  const e1 = effectOf(o1), e2 = effectOf(o2);
  const notes = [`shortcut ${label} and ${names.join(' > ')} (${id})`, `shortcut: ${describe(o1)}`, `menu: ${describe(o2)}`, `same effect: ${e1 === e2 ? 'yes' : 'no'} (${e1} | ${e2})`];
  if (left1.length || left2.length) notes.push(`still open after Escape: ${[...left1, ...left2].map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return {
    ok: o1.hasEffect && o2.hasEffect && e1 === e2 && state === 'engine-idle',
    engineState: state,
    notes,
    results: { key: label, id, shortcut: o1.dialogs.map(winName), menu: o2.dialogs.map(winName), sameEffect: e1 === e2 },
  };
}

/** a key (or a Ctrl or Alt chord) as a person presses it: the effect it has on the window table; the new windows are then closed */
export async function shortcut(ctx, label, press, restore = null) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const quiet = await quietWindows(guest);
  const before = await snapshot(guest);
  await press(guest);
  await sleep(900);
  const obs = observe(before, await snapshot(guest), quiet);
  const sub = popSizes(obs.popups);
  if (restore) await restore(guest);
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const notes = [`shortcut ${label}`, describe(obs)];
  if (sub !== null) notes.push(`drop-downs open: ${sub.join(', ')}`);
  if (left.length) notes.push(`still open after Escape: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: obs.hasEffect && state === 'engine-idle', engineState: state, notes, results: { key: label, dialogs: obs.dialogs.map(winName), submenus: obs.popups.length, submenuSizes: sub, changed: obs.changed.length, titleChanged: obs.titleChanged } };
}

/** a label's drop-down opens on a click, and its item count is read (Down presses) */
export async function labelOpens(ctx, label) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  await openMenu(guest, label);
  await sleep(900);
  const obs = observe(before, await snapshot(guest));
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const p = obs.popups[0];
  const notes = [`${label} drop-down: ${p ? `${p.w}x${p.h}` : 'none'}`, `items in the E0 list: ${e0Count(label)}`];
  if (left.length) notes.push(`still open after Escape: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: obs.popups.length >= 1 && state === 'engine-idle', engineState: state, notes, results: { label, e0Items: e0Count(label), w: p?.w ?? null, h: p?.h ?? null } };
}

/** the label's drop-down closes on Escape */
export async function labelEscape(ctx, label) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  await openMenu(guest, label);
  await sleep(900);
  const opened = observe(before, await snapshot(guest)).popups.length;
  await guest.key(ESC.code, ESC.key, ESC.vk);
  await sleep(700);
  const closed = observe(before, await snapshot(guest)).popups.length;
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const notes = [`${label}: drop-downs open ${opened}, after Escape ${closed}`];
  if (left.length) notes.push(`still open: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: opened >= 1 && closed === 0 && state === 'engine-idle', engineState: state, notes, results: { label, opened, closedAfterEscape: closed } };
}

/** a click on another label while one drop-down is open: the first click closes it (no drop-down is left), and a second click on that label opens its drop-down (to the right of the first) */
export async function labelSwitch(ctx, from, to) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  await openMenu(guest, from);
  await sleep(900);
  const first = observe(before, await snapshot(guest)).popups;
  await openMenu(guest, to);
  await sleep(900);
  const afterClick = observe(before, await snapshot(guest)).popups;
  await openMenu(guest, to);
  await sleep(900);
  const second = observe(before, await snapshot(guest)).popups;
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const right = first.length === 1 && second.length === 1 && second[0].x > first[0].x;
  const notes = [`${from} open, then ${to}: drop-downs ${afterClick.length} after the click on ${to}, ${second.length} after a second click on ${to}`];
  if (left.length) notes.push(`still open: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: right && afterClick.length === 0 && state === 'engine-idle', engineState: state, notes, results: { from, to, afterFirstClick: afterClick.length, afterSecondClick: second.length } };
}

/** ArrowRight on a submenu's item (the keys: Down to the item, ArrowRight opens its submenu): the submenu opens (a new drop-down) */
export async function arrowOpens(ctx, names) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  await openMenu(guest, names[0]);
  for (const p of names.slice(1)) {
    const k = Number(p.slice(1));
    for (let i = 0; i < k; i++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(120); }
    await guest.key('ArrowRight', 'ArrowRight', 39);
    await sleep(900);
  }
  const obs = observe(before, await snapshot(guest));
  const n = popSizes(obs.popups);
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const notes = [`${names.join(' > ')} by ArrowRight: submenus open ${obs.popups.length}`, `drop-downs open: ${n ? n.join(', ') : 'none'}`];
  if (left.length) notes.push(`still open: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: obs.popups.length >= 1 && state === 'engine-idle', engineState: state, notes, results: { path: names.join(' > '), submenus: obs.popups.length, submenuSizes: n } };
}

/** a submenu's Escape closes only the submenu: the drop-down of its parent stays open */
export async function submenuEscape(ctx, names) {
  const { guest } = ctx;
  const pre = await ensureEngine(ctx);
  const before = await snapshot(guest);
  await menuPath(guest, names);
  await sleep(500);
  const opened = observe(before, await snapshot(guest)).popups.length;
  await guest.key(ESC.code, ESC.key, ESC.vk);
  await sleep(700);
  const after = observe(before, await snapshot(guest)).popups.length;
  const left = await closeNew(guest, before.lids);
  const state = await engineState(guest);
  const notes = [`${names.join(' > ')}: drop-downs open ${opened}, after Escape ${after}`];
  if (left.length) notes.push(`still open: ${left.map(winName).join(', ')}`);
  if (pre.restarted) notes.push('the engine had stopped: the page was loaded again first');
  return { ok: opened >= 2 && after === opened - 1 && state === 'engine-idle', engineState: state, notes, results: { path: names.join(' > '), opened, afterEscape: after } };
}
