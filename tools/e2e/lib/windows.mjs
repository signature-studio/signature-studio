// The window operations and the window checks of the windows area (Issue #1490). A person's actions are real mouse and key events (lib/guest.mjs); the checks read the
// window table (deskDumpBase64) and the picture's hash of a part of a window (pixelHashRegion). The places are the window table's rectangles plus the offsets inside a
// window (guest px, measured on the 3840 x 2160 guest picture; the guest picture is the same size at every DPR).
import { absoluteRect, childrenOfClass } from './dump.mjs';
import { closeNew, newWindows, openLids, pixelHashRegion, windowTable } from './common.mjs';
import { sleep } from './flow.mjs';

export const RACK = 'TStepSeqForm';
export const PLAYLIST = 'TEventEditForm';
export const MIXER = 'TFXForm';
export const BROWSER = 'TSampleListForm';
export const SETTINGS = 'TMIDIForm';
export const PLUGIN_LIST = 'TPlugListForm';

/** the keys, as Guest.key takes them (code, key, Windows virtual key) */
export const KEYS = { F7: ['F7', 'F7', 118], F8: ['F8', 'F8', 119], F10: ['F10', 'F10', 121], F11: ['F11', 'F11', 122], F12: ['F12', 'F12', 123], Escape: ['Escape', 'Escape', 27] };

const rect = (x, y, w, h) => ({ x, y, w, h });
export { rect };

/** the absolute rectangle of the first shown child of a class (the window table), or null */
export async function childRect(guest, cls, text = undefined) {
  const t = await windowTable(guest);
  const c = childrenOfClass(t, [cls], text).find((x) => x.vis);
  return c ? absoluteRect(t, c.lid) : null;
}

/** the absolute rectangle of a window by lid, or null */
export async function lidRect(guest, lid) {
  return absoluteRect(await windowTable(guest), lid);
}

/** the hash of a part of the picture (a rectangle in guest px) */
export async function hash(guest, r) {
  return pixelHashRegion(guest, r);
}

/** a key press, then the window table's answer after `ms` */
export async function press(guest, name, ms = 1200) {
  await guest.focusDesktop();
  await guest.key(...KEYS[name]);
  await sleep(ms);
}

/** the windows of the top level and the child windows whose text starts with `prefix` */
export async function childTexts(guest, cls, prefix) {
  const t = await windowTable(guest);
  return childrenOfClass(t, [cls]).filter((c) => c.text.startsWith(prefix)).map((c) => ({ lid: c.lid, text: c.text, vis: c.vis, w: c.w, h: c.h }));
}

/** the Channel rack: the offsets of its rows, steps and buttons (guest px from the rack's top-left). Row i is 30 px apart from row 0 */
export const RACK_LAYOUT = { rowY0: 62, rowH: 30, ledX: 18, nameX: 110, knobX: 40, stepX0: 219, stepW: 15.5, plusX: 145, plusY: 235 };
export function rackPoint(rack, row, col) {
  return { x: rack.x + RACK_LAYOUT.stepX0 + (col + 0.5) * RACK_LAYOUT.stepW, y: rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH };
}

/** the grid of steps of the Channel rack (one row's steps as a rectangle, for the hash) */
export function rowStepsRect(rack, row) {
  return rect(rack.x + RACK_LAYOUT.stepX0, rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH - 10, 16 * RACK_LAYOUT.stepW, 20);
}

/** a result: the checks, the values and the notes of one scenario */
export function done(ok, results, notes) {
  return { ok: ok === true, results, notes };
}

export async function rackOf(guest) {
  const r = await childRect(guest, RACK);
  if (!r) {
    const t = await windowTable(guest);
    const all = t.children.filter((c) => c.cls === RACK).map((c) => `lid ${c.lid} parent ${c.parent} vis ${c.vis} ${c.w}x${c.h}`);
    const main = t.children.filter((c) => c.cls === 'TWPControl' && c.lid === 16).map((c) => `${c.x},${c.y} ${c.w}x${c.h} vis ${c.vis}`);
    throw new Error(`the Channel rack (TStepSeqForm) is not shown: ${all.join('; ') || 'not in the table'}; workspace ${main.join('; ') || '-'}; tops ${t.top.map((w) => w.cls).join(',')}`);
  }
  return r;
}

