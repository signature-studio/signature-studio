// The top bar's parts and their operations (Issue #1485, the topbar area). A part is a child window of the window table: the n-th shown child of its class by lid
// (the number after the hyphen in features/generated/topbar.tsv; the splitter is the 13th shown TQuickEditToolbarPanel). Its place is the table's absolute rectangle.
// A person's operation: a left click, a right click, wheel notches or a drag at the part's centre (a drag along the longer side, from a quarter to three quarters).
// The effect is read from the picture: the part's rectangle hashed before the operation, after one second with no input (the control), and after the operation.
// A part whose picture moves by itself (a meter) has before !== control; its effect is still the change from the control. A right click's menu is a new TQuickPopupMenuWindow that holds the click point.
// The memory label ("<n> MB") is read from the guest's memory (guestText); its value is the MiB floor of the last K32GetProcessMemoryInfo answer in the engine log (#1391).
import { absoluteRect } from './dump.mjs';
import { closeNew, guestText, newWindows, openLids, pixelHashRegion, windowTable } from './common.mjs';
import { sleep } from './flow.mjs';

const CLASS_OF = { twpcontrol: 'TWPControl', tquickcontrolbar: 'TQuickControlBar', tquickedittoolbar: 'TQuickEditToolBar', tquickedittoolbarpanel: 'TQuickEditToolbarPanel', tnewmenu: 'TNewMenu' };
const OPS = ['click', 'rclick', 'wheel', 'drag'];
const MIB = 1048576;
const MEMORY_TOLERANCE_MIB = 4;
const MEMORY_WAIT_MS = 15000;
const MB_RUN = /^(\d+) MB$/;
const K32_CALLS = 'window.__app.engineLog().filter(function (l) { return /memory-usage: K32GetProcessMemoryInfo/.test(l); }).map(function (l) { return Number(/WorkingSetSize (\\d+)/.exec(l)[1]); })';

/** the class and the ordinal of a part from its slug (`twpcontrol-2`: the 2nd TWPControl; `splitter-1`: the 13th TQuickEditToolbarPanel) */
export function partOf(slug) {
  if (slug === 'splitter-1') return { cls: 'TQuickEditToolbarPanel', n: 13 };
  const m = /^([a-z]+)(?:-(\d+))?$/.exec(String(slug));
  const cls = m ? CLASS_OF[m[1]] : undefined;
  if (!cls) throw new Error(`topbar part ${slug}: not one of the top bar's parts (${Object.keys(CLASS_OF).join(', ')}, splitter-1)`);
  return { cls, n: m[2] ? Number(m[2]) : 1 };
}

/** the n-th shown child of the part's class in the table, by lid (the inventory numbers the shown parts; a hidden window of the class does not count); null when there is none */
export function findPart(table, spec) {
  const kids = table.children.filter((c) => c.cls === spec.cls && c.vis).sort((a, b) => a.lid - b.lid);
  return kids[spec.n - 1] ?? null;
}

/** the children of the part's class in the table, by lid, as text (the note when the part is not found) */
function sameClass(t, spec) {
  const kids = t.children.filter((c) => c.cls === spec.cls).sort((a, b) => a.lid - b.lid);
  return kids.length ? kids.map((c) => `lid ${c.lid} ${c.vis ? 'shown' : 'hidden'} ${c.w}x${c.h}`).join(', ') : 'none';
}

const holds = (w, p, tol = 4) => p.x >= w.x - tol && p.y >= w.y - tol && p.x < w.x + w.w + tol && p.y < w.y + w.h + tol;

/** a part that the table holds with no area (its width or height is 0): the part is listed, and it has no place to operate on */
export async function presentPart(guest, slug) {
  const spec = partOf(slug);
  const t = await windowTable(guest);
  const c = findPart(t, spec);
  const engineState = (await guest.state())?.state ?? null;
  if (!c) return { ok: false, engineState, notes: [`${slug}: no ${spec.cls} #${spec.n} in the window table (${sameClass(t, spec)})`], results: [] };
  const zero = c.w === 0 || c.h === 0;
  const row = { part: slug, cls: spec.cls, n: spec.n, lid: c.lid, vis: c.vis, size: [c.w, c.h] };
  return { ok: zero, engineState, notes: [`${slug}: listed as ${spec.cls} lid ${c.lid}, ${c.w}x${c.h}, ${zero ? 'with no area (presence only)' : 'with an area (not a zero-size part)'}`], results: [row] };
}

/**
 * one operation on a part: op is click, rclick, wheel or drag. ok: a click changes the part's picture or opens a window; a right click opens a drop-down at the point;
 * a wheel or a drag changes the part's picture. Returns { ok, engineState, notes, results } (results: one row with the hashes, the new windows and the menu).
 */