/** a step of the Channel rack: pressed on (the hash of its row changes), and pressed again (it changes back) */
export async function stepToggle(guest, row, col) {
  const rack = await rackOf(guest);
  const area = rowStepsRect(rack, row);
  const p = rackPoint(rack, row, col);
  const h0 = await hash(guest, area);
  await guest.click(p.x, p.y);
  await sleep(600);
  const h1 = await hash(guest, area);
  await guest.click(p.x, p.y);
  await sleep(600);
  const h2 = await hash(guest, area);
  const on = h0 !== h1, off = h1 !== h2;
  return done(on && off, { row, col, changedOn: on, changedOff: off, point: p }, [`step ${row + 1}.${col + 1}: on ${on}, off ${off}`]);
}

/** the mute button of a channel's row: pressed twice; the row's LED area changes each time */
export async function muteToggle(guest, row) {
  const rack = await rackOf(guest);
  const led = rect(rack.x + RACK_LAYOUT.ledX - 12, rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH - 8, 36, 16);
  const p = { x: rack.x + RACK_LAYOUT.ledX, y: rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH };
  const h0 = await hash(guest, led);
  await guest.click(p.x, p.y);
  await sleep(600);
  const h1 = await hash(guest, led);
  await guest.click(p.x, p.y);
  await sleep(600);
  const h2 = await hash(guest, led);
  const on = h0 !== h1, off = h1 !== h2;
  return done(on && off, { row, changedOn: on, changedOff: off }, [`mute row ${row + 1}: changed on ${on}, back ${off}`]);
}

/** a channel's name: clicked; the row's strip changes (its selection) */
export async function selectChannel(guest, row) {
  const rack = await rackOf(guest);
  const strip = rect(rack.x + 100, rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH - 10, 380, 20);
  const h0 = await hash(guest, strip);
  await guest.click(rack.x + RACK_LAYOUT.nameX, rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH);
  await sleep(600);
  const h1 = await hash(guest, strip);
  return done(h0 !== h1, { row, changed: h0 !== h1 }, [`select row ${row + 1}: strip changed ${h0 !== h1}`]);
}

/** the knob of a channel's row: dragged up by `dy` guest px; the knob's area changes */
export async function knobDrag(guest, row, dy = 30) {
  const rack = await rackOf(guest);
  const y = rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH;
  const knob = rect(rack.x + RACK_LAYOUT.knobX - 12, y - 10, 24, 20);
  const h0 = await hash(guest, knob);
  await guest.drag({ x: rack.x + RACK_LAYOUT.knobX, y }, { x: rack.x + RACK_LAYOUT.knobX, y: y - dy }, 10);
  await sleep(600);
  const h1 = await hash(guest, knob);
  return done(h0 !== h1, { row, dy, changed: h0 !== h1 }, [`knob row ${row + 1} dragged ${dy} px: changed ${h0 !== h1}`]);
}

/** the right-click menu of a channel: one drop-down (TQuickPopupMenuWindow) opens; its size is read from the table; Escape closes it */
export async function rowMenu(guest, row) {
  const rack = await rackOf(guest);
  const base = await openLids(guest);
  await guest.rightClick(rack.x + RACK_LAYOUT.nameX, rack.y + RACK_LAYOUT.rowY0 + row * RACK_LAYOUT.rowH, 1000);
  await sleep(800);
  const { pops } = await newWindows(guest, base);
  const p = pops[0] ?? null;
  const left = await closeNew(guest, base);
  return done(!!p && p.h > 0 && left.length === 0, { row, pop: p ? { w: p.w, h: p.h } : null }, [`row ${row + 1} menu: ${pops.length} drop-down(s), ${p ? `${p.w}x${p.h}` : 'none'}`]);
}

/** the + button under the channels: a drop-down opens or the rack's picture changes */
export async function plusButton(guest) {
  const rack = await rackOf(guest);
  const whole = rect(rack.x, rack.y, rack.w, rack.h);
  const h0 = await hash(guest, whole);
  const base = await openLids(guest);
  await guest.click(rack.x + RACK_LAYOUT.plusX, rack.y + RACK_LAYOUT.plusY);
  await sleep(800);
  const h1 = await hash(guest, whole);
  const { pops } = await newWindows(guest, base);
  const left = await closeNew(guest, base);
  const changed = h0 !== h1;
  return done(changed || pops.length > 0, { rackChanged: changed, drops: pops.length }, [`+: rack changed ${changed}, drop-downs ${pops.length}`]);
}

/** a Playlist cell (track t, bar b): clicked with the default tool; the grid changes */
export async function playlistCell(guest, t, b) {
  const pl = await childRect(guest, PLAYLIST);
  if (!pl) throw new Error('the Playlist (TEventEditForm) is not shown');
  const grid = rect(pl.x + 280, pl.y + 60, 1200, 520);
  const p = { x: pl.x + 300 + b * 60, y: pl.y + 100 + t * 48 };
  const h0 = await hash(guest, grid);
  await guest.click(p.x, p.y);
  await sleep(700);
  const h1 = await hash(guest, grid);
  return done(h0 !== h1, { track: t, bar: b, point: p, changed: h0 !== h1 }, [`playlist track ${t + 1} bar ${b + 1}: grid changed ${h0 !== h1}`]);
}

/** a Browser tab (k from 0): clicked; the tab strip's picture changes */
export async function browserTab(guest, k) {
  const t = await windowTable(guest);
  const s = absoluteRect(t, 107);
  if (!s) throw new Error('the Browser tab strip (lid 107) is not in the table');
  const h0 = await hash(guest, s);
  await guest.click(s.x + 22 + k * 44, s.y + 18);
  await sleep(600);
  const h1 = await hash(guest, s);
  return done(h0 !== h1, { tab: k + 1, changed: h0 !== h1 }, [`browser tab ${k + 1}: strip changed ${h0 !== h1}`]);
}

/** a Mixer insert (slot k from 0): clicked; the mixer's insert area changes */
export async function mixerInsert(guest, k) {
  const mx = await childRect(guest, MIXER);
  if (!mx) throw new Error('the Mixer (TFXForm) is not shown');
  const area = rect(mx.x + 100, mx.y + 60, 2600, 400);
  // another slot is pressed first: a slot that is already selected (by the fader of its own slot) does not change its picture
  await guest.click(mx.x + 127 + 44 * (k === 0 ? 1 : 0), mx.y + 120);
  await sleep(600);
  const h0 = await hash(guest, area);
  await guest.click(mx.x + 127 + 44 * k, mx.y + 120);
  await sleep(600);
  const h1 = await hash(guest, area);
  return done(h0 !== h1, { slot: k + 1, changed: h0 !== h1 }, [`insert ${k + 1}: area changed ${h0 !== h1}`]);
}

/** a Mixer fader (slot k from 0): dragged up by `dy`; the fader's area changes */
export async function mixerFader(guest, k, dy = 40) {
  const mx = await childRect(guest, MIXER);
  if (!mx) throw new Error('the Mixer (TFXForm) is not shown');
  const x = mx.x + 127 + 44 * k;
  const area = rect(x - 16, mx.y + 236, 32, 44);
  const h0 = await hash(guest, area);
  await guest.drag({ x, y: mx.y + 258 }, { x, y: mx.y + 258 - dy }, 10);
  await sleep(600);
  const h1 = await hash(guest, area);
  return done(h0 !== h1, { slot: k + 1, dy, changed: h0 !== h1 }, [`fader ${k + 1} dragged ${dy}: changed ${h0 !== h1}`]);
}

/** a top-level window opened by a key: it is new since the baseline (its class and title); closed again with Escape */
export async function openByKey(guest, name, cls) {
  const base = await openLids(guest);
  await guest.focusDesktop();
  await guest.key(...KEYS[name]);
  await sleep(1500);
  const { others } = await newWindows(guest, base);
  const w = others.find((x) => x.cls === cls) ?? null;
  return { w, base };
}

/** the Settings window (TMIDIForm, top-level) opened by a key (F10: the MIDI page; F11: the project credits): its rectangle, or null */
export async function settingsOpen(guest, key = 'F10') {
  const { w, base } = await openByKey(guest, key, SETTINGS);
  const left = await closeNew(guest, base);
  return done(!!w && left.length === 0, { key, title: w?.title ?? null, rect: w ? { x: w.x, y: w.y, w: w.w, h: w.h } : null }, [`${key}: ${w ? `${SETTINGS} "${w.title}" ${w.w}x${w.h}` : 'no Settings window'}`]);
}