export async function partOperation(guest, slug, op) {
  if (!OPS.includes(op)) throw new Error(`partOperation: op ${op} (${OPS.join(', ')})`);
  const spec = partOf(slug);
  const t = await windowTable(guest);
  const hit = findPart(t, spec);
  const rect = hit ? absoluteRect(t, hit.lid) : null;
  const at = hit && rect ? { lid: hit.lid, rect } : null;
  const engineState = (await guest.state())?.state ?? null;
  if (!at || at.rect.w < 1 || at.rect.h < 1) {
    return { ok: false, engineState, notes: [`${slug}: no ${spec.cls} #${spec.n} with an area in the window table (${sameClass(t, spec)})`], results: [] };
  }
  const r = at.rect;
  const c = { x: r.x + r.w / 2, y: r.y + r.h / 2 };
  const base = await openLids(guest);
  const before = await pixelHashRegion(guest, r);
  await sleep(1000);
  const control = await pixelHashRegion(guest, r);
  if (op === 'click') await guest.click(c.x, c.y);
  else if (op === 'rclick') await guest.rightClick(c.x, c.y);
  else if (op === 'wheel') await guest.wheel(c.x, c.y, 3, 120);
  else {
    const horizontal = r.w >= r.h;
    const a = horizontal ? { x: r.x + r.w * 0.25, y: c.y } : { x: c.x, y: r.y + r.h * 0.25 };
    const b = horizontal ? { x: r.x + r.w * 0.75, y: c.y } : { x: c.x, y: r.y + r.h * 0.75 };
    await guest.drag(a, b, 10);
  }
  await sleep(1200);
  const after = await pixelHashRegion(guest, r);
  const fresh = await newWindows(guest, base);
  const opened = [...fresh.pops, ...fresh.others].map((w) => w.cls);
  const menu = fresh.pops.find((w) => holds(w, c)) ?? null;
  await closeNew(guest, base);
  const changed = after !== control;
  const animating = control !== before;
  const inert = op === 'click' && slug === 'splitter-1';
  const ok = op === 'rclick' ? fresh.pops.length > 0 : op === 'click' ? (inert ? !changed && opened.length === 0 : changed || opened.length > 0) : changed;
  const row = {
    part: slug, op, cls: spec.cls, n: spec.n, lid: at.lid, rect: r, before, control, after, animating, changed,
    opened, menu: menu ? { cls: menu.cls, x: menu.x, y: menu.y, w: menu.w, h: menu.h } : null,
  };
  const verdict = inert ? (ok ? 'inert: a click does not move it' : 'moved by a click') : (ok ? 'effect' : 'no effect');
  const note = `${slug} ${op}: ${verdict} (picture ${changed ? 'changed' : 'unchanged'}${animating ? ', moves by itself' : ''}; opened ${opened.join(',') || 'none'}; menu ${fresh.pops.length ? `${fresh.pops[0].w}x${fresh.pops[0].h}${menu ? ' at the point' : ' not at the point'}` : 'none'})`;
  return { ok, engineState: (await guest.state())?.state ?? engineState, notes: [note], results: [row] };
}

/** the "<n> MB" runs of a guestText answer (the memory label): { mb, run } */
function memoryRuns(text) {
  const out = [];
  for (const n of text.near) for (const run of n.runs) {
    const m = MB_RUN.exec(String(run).trim());
    if (m) out.push({ mb: Number(m[1]), run: String(run).trim() });
  }
  return out;
}

/**
 * the top bar's Memory Usage: the "<n> MB" label in the guest's memory is read until it agrees with the last K32GetProcessMemoryInfo answer
 * (its MiB floor, within MEMORY_TOLERANCE_MIB; the label is redrawn, so the wait is up to MEMORY_WAIT_MS). Returns { ok, engineState, notes, results }.
 */
export async function memoryValue(guest) {
  const t0 = Date.now();
  let calls = [], runs = [], shown = null;
  for (;;) {
    runs = memoryRuns(await guestText(guest, [' MB'], 40));
    calls = await guest.ev(K32_CALLS);
    const last = calls.length ? calls[calls.length - 1] : null;
    const floor = last === null ? null : Math.floor(last / MIB);
    shown = floor === null ? null : runs.find((r) => r.mb > 0 && Math.abs(r.mb - floor) <= MEMORY_TOLERANCE_MIB) ?? null;
    if (shown || Date.now() - t0 >= MEMORY_WAIT_MS) break;
    await sleep(500);
  }
  const last = calls.length ? calls[calls.length - 1] : null;
  const row = {
    feature: 'topbar/value/memory', k32Calls: calls.length, lastBytes: last, floorMiB: last === null ? null : Math.floor(last / MIB),
    labels: runs.map((r) => r.run), shownMiB: shown ? shown.mb : null, waitedMs: Date.now() - t0,
  };
  const note = `memory label ${shown ? `${shown.mb} MB` : 'not found'}; K32 calls ${calls.length}, last ${last ?? 'none'} B (floor ${row.floorMiB ?? 'none'} MiB, tolerance ${MEMORY_TOLERANCE_MIB})`;
  return { ok: shown !== null, engineState: (await guest.state())?.state ?? null, notes: [note], results: [row] };
}