/** the Settings window moved by its title (a drag of the caption row): the rectangle moves by the drag */
export async function settingsMove(guest, dx, dy) {
  const { w, base } = await openByKey(guest, 'F10', SETTINGS);
  if (!w) { await closeNew(guest, base); return done(false, {}, ['no Settings window']); }
  const before = { x: w.x, y: w.y, w: w.w, h: w.h };
  await guest.drag({ x: w.x + 300, y: w.y + 12 }, { x: w.x + 300 + dx, y: w.y + 12 + dy }, 12);
  await sleep(800);
  const now = await lidRect(guest, w.lid);
  await closeNew(guest, base);
  const ddx = now.x - before.x, ddy = now.y - before.y;
  const ok = Math.abs(ddx - dx) <= 2 && Math.abs(ddy - dy) <= 2 && now.w === before.w && now.h === before.h;
  return done(ok, { before, after: now, moved: { dx: ddx, dy: ddy }, asked: { dx, dy } }, [`settings moved by ${ddx},${ddy} (asked ${dx},${dy})`]);
}

/** the Settings window's bottom edge dragged down by `dy` guest px: the height grows by it; the width stays */
export async function settingsResize(guest, dy) {
  const { w, base } = await openByKey(guest, 'F10', SETTINGS);
  if (!w) { await closeNew(guest, base); return done(false, {}, ['no Settings window']); }
  const before = { x: w.x, y: w.y, w: w.w, h: w.h };
  await guest.drag({ x: w.x + w.w - 2, y: w.y + w.h - 2 }, { x: w.x + w.w - 2, y: w.y + w.h - 2 + dy }, 12);
  await sleep(800);
  const now = await lidRect(guest, w.lid);
  await closeNew(guest, base);
  const dh = now.h - before.h, dw = now.w - before.w;
  const ok = Math.abs(dh - dy) <= 3 && dw === 0;
  return done(ok, { before, after: now, dh, dw, asked: dy }, [`settings height ${dh} (asked ${dy}), width ${dw}`]);
}

/** a top-level window is closed with Escape: it is gone from the table */
export async function closeByEscape(guest, key, cls) {
  const { w, base } = await openByKey(guest, key, cls);
  if (!w) { await closeNew(guest, base); return done(false, {}, [`${key}: no ${cls}`]); }
  await guest.focusDesktop();
  await guest.escape();
  await sleep(800);
  const gone = !(await guest.windows()).some((x) => x.lid === w.lid);
  await closeNew(guest, base);
  return done(gone, { key, closed: gone }, [`${key} ${cls}: closed by Escape ${gone}`]);
}

/** F12 closes all the windows that are open: only the main form is left in the table */
export async function f12CloseAll(guest, key) {
  const { w, base } = await openByKey(guest, key, cls_for(key));
  if (!w) { await closeNew(guest, base); return done(false, {}, [`${key}: nothing opened`]); }
  await guest.focusDesktop();
  await guest.key(...KEYS.F12);
  await sleep(1500);
  const left = (await guest.windows()).map((x) => x.cls);
  await closeNew(guest, base);
  const ok = !left.includes(w.cls) && left.length === 1;
  return done(ok, { key, left }, [`F12 after ${key}: top-level left ${left.join(', ')}`]);
}
function cls_for(key) {
  return { F8: PLUGIN_LIST, F10: SETTINGS, F11: SETTINGS }[key] ?? '';
}

/** the plugin list (TPlugListForm, top-level) by F8 or Alt+F8: it opens, at the size the table gives */
export async function pluginListOpen(guest, key = 'F8') {
  if (key === 'AltF8') {
    // the inventory's order: F8 first (its list is closed), then Alt+F8 (modifier Alt, no Alt key message)
    await guest.focusDesktop();
    await guest.key(...KEYS.F8);
    await sleep(1500);
    await closeNew(guest, await openLids(guest));
  }
  const base = await openLids(guest);
  await guest.focusDesktop();
  await guest.key(...KEYS.F8, key === 'AltF8' ? 1 : 0);
  await sleep(1500);
  const { others } = await newWindows(guest, base);
  const w = others.find((x) => x.cls === PLUGIN_LIST) ?? null;
  const left = await closeNew(guest, base);
  return done(!!w && left.length === 0, { key, rect: w ? { x: w.x, y: w.y, w: w.w, h: w.h } : null }, [`${key}: ${w ? `${PLUGIN_LIST} ${w.w}x${w.h}` : 'no plugin list'}`]);
}

/** the Piano roll (a child of the main form, class TEventEditForm, caption "Piano roll"): F7 opens it for the selected channel; Escape closes it */
export async function pianoRoll(guest, selectFirst = true) {
  if (selectFirst) await selectChannel(guest, 0);
  await guest.focusDesktop();
  await guest.key(...KEYS.F7);
  await sleep(1500);
  const open = await childTexts(guest, PLAYLIST, 'Piano roll');
  await guest.focusDesktop();
  await guest.escape();
  await sleep(800);
  const after = await childTexts(guest, PLAYLIST, 'Piano roll');
  return done(open.length > 0 && after.length === 0, { open: open.length, afterEscape: after.length, selectFirst }, [`piano roll: open ${open.length}, after Escape ${after.length}`]);
}

/** the Playlist's right-click: a drop-down opens on the grid */
export async function playlistMenu(guest) {
  const pl = await childRect(guest, PLAYLIST);
  const base = await openLids(guest);
  await guest.rightClick(pl.x + 20, pl.y + 60, 1000);
  await sleep(800);
  const { pops } = await newWindows(guest, base);
  const left = await closeNew(guest, base);
  return done(pops.length >= 1 && left.length === 0, { drops: pops.length, pop: pops[0] ? { w: pops[0].w, h: pops[0].h } : null }, [`playlist right-click: ${pops.length} drop-down(s)`]);
}

/** the TEdgeBrowser child under the Browser is shown (the web content of the Browser panel) */
export async function browserWebChild(guest) {
  const t = await windowTable(guest);
  const c = childrenOfClass(t, ['TEdgeBrowser'])[0] ?? null;
  const r = c ? absoluteRect(t, c.lid) : null;
  return done(!!c, { rect: r, shown: c ? c.vis : null }, [c ? `TEdgeBrowser child ${r.w}x${r.h}, shown ${c.vis}` : 'no TEdgeBrowser child in the table']);
}

/** a Mixer fader raised by 40 px and then lowered by 40 px: the picture changes on the raise and again on the lower */
export async function mixerFaderRoundTrip(guest, k) {
  const mx = await childRect(guest, MIXER);
  if (!mx) throw new Error('the Mixer (TFXForm) is not shown');
  const x = mx.x + 127 + 44 * k;
  const area = rect(x - 16, mx.y + 236, 32, 44);
  const h0 = await hash(guest, area);
  await guest.drag({ x, y: mx.y + 258 }, { x, y: mx.y + 218 }, 10);
  await sleep(600);
  const h1 = await hash(guest, area);
  await guest.drag({ x, y: mx.y + 218 }, { x, y: mx.y + 258 }, 10);
  await sleep(600);
  const h2 = await hash(guest, area);
  const raised = h0 !== h1, lowered = h1 !== h2;
  return done(raised && lowered, { slot: k + 1, raised, lowered }, [`fader ${k + 1}: raised ${raised}, lowered ${lowered}`]);
}

/** the plugin list closed with Escape, then opened again by F8: both the close and the open are seen in the table */
export async function reopenPlugin(guest) {
  const a = await closeByEscape(guest, 'F8', PLUGIN_LIST);
  const b = await pluginListOpen(guest, 'F8');
  const ok = a.ok === true && b.ok === true;
  return done(ok, { closed: a.results, opened: b.results }, [...a.notes, ...b.notes]);
}

/** the Settings window closed with F10 and Escape, then opened again by F10 */
export async function reopenSettings(guest) {
  const a = await closeByEscape(guest, 'F10', SETTINGS);
  const b = await settingsOpen(guest, 'F10');
  const ok = a.ok === true && b.ok === true;
  return done(ok, { closed: a.results, opened: b.results }, [...a.notes, ...b.notes]);
}
