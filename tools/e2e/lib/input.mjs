// The shared steps of the input scenarios (Issue #1487). The actions are real mouse, key and touch events, sent the way lib/guest.mjs sends them (CDP Input);
// no product function is called. The reads are the page's own records and test aids:
//   window.__app.latency.records: every input the page sent: `desc` as the engine names it ("key down vk 0x41 scan 0x1e", "button L down at screen (x, y)",
//     "move at screen (x, y)", "wheel 120 at screen (x, y)"); `processed` once the engine was idle again after it; `dropped` when it was not taken;
//   the engine's window-API trace: the `hit:` lines of the pointer (while window.__app.windowApiTrace(true) is on);
//   the window table (lib/dump.mjs), the picture (lib/picture.mjs), the guest's memory text (guestText), the page's cursor (#desktop's cursor),
//   and the page's clipboard record (window.__app.clipboard) with a count of the navigator.clipboard calls (installed once per page).
// The text is read from the guest's memory, never from the picture (no OCR). The screen's coordinates are the guest's pixels (the engine's screen).
import { createHash } from 'node:crypto';
import { sleep } from './flow.mjs';
import { guestText, windowTable, menuBarRect, menuLabelPlace, POPUP, MAIN_FORM } from './common.mjs';
import { absoluteRect, parseDump, rootOf } from './dump.mjs';
import { decodePicture, regionHash } from './picture.mjs';

export const WS_THICKFRAME = 0x00040000;
export const WS_MAXIMIZE = 0x01000000;
export const WS_MINIMIZE = 0x20000000;
export const EDGE_CSS = { N: 'ns-resize', S: 'ns-resize', E: 'ew-resize', W: 'ew-resize', NE: 'nesw-resize', SW: 'nesw-resize', NW: 'nwse-resize', SE: 'nwse-resize' };
// the Piano roll's grid as fractions of its rectangle: where the pointer is put in turn (tools/cursor/measure.mjs PENCIL_POINTS)
export const PENCIL_POINTS = [[0.5, 0.5], [0.3, 0.5], [0.7, 0.5], [0.5, 0.75], [0.3, 0.75], [0.7, 0.75], [0.5, 0.3], [0.8, 0.9]];
const LETTERS = 'abcdefghijklmnopqrstuvwxyz';
// the engine's button numbers (lift/runtime/fle_input.h: L 0, R 1, M 2) and the names of the events (CDP)
const BUTTONS = { left: { cdp: 'left', mask: 1, letter: 'L', engine: 0 }, middle: { cdp: 'middle', mask: 4, letter: 'M', engine: 2 }, right: { cdp: 'right', mask: 2, letter: 'R', engine: 1 } };
const HIT = /hit: (move|down|up) btn=(-?\d+) first=(0x[0-9a-f]+|0) final=(0x[0-9a-f]+|0) ht=(-?\d+) captured=([01]) ptr=(-?\d+),(-?\d+)/;
const ASK_AGAIN = /ask again/;

export const stateOf = async (guest) => (await guest.state())?.state ?? null;
export const word = (n = 6, rnd = Math.random) => Array.from({ length: n }, () => LETTERS[Math.floor(rnd() * 26)]).join('');

// ---- the engine's input records -------------------------------------------------------------------------------------

/** the seq of the last input the page keeps (a mark: the records after it are the ones the step made) */
export function markSeq(page) {
  return page.evaluate(() => {
    const rs = window.__app.latency.records;
    return rs.length > 0 ? rs[rs.length - 1].seq : 0;
  });
}

/** the inputs the page sent after the mark: desc, processed (the engine was idle after it), dropped */
export function recordsSince(page, mark) {
  return page.evaluate((m) => window.__app.latency.records.filter((r) => r.seq > m).map((r) => ({ desc: r.desc, processed: r.tIdle !== null, dropped: r.dropped !== null })), mark);
}

/** waits until every input the page sent is processed or dropped (pending 0); false at the limit */
export async function settle(page, ms = 15000) {
  const t0 = Date.now();
  for (;;) {
    if ((await page.evaluate(() => window.__app.latency.pending)) === 0) return true;
    if (Date.now() - t0 >= ms) return false;
    await sleep(150);
  }
}

export const hasDesc = (recs, desc) => recs.some((r) => r.processed && r.desc === desc);
export const countDesc = (recs, desc) => recs.filter((r) => r.processed && r.desc === desc).length;
export const hasKey = (recs, vk, down = true) => recs.some((r) => r.processed && r.desc.startsWith(`key ${down ? 'down' : 'up'} vk 0x${vk.toString(16)} `));
export const countKey = (recs, vk, down = true) => recs.filter((r) => r.processed && r.desc.startsWith(`key ${down ? 'down' : 'up'} vk 0x${vk.toString(16)} `)).length;
export const btnDesc = (b, kind, x, y) => `button ${BUTTONS[b].letter} ${kind} at screen (${x}, ${y})`;
export const moveDesc = (x, y) => `move at screen (${x}, ${y})`;

// ---- the engine's pointer trace (`hit:` lines) -----------------------------------------------------------------------

export function traceOn(guest) { return guest.ev('window.__app.windowApiTrace(true)'); }
export function logMark(guest) { return guest.ev('window.__app.engineLog().length'); }
export function parseHit(line) {
  const m = HIT.exec(String(line));
  return m ? { what: m[1], btn: Number(m[2]), final: m[4], ht: Number(m[5]), x: Number(m[7]), y: Number(m[8]) } : null;
}
/** the pointer's hit lines the engine printed after the log mark (the trace must be on) */
export async function hitsSince(guest, mark) {
  const lines = await guest.ev(`window.__app.engineLog().slice(${Number(mark)}).filter((l) => /hit: /.test(l))`);
  return lines.map(parseHit).filter(Boolean);
}
/** a hit that went to a window (final is not 0) */
export const hitWindowed = (h) => h.final !== '0' && h.final !== '0x0';

// ---- the desk: the window table, the styles and the picture --------------------------------------------------------

/** the window table's style words: top-level lines (`style 0x...`) and child lines (`child style lid N 0x...`) */
export function styleMap(text) {
  const st = new Map();
  for (const l of String(text).split('\n')) {
    let m = /^lid (\d+) at \(-?\d+, -?\d+\) \d+x\d+ style 0x([0-9a-fA-F]+)/.exec(l);
    if (m) { st.set(Number(m[1]), parseInt(m[2], 16) >>> 0); continue; }
    m = /^child style lid (\d+) 0x([0-9a-fA-F]+)\s*$/.exec(l);
    if (m) st.set(Number(m[1]), parseInt(m[2], 16) >>> 0);
  }
  return st;
}

/** the desk now: the window table, the styles, and the picture as the page draws it (asked again while the engine presents: as guest.windows does) */
export async function deskNow(guest) {
  const t0 = Date.now();
  for (;;) {
    try {
      const d = await guest.ev('window.__app.deskDumpBase64(false)');
      return { table: parseDump(d.windows), styles: styleMap(d.windows), pic: decodePicture({ base64: d.base64, width: d.width, height: d.height }) };
    } catch (e) {
      if (!ASK_AGAIN.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
      guest.dumpRetries++;
      await sleep(100);
    }
  }
}

/** the table and the SHA-256 of a rectangle of the picture (the whole picture when rect is null) */
export async function snap(guest, rect = null) {
  const s = await deskNow(guest);
  return { table: s.table, styles: s.styles, hash: regionHash(s.pic, rect ?? { x: 0, y: 0, w: s.pic.w, h: s.pic.h }) };
}

// ---- windows: top-level and child, with their rectangles (guest px) -------------------------------------------------

/** every window of a table: { lid, cls, label (the title, or the child's text), child, vis, rect } */
export function windowsOf(t) {
  return [
    ...t.top.map((w) => ({ lid: w.lid, cls: w.cls, label: w.title, child: false, vis: true, rect: { x: w.x, y: w.y, w: w.w, h: w.h } })),
    ...t.children.map((c) => ({ lid: c.lid, cls: c.cls, label: c.text, child: true, vis: c.vis, rect: absoluteRect(t, c.lid) })),
  ];
}
export const findWin = (t, pred) => windowsOf(t).find(pred) ?? null;
export const lidsOf = (t) => new Set(windowsOf(t).map((w) => w.lid));
/** the windows that are new since the base (lids) and shown */
export const freshShown = (t, base) => windowsOf(t).filter((w) => !base.has(w.lid) && w.vis !== false);
export const isPianoRoll = (w) => w.vis !== false && /^Piano roll/.test(w.label ?? '');

export async function ensurePianoRoll(guest) {
  let w = findWin(await windowTable(guest), isPianoRoll);
  if (!w) {
    await guest.focusDesktop();
    await fkeyPress(guest, 7);
    await waitFor(async () => findWin(await windowTable(guest), isPianoRoll), 4000);
    w = findWin(await windowTable(guest), isPianoRoll);
  }
  return w;
}

/** a window the pick finds is shown; when none is, the key shows it (a scenario sets the desk it needs, since the scenarios share the page) */
export async function ensureShown(guest, pick, key) {
  const shown = async () => findWin(await windowTable(guest), (x) => pick(x) && x.vis !== false);
  let w = await shown();
  if (!w) {
    await guest.focusDesktop();
    await fkeyPress(guest, key);
    w = await waitFor(shown, 4000);
  }
  return w;
}
export const ensureMixer = (guest) => ensureShown(guest, (x) => x.cls === 'TFXForm', 9);
export const ensureChannelRack = (guest) => ensureShown(guest, (x) => x.cls === 'TStepSeqForm', 6);
export const ensurePlaylist = (guest) => ensureShown(guest, (x) => x.cls === 'TEventEditForm' && !/Piano roll/.test(x.label ?? ''), 5);

/** Escape until no window is new (shown) since the base; returns the new windows still shown */
export async function closeFresh(guest, base) {
  await guest.focusDesktop();
  for (let i = 0; i < 8; i++) {
    if (freshShown(await windowTable(guest), base).length === 0) return [];
    await guest.escape();
    await sleep(400);
  }
  return freshShown(await windowTable(guest), base);
}

/** the test is asked every 150 ms until it gives a value (returned) or the limit passes (null) */
export async function waitFor(test, ms) {
  const t0 = Date.now();
  for (;;) {
    const v = await test().catch(() => null);
    if (v) return v;
    if (Date.now() - t0 >= ms) return null;
    await sleep(150);
  }
}

/** the picture of a rectangle (the desk when null) once it stops changing (two reads 300 ms apart are the same), or at the limit: its hash */
export async function pictureSettled(guest, rect = null, ms = 4000) {
  const t0 = Date.now();
  let last = (await snap(guest, rect)).hash;
  for (;;) {
    await sleep(300);
    const h = (await snap(guest, rect)).hash;
    if (h === last || Date.now() - t0 >= ms) return h;
    last = h;
  }
}

/** the top-level window on top at a guest point: the table lists the drawn top-level windows bottom first, so the last one holding the point is the top one (0: none) */
export function topLidAt(top, x, y) {
  let r = 0;
  for (const w of top) if (x >= w.x && y >= w.y && x < w.x + w.w && y < w.y + w.h) r = w.lid;
  return r;
}

// ---- keys ----------------------------------------------------------------------------------------------------------

const NAMED = {
  Enter: ['Enter', 'Enter', 13], Escape: ['Escape', 'Escape', 27], Tab: ['Tab', 'Tab', 9], Space: ['Space', ' ', 32], Backspace: ['Backspace', 'Backspace', 8],
  Delete: ['Delete', 'Delete', 46], Home: ['Home', 'Home', 36], End: ['End', 'End', 35], ArrowLeft: ['ArrowLeft', 'ArrowLeft', 37], ArrowUp: ['ArrowUp', 'ArrowUp', 38],
  ArrowRight: ['ArrowRight', 'ArrowRight', 39], ArrowDown: ['ArrowDown', 'ArrowDown', 40],
};

export const pressNamed = (guest, name, modifiers = 0) => guest.key(NAMED[name][0], NAMED[name][1], NAMED[name][2], modifiers);
/** the function key Fn (F1 is 0x70) */
export const fkeyPress = (guest, n, modifiers = 0) => guest.key(`F${n}`, `F${n}`, 0x6f + n, modifiers);

/** the key of a character typed in an edit box: lower case letters, upper case letters (with Shift), digits, a space and a hyphen (US layout) */
export function charKey(ch) {
  if (/^[a-z]$/.test(ch)) return { code: `Key${ch.toUpperCase()}`, key: ch, vk: 0x41 + LETTERS.indexOf(ch), shift: false };
  if (/^[A-Z]$/.test(ch)) return { code: `Key${ch}`, key: ch, vk: 0x41 + LETTERS.indexOf(ch.toLowerCase()), shift: true };
  if (/^[0-9]$/.test(ch)) return { code: `Digit${ch}`, key: ch, vk: 0x30 + Number(ch), shift: false };
  if (ch === ' ') return { code: 'Space', key: ' ', vk: 0x20, shift: false };
  if (ch === '-') return { code: 'Minus', key: '-', vk: 0xbd, shift: false };
  throw new Error(`no key for ${JSON.stringify(ch)}`);
}

/** the left Shift key goes down (or up) itself: the engine reads the Shift state from its own key messages */
export async function shiftKey(guest, down) {
  await guest.send('Input.dispatchKeyEvent', { type: down ? 'rawKeyDown' : 'keyUp', code: 'ShiftLeft', key: 'Shift', windowsVirtualKeyCode: 0xa0, nativeVirtualKeyCode: 0xa0, modifiers: down ? 8 : 0 });
  await sleep(40);
}

export async function typeText(guest, text) {
  for (const ch of text) {
    const k = charKey(ch);
    if (k.shift) await shiftKey(guest, true);
    await guest.key(k.code, k.key, k.vk, k.shift ? 8 : 0);
    if (k.shift) await shiftKey(guest, false);
  }
}

/** a key held down: one key down, then `n - 1` repeats (autoRepeat), then the key up */
export async function holdKey(guest, name, n) {
  const [code, key, vk] = NAMED[name];
  const ev = (type, extra = {}) => guest.send('Input.dispatchKeyEvent', { type, code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: 0, ...extra });
  await ev('rawKeyDown');
  for (let i = 1; i < n; i++) { await sleep(60); await ev('rawKeyDown', { autoRepeat: true }); }
  await sleep(60);
  await ev('keyUp');
  await sleep(100);
}

export async function altFkey(guest, n) {
  await guest.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'AltLeft', key: 'Alt', windowsVirtualKeyCode: 0xa4, nativeVirtualKeyCode: 0xa4, modifiers: 1 });
  await sleep(40);
  await fkeyPress(guest, n, 1);
  await guest.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'AltLeft', key: 'Alt', windowsVirtualKeyCode: 0xa4, nativeVirtualKeyCode: 0xa4, modifiers: 0 });
  await sleep(100);
}

// ---- mouse ---------------------------------------------------------------------------------------------------------

/** a press and a release of a button at a guest point (one click, or `clicks` clicks in a row with clickCount 1, 2 ...) */
export async function press(guest, gx, gy, button = 'left', clicks = 1) {
  const c = await guest.pointAt(gx, gy);
  await guest.move(c);
  const b = BUTTONS[button];
  for (let k = 1; k <= clicks; k++) {
    await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: b.cdp, buttons: b.mask, clickCount: k, pointerType: 'mouse' });
    await sleep(60);
    await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: b.cdp, buttons: 0, clickCount: k, pointerType: 'mouse' });
    await sleep(120);
  }
}

/** the centre of a menu label in guest px (the bar is read from the window table) */
export async function labelPoint(guest, label) {
  const bar = menuBarRect(await windowTable(guest));
  if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
  return menuLabelPlace(bar, label);
}

/** the drop-down (TQuickPopupMenuWindow) that is new since the base, the largest first; null when none */
export function dropDown(t, base) {
  return freshShown(t, base).filter((w) => w.cls === POPUP).sort((a, b) => b.rect.h - a.rect.h)[0] ?? null;
}

/** the Piano roll's rectangle centre in guest px, or null when there is no Piano roll */
export async function pianoRollCentre(guest) {
  const w = await ensurePianoRoll(guest);
  if (!w || !w.rect) return null;
  return { x: Math.floor(w.rect.x + w.rect.w / 2), y: Math.floor(w.rect.y + w.rect.h / 2), rect: w.rect };
}

export async function wheelAt(guest, gx, gy, notches, deltaX, deltaY) {
  const c = await guest.pointAt(gx, gy);
  for (let i = 0; i < notches; i++) {
    await guest.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX, deltaY, pointerType: 'mouse' });
    await sleep(300);
  }
}

// ---- the rename box (the channel's name box, opened by F2 on the selected channel) ------------------------------------

/** F2: the box opens; returns the base (the lids before) and the new windows shown */
export async function openRename(guest) {
  await ensureChannelRack(guest);
  await guest.focusDesktop();
  const base = lidsOf(await windowTable(guest));
  await fkeyPress(guest, 2);
  await waitFor(async () => freshShown(await windowTable(guest), base).length > 0, 3000);
  return { base, fresh: freshShown(await windowTable(guest), base) };
}

/** the guest's memory holds the text (ASCII or UTF-16LE): the marker is the text itself */
export async function memHas(guest, text) {
  const t = await guestText(guest, [text]);
  return t.best !== null && t.best.marker === text;
}

// ---- cursor --------------------------------------------------------------------------------------------------------

/** the cursor of the desk (the page sets #desktop's cursor from the engine's cursor) */
export const deskCursor = (page) => page.evaluate(() => { const d = document.getElementById('desktop'); return d.style.cursor || getComputedStyle(d).cursor; });
export const appCursor = (page) => page.evaluate(() => {
  const c = window.__app.cursor;
  const p = c.picture;
  return { css: c.css, why: c.why, picture: p ? { width: p.width, height: p.height, hotX: p.hotX, hotY: p.hotY, css: p.css, source: p.source } : null };
});

/** the pointer at a guest point, and the cursor once two reads in a row agree (the engine answers a move at its next idle) */
export async function cursorAt(guest, page, x, y, ms = 6000) {
  await guest.move(await guest.pointAt(x, y));
  let last = null, same = 0;
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    const v = await deskCursor(page);
    if (v === last) { same++; if (same >= 2) return v; } else { last = v; same = 0; }
    await sleep(250);
  }
  return last;
}

/** a point 2 px inside the window's edge (or corner) in the direction dir (N, S, E, W, NE, NW, SE, SW), in guest px */
export function edgePoint(r, dir) {
  const x = dir.includes('W') ? r.x + 2 : dir.includes('E') ? r.x + r.w - 3 : r.x + Math.floor(r.w / 2);
  const y = dir.includes('N') ? r.y + 2 : dir.includes('S') ? r.y + r.h - 3 : r.y + Math.floor(r.h / 2);
  return { x, y };
}

/** the sizable windows of a desk (WS_THICKFRAME, not maximised or minimised, visible): the Piano roll first */
export function sizableWindows(desk) {
  return windowsOf(desk.table).filter((w) => {
    const st = desk.styles.get(w.lid);
    return st !== undefined && (st & WS_THICKFRAME) !== 0 && (st & (WS_MAXIMIZE | WS_MINIMIZE)) === 0 && w.vis !== false && w.rect && w.rect.w >= 150 && w.rect.h >= 150;
  }).sort((a, b) => Number(isPianoRoll(b)) - Number(isPianoRoll(a)));
}

// ---- clipboard -----------------------------------------------------------------------------------------------------

const CLIP_INSTALL = `(() => { if (window.__e2eInputClip) return true; const c = navigator.clipboard; const n = { readText: 0, writeText: 0 }; window.__e2eInputClip = n; if (!c) return false; for (const k of ['readText', 'writeText']) { if (typeof c[k] !== 'function') continue; const f = c[k].bind(c); c[k] = (...a) => { n[k]++; return f(...a); }; } return true; })()`;
export const clipCounter = (page) => page.evaluate(CLIP_INSTALL);
export const clipCounts = (page) => page.evaluate(() => (window.__e2eInputClip ? { ...window.__e2eInputClip } : { readText: 0, writeText: 0 }));
export const clipRecord = (page) => page.evaluate(() => {
  const c = window.__app.clipboard;
  return { readPermission: c.readPermission, writePermission: c.writePermission, reads: c.reads, writes: c.writes, lastWritten: c.lastWritten, lastRead: c.lastRead, refusal: c.refusal };
});
export const permissionPair = (page) => page.evaluate(() => Promise.all(['clipboard-read', 'clipboard-write'].map((n) => navigator.permissions.query({ name: n }).then((p) => p.state, () => 'unknown'))).then(([read, write]) => ({ read, write })));

/** the browser's permission for the clipboard (both names) for this page's origin and browser context */
export async function setClip(ctx, setting) {
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await ctx.browserCdp.send('Browser.setPermission', { permission: { name }, setting, origin: ctx.origin, browserContextId: ctx.browserContextId });
  }
}

/** the page's own Clipboard API calls, with a limit: a call that does not answer in 3 s is null */
export function clipWriteFromPage(page, text) {
  return page.evaluate(async (t) => { try { await navigator.clipboard.writeText(t); return true; } catch (e) { return String(e?.message ?? e).slice(0, 120); } }, text);
}
export function clipReadFromPage(page) {
  return Promise.race([
    page.evaluate(async () => { try { return await navigator.clipboard.readText(); } catch (e) { return null; } }),
    sleep(3000).then(() => null),
  ]);
}

// ---- touch ---------------------------------------------------------------------------------------------------------

export const touchOn = (guest, on) => guest.send('Emulation.setTouchEmulationEnabled', { enabled: on, maxTouchPoints: 5 });
/** a touch event with the touch points given in CSS px (their ids are their positions in the list) */
export const touchSend = (guest, type, pts) => guest.send('Input.dispatchTouchEvent', { type, touchPoints: pts.map((p, i) => ({ x: p.x, y: p.y, id: i })) });

// ---- IME (the page's composition, as the browser sends it) -------------------------------------------------------------

export const imeSet = (guest, text) => guest.send('Input.imeSetComposition', { text, selectionStart: text.length, selectionEnd: text.length });
export const imeInsert = (guest, text) => guest.send('Input.insertText', { text });

export const activeId = (page) => page.evaluate(() => (document.activeElement ? document.activeElement.id : null));

/** Enter on the rename box: it commits and closes; true when no box is shown after it */
export async function commitRename(guest, base, page) {
  await pressNamed(guest, 'Enter');
  await sleep(800);
  await settle(page, 8000);
  return freshShown(await windowTable(guest), base).length === 0;
}

// ---- the flows of the scenarios -----------------------------------------------------------------------------------------

const fail = (guest, notes) => guest.state().then((s) => ({ ok: false, engineState: s?.state ?? null, notes }));

/** a left click on a menu label opens its drop-down (a new TQuickPopupMenuWindow below the bar); Escape closes it */
export async function menuLabelClick({ guest }, label) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, label);
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const pop = dropDown(await windowTable(guest), base);
  const left = await closeFresh(guest, base);
  return { ok: pop !== null && pop.rect.w > 0 && left.length === 0, engineState: await stateOf(guest), notes: [`${label}: drop-down ${pop ? `${pop.rect.w}x${pop.rect.h} at ${pop.rect.x},${pop.rect.y}` : 'none'}; shown after Escape ${left.length}`] };
}

/** a press of a button on a place (the desk's centre, the Mixer's centre or a menu label): the press and the release reach the engine at the place; the hit line of the press goes to a window */
export async function clickTrace({ guest, page }, button, where) {
  await traceOn(guest);
  await sleep(300);
  let p;
  if (where === 'desk') {
    const d = await guest.desk();
    p = { x: Math.floor(d.gw * 0.5), y: Math.floor(d.gh * 0.5) };
  } else if (where === 'mixer') {
    const w = await ensureMixer(guest);
    if (!w) return fail(guest, ['no Mixer in the window table']);
    p = { x: Math.floor(w.rect.x + w.rect.w / 2), y: Math.floor(w.rect.y + w.rect.h / 2) };
  } else {
    const q = await labelPoint(guest, where);
    p = { x: Math.floor(q.x), y: Math.floor(q.y) };
  }
  const lmark = await logMark(guest);
  const mark = await markSeq(page);
  await press(guest, p.x, p.y, button);
  await settle(page, 8000);
  await sleep(300);
  const recs = await recordsSince(page, mark);
  const hits = await hitsSince(guest, lmark);
  const down = hasDesc(recs, btnDesc(button, 'down', p.x, p.y));
  const up = hasDesc(recs, btnDesc(button, 'up', p.x, p.y));
  const hit = hits.find((h) => h.what === 'down' && h.btn === BUTTONS[button].engine && h.x === p.x && h.y === p.y) ?? null;
  return { ok: down && up && hit !== null && hitWindowed(hit), engineState: await stateOf(guest), notes: [`${button} at ${p.x},${p.y} (${where}): records down ${down}, up ${up}; the engine's hit line ${hit ? `final ${hit.final} ht ${hit.ht}` : 'none'}`] };
}

/** a press of the right button on a shown window (20, 60 inside its rectangle, held for `hold` ms): the window's drop-down opens; Escape closes it */
export async function rightClickWindow({ guest }, pick, label, hold = 60) {
  const t = await windowTable(guest);
  const w = findWin(t, pick);
  if (!w || !w.rect) return fail(guest, [`${label}: no shown window of that kind`]);
  const base = lidsOf(t);
  await guest.rightClick(w.rect.x + 20, w.rect.y + 60, hold);
  await sleep(800);
  const pop = dropDown(await windowTable(guest), base);
  const left = await closeFresh(guest, base);
  return { ok: pop !== null && left.length === 0, engineState: await stateOf(guest), notes: [`${label}: drop-down ${pop ? `${pop.rect.w}x${pop.rect.h} at ${pop.rect.x},${pop.rect.y}` : 'none'} after a ${hold} ms press at ${w.rect.x + 20},${w.rect.y + 60}; shown after Escape ${left.length}`] };
}

/** two clicks in a row on the desk (clickCount 1 and 2): two button downs and two ups reach the engine at the place (the double-click rule is the engine's) */
export async function doubleClick({ guest, page }, trace) {
  if (trace) { await traceOn(guest); await sleep(300); }
  const d = await guest.desk();
  const p = { x: Math.floor(d.gw * 0.5), y: Math.floor(d.gh * 0.5) };
  const lmark = await logMark(guest);
  const mark = await markSeq(page);
  await press(guest, p.x, p.y, 'left', 2);
  await settle(page, 8000);
  await sleep(300);
  const recs = await recordsSince(page, mark);
  const downs = countDesc(recs, btnDesc('left', 'down', p.x, p.y));
  const ups = countDesc(recs, btnDesc('left', 'up', p.x, p.y));
  const hits = trace ? (await hitsSince(guest, lmark)).filter((h) => h.what === 'down' && h.btn === 0 && h.x === p.x && h.y === p.y).length : null;
  return { ok: downs === 2 && ups === 2 && (hits === null || hits >= 2), engineState: await stateOf(guest), notes: [`double click at ${p.x},${p.y}: records down ${downs}, up ${ups}${trace ? `; hit lines down ${hits}` : ''}`] };
}

/** the left button from one place to another (12 moves): the press at the start, the moves while it is held, the release at the end */
export async function dragCase({ guest, page }, mode) {
  const d = await guest.desk();
  const from = { x: Math.floor(d.gw * 0.3), y: Math.floor(d.gh * 0.4) };
  const to = mode === 'same' ? from : { x: Math.floor(d.gw * 0.45), y: Math.floor(d.gh * 0.55) };
  const mark = await markSeq(page);
  await guest.drag(from, to, 12);
  await settle(page, 8000);
  await sleep(300);
  const recs = await recordsSince(page, mark);
  const down = hasDesc(recs, btnDesc('left', 'down', from.x, from.y));
  const up = hasDesc(recs, btnDesc('left', 'up', to.x, to.y));
  const moves = recs.filter((r) => r.processed && r.desc.startsWith('move at screen ('));
  const between = moves.filter((r) => r.desc !== moveDesc(from.x, from.y) && r.desc !== moveDesc(to.x, to.y)).length;
  const atEnd = moves.some((r) => r.desc === moveDesc(to.x, to.y));
  const ok = mode === 'start' ? down : mode === 'middle' ? down && between >= 4 : mode === 'end' ? up && atEnd : down && up;
  return { ok, engineState: await stateOf(guest), notes: [`drag ${from.x},${from.y} -> ${to.x},${to.y}: button down at the start ${down}; moves processed ${moves.length} (${between} between the ends); a move at the end ${atEnd}; button up at the end ${up}`] };
}

/** three notches over the Piano roll: the wheel reaches the engine (its delta: 120 for a notch away from the user, -120 towards), and the Piano roll's picture changes */
export async function wheelCase({ guest, page }, deltaY) {
  const pr = await pianoRollCentre(guest);
  if (!pr) return fail(guest, ['no Piano roll window (F7)']);
  const before = await snap(guest, pr.rect);
  const mark = await markSeq(page);
  await wheelAt(guest, pr.x, pr.y, 3, 0, deltaY);
  await settle(page, 8000);
  await sleep(1000);
  const after = await snap(guest, pr.rect);
  const recs = await recordsSince(page, mark);
  const delta = deltaY > 0 ? -120 : 120;
  const n = countDesc(recs, `wheel ${delta} at screen (${pr.x}, ${pr.y})`);
  return { ok: n >= 3 && before.hash !== after.hash, engineState: await stateOf(guest), notes: [`wheel deltaY ${deltaY} x3 over the Piano roll at ${pr.x},${pr.y}: records wheel ${delta} processed ${n}; the Piano roll's picture ${before.hash === after.hash ? 'unchanged' : 'changed'}`] };
}

/** a horizontal wheel (deltaX) over the Piano roll: the engine should be given it and the view moves; the page sends no horizontal wheel (keymap and pointer.ts read deltaY only) */
export async function wheelHorizontal({ guest, page }) {
  const pr = await pianoRollCentre(guest);
  if (!pr) return fail(guest, ['no Piano roll window (F7)']);
  const before = await snap(guest, pr.rect);
  const mark = await markSeq(page);
  await wheelAt(guest, pr.x, pr.y, 2, -120, 0);
  await settle(page, 8000);
  await sleep(1000);
  const after = await snap(guest, pr.rect);
  const recs = await recordsSince(page, mark);
  const wheels = recs.filter((r) => r.desc.startsWith('wheel ')).length;
  return { ok: wheels >= 2 && before.hash !== after.hash, engineState: await stateOf(guest), notes: [`horizontal wheel (deltaX -120) x2 over the Piano roll: wheel records ${wheels}; the picture ${before.hash === after.hash ? 'unchanged' : 'changed'}`] };
}

/** the pointer over the File drop-down's items: the moves reach the engine and the drop-down's picture changes (the highlight follows the pointer) */
export async function hoverMenu({ guest, page }) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, 'File');
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const pop = dropDown(await windowTable(guest), base);
  if (!pop) return fail(guest, ['no File drop-down']);
  const r = pop.rect;
  const h0 = (await snap(guest, r)).hash;
  const mark = await markSeq(page);
  for (const f of [0.3, 0.5, 0.7]) {
    await guest.move(await guest.pointAt(Math.floor(r.x + r.w * 0.5), Math.floor(r.y + r.h * f)));
    await sleep(400);
  }
  const h1 = (await snap(guest, r)).hash;
  const moves = (await recordsSince(page, mark)).filter((x) => x.processed && x.desc.startsWith('move at screen (')).length;
  const left = await closeFresh(guest, base);
  return { ok: h1 !== h0 && moves >= 3 && left.length === 0, engineState: await stateOf(guest), notes: [`the pointer over the File drop-down: ${moves} moves processed; its picture ${h0 === h1 ? 'unchanged' : 'changed'} (the highlight)`] };
}

/** ten moves over the desk: the engine's hit line of the last move is the last place */
export async function moveTrace({ guest, page }) {
  await traceOn(guest);
  await sleep(300);
  const d = await guest.desk();
  const lmark = await logMark(guest);
  const mark = await markSeq(page);
  const y = Math.floor(d.gh * 0.3);
  const pts = Array.from({ length: 10 }, (_, i) => ({ x: Math.floor(d.gw * 0.2) + i * 20, y }));
  for (const p of pts) await guest.move(await guest.pointAt(p.x, p.y));
  await settle(page, 8000);
  await sleep(300);
  const hits = (await hitsSince(guest, lmark)).filter((h) => h.what === 'move');
  const recs = (await recordsSince(page, mark)).filter((r) => r.processed && r.desc.startsWith('move at screen ('));
  const last = pts[pts.length - 1];
  const lastHit = hits.length > 0 ? hits[hits.length - 1] : null;
  return { ok: recs.length >= 9 && lastHit !== null && lastHit.x === last.x && lastHit.y === last.y, engineState: await stateOf(guest), notes: [`10 moves over the desk: ${recs.length} processed; the engine's last move line at ${lastHit ? `${lastHit.x},${lastHit.y}` : 'none'} (sent ${last.x},${last.y}); ${hits.length} move lines`] };
}

/** 200 moves as fast as the page sends them (one pixel apart): every one is processed or dropped, the last one reaches the engine */
export async function move1khz({ guest, page }) {
  const d = await guest.desk();
  const y = Math.floor(d.gh * 0.35), x0 = Math.floor(d.gw * 0.2), N = 200;
  const mark = await markSeq(page);
  const t0 = Date.now();
  for (let i = 0; i < N; i++) {
    const c = await guest.pointAt(x0 + i, y);
    await guest.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
  }
  const ms = Date.now() - t0;
  const settled = await settle(page, 15000);
  const recs = await recordsSince(page, mark);
  const moves = recs.filter((r) => r.desc.startsWith('move at screen ('));
  const processed = moves.filter((r) => r.processed).length;
  const dropped = moves.filter((r) => r.dropped).length;
  const lastOk = hasDesc(recs, moveDesc(x0 + N - 1, y));
  return { ok: settled && moves.length >= Math.floor(N * 0.9) && processed + dropped === moves.length && lastOk, engineState: await stateOf(guest), notes: [`${N} moves sent in ${ms} ms (${(ms / N).toFixed(2)} ms apart); processed ${processed}, dropped ${dropped} of ${moves.length} records; the last one processed ${lastOk}`] };
}

/** a left click on the desk gives the desk the focus (the page's pointer handler focuses #desktop) */
export async function focusClick({ guest, page }) {
  // Tab takes the focus off the desk (a script's blur is given back to the desk by the page: keymap and desktop.ts)
  await guest.focusDesktop();
  await pressNamed(guest, 'Tab');
  await sleep(300);
  const before = await activeId(page);
  const d = await guest.desk();
  await press(guest, Math.floor(d.gw * 0.5), Math.floor(d.gh * 0.5));
  await sleep(300);
  const after = await activeId(page);
  return { ok: after === 'desktop' && before !== 'desktop', engineState: await stateOf(guest), notes: [`focus before the click "${before}", after it "${after}"`] };
}

/** types text in the channel name box (F2) and checks the text is in the guest's memory; `commit` 'enter' also checks that Enter closes the box */
export async function typedRename({ guest, page }, text, commit = 'none') {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2 (the channel name box)']); }
  await typeText(guest, text);
  await settle(page, 8000);
  const inMem = await memHas(guest, text);
  const closed = commit === 'enter' ? await commitRename(guest, r.base, page) : null;
  await closeFresh(guest, r.base);
  return { ok: commit === 'enter' ? inMem && closed === true : inMem, engineState: await stateOf(guest), notes: [`typed "${text}": in the guest's memory ${inMem}${closed === null ? '' : `; the box closed on Enter ${closed}`}`] };
}

/** the text typed, then Shift+Left selects the last character and a typed letter replaces it */
export async function shiftArrow({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, w.slice(0, 3));
  await shiftKey(guest, true);
  await pressNamed(guest, 'ArrowLeft', 8);
  await shiftKey(guest, false);
  await typeText(guest, 'x');
  await settle(page, 8000);
  const marker = `${w.slice(0, 2)}x`;
  const inMem = await memHas(guest, marker);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`Shift+Left selects the last character; "${marker}" in the guest's memory ${inMem}`] };
}

/** Ctrl+A selects the whole text: a typed text replaces it */
export async function ctrlA({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w1 = word(4), w2 = word(5);
  await typeText(guest, w1);
  await guest.ctrl('KeyA', 'a', 0x41);
  await typeText(guest, w2);
  await settle(page, 8000);
  const inMem = await memHas(guest, w2);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`Ctrl+A, then "${w2}" typed: in the guest's memory ${inMem}`] };
}

/** Ctrl+Left moves the caret to the start of the word before: a letter typed there goes before it */
export async function ctrlArrow({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, `${w.slice(0, 2)} ${w.slice(2)}`);
  await guest.ctrl('ArrowLeft', 'ArrowLeft', 37);
  await typeText(guest, 'x');
  await settle(page, 8000);
  const marker = `${w.slice(0, 2)} x${w.slice(2)}`;
  const inMem = await memHas(guest, marker);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`Ctrl+Left, then x: "${marker}" in the guest's memory ${inMem}`] };
}

/** Alt+F8 opens the plugin list (a top-level TPlugListForm) */
export async function altF8({ guest, page }) {
  const base = lidsOf(await windowTable(guest));
  await guest.focusDesktop();
  const mark = await markSeq(page);
  await altFkey(guest, 8);
  await waitFor(async () => windowsOf(await windowTable(guest)).some((w) => w.cls === 'TPlugListForm' && w.vis !== false) || null, 4000);
  const shown = windowsOf(await windowTable(guest)).some((w) => w.cls === 'TPlugListForm' && w.vis !== false);
  const recs = await recordsSince(page, mark);
  const left = await closeFresh(guest, base);
  return { ok: shown, engineState: await stateOf(guest), notes: [`Alt+F8: the plugin list ${shown ? 'shown' : 'not shown'} (Alt down reached the engine ${hasKey(recs, 0xa4)}, F8 down ${hasKey(recs, 0x77)}); shown after Escape ${left.length}`] };
}

/** the shown windows (top-level and child) as a set of class, title and rectangle */
export async function shownSet(guest) {
  return new Set(windowsOf(await windowTable(guest)).filter((w) => w.vis !== false).map((w) => `${w.cls}"${(w.label ?? '').slice(0, 20)}"${w.rect ? `${w.rect.x},${w.rect.y},${w.rect.w}x${w.rect.h}` : ''}`));
}

/** a function key reaches the engine (its key down and key up are processed) and the engine stays idle; what the key shows is not read */
export async function fkeyDelivered({ guest, page }, n) {
  await guest.focusDesktop();
  const base = lidsOf(await windowTable(guest));
  const mark = await markSeq(page);
  await fkeyPress(guest, n);
  await settle(page, 8000);
  await sleep(600);
  const recs = await recordsSince(page, mark);
  const down = hasKey(recs, 0x6f + n, true), up = hasKey(recs, 0x6f + n, false);
  const st = await stateOf(guest);
  const left = await closeFresh(guest, base);
  return { ok: down && up && st === 'engine-idle', engineState: st, notes: [`F${n}: key down ${down}, key up ${up} processed; the engine ${st}; shown windows closed ${left.length === 0}`] };
}

/** F2 opens the channel name box (a new window) */
export async function fkeyF2({ guest }) {
  const r = await openRename(guest);
  const left = await closeFresh(guest, r.base);
  return { ok: r.fresh.length >= 1, engineState: await stateOf(guest), notes: [`F2: ${r.fresh.length} new window(s): ${r.fresh.map((w) => `${w.cls} ${w.rect?.w}x${w.rect?.h}`).join('; ') || 'none'}; shown after Escape ${left.length}`] };
}

/** F7 shows the Piano roll (a window whose caption starts with "Piano roll"); an open one is closed first with Escape */
export async function fkeyPianoRoll({ guest }) {
  await guest.focusDesktop();
  if (findWin(await windowTable(guest), isPianoRoll)) { await guest.escape(); await sleep(600); }
  await fkeyPress(guest, 7);
  await waitFor(async () => findWin(await windowTable(guest), isPianoRoll), 4000);
  const w = findWin(await windowTable(guest), isPianoRoll);
  return { ok: w !== null, engineState: await stateOf(guest), notes: [`F7: the Piano roll ${w ? `shown at ${w.rect.x},${w.rect.y} ${w.rect.w}x${w.rect.h} (${w.cls}, "${w.label}")` : 'not shown'}`] };
}

/** a function key shows a window of the class (and the caption, when given): an open one is closed first with Escape; then the key; then Escape */
export async function fkeyWindow({ guest }, n, cls, labelRe = null) {
  await guest.focusDesktop();
  await guest.escape();
  await sleep(500);
  await fkeyPress(guest, n);
  const find = async () => windowsOf(await windowTable(guest)).find((w) => w.cls === cls && w.vis !== false && (labelRe === null || labelRe.test(w.label ?? ''))) ?? null;
  await waitFor(find, 4000);
  const w = await find();
  await guest.escape();
  await sleep(600);
  return { ok: w !== null, engineState: await stateOf(guest), notes: [`F${n}: ${cls} ${w ? `shown at ${w.rect.x},${w.rect.y} ${w.rect.w}x${w.rect.h}, caption "${w.label}"` : 'not shown'}`] };
}

/** Escape closes a window the key shows (the window is shown before, gone after) */
export async function escapeWindow({ guest }, n, cls) {
  await guest.focusDesktop();
  await guest.escape();
  await sleep(500);
  await fkeyPress(guest, n);
  const shown = async () => windowsOf(await windowTable(guest)).some((w) => w.cls === cls && w.vis !== false);
  await waitFor(async () => (await shown()) || null, 4000);
  const before = await shown();
  await guest.escape();
  await sleep(800);
  const after = await shown();
  return { ok: before && !after, engineState: await stateOf(guest), notes: [`F${n}: ${cls} shown ${before}; after Escape shown ${after}`] };
}

/** a function key that shows or hides the desk's picture (F5 the Playlist, F6 the Browser, F9 the Mixer): one press changes the picture; a second press brings it back (`mode` 'toggle') */
export async function fkeyToggle({ guest, page }, n, mode) {
  await guest.focusDesktop();
  const set0 = await shownSet(guest);
  const h0 = (await snap(guest)).hash;
  await fkeyPress(guest, n);
  await settle(page, 8000);
  const h1 = await pictureSettled(guest);
  const set1 = await shownSet(guest);
  if (mode === 'once') {
    const r = { ok: h1 !== h0, engineState: await stateOf(guest), notes: [`F${n}: the picture ${h1 !== h0 ? 'changed' : 'unchanged'} by one press; shown windows ${set0.size} -> ${set1.size}`] };
    await fkeyPress(guest, n);
    await settle(page, 8000);
    return r;
  }
  await fkeyPress(guest, n);
  await settle(page, 8000);
  const h2 = await pictureSettled(guest);
  const set2 = await shownSet(guest);
  const lost = [...set0].filter((k) => !set2.has(k)), extra = [...set2].filter((k) => !set0.has(k));
  const diff = lost.length || extra.length ? `; not restored: ${lost.slice(0, 3).join(' | ') || '-'}; extra: ${extra.slice(0, 3).join(' | ') || '-'}` : '';
  return { ok: h1 !== h0 && h2 === h0, engineState: await stateOf(guest), notes: [`F${n}: the picture changed by one press ${h1 !== h0}; the same as before after the second press ${h2 === h0}; shown windows ${set0.size} -> ${set1.size} -> ${set2.size}${diff}`] };
}

/** the Down key moves the highlight of the File drop-down: `down` (one press changes it), `down2` (two presses), `up` (Down, Down, Up: the highlight is back on the first item) */
export async function arrowMenu({ guest }, mode) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, 'File');
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const pop = dropDown(await windowTable(guest), base);
  if (!pop) return fail(guest, ['no File drop-down']);
  const r = pop.rect;
  const hash = async () => (await snap(guest, r)).hash;
  const h0 = await pictureSettled(guest, r, 3000);
  let ok;
  let notes;
  if (mode === 'down') {
    await pressNamed(guest, 'ArrowDown');
    await sleep(500);
    const h1 = await hash();
    ok = h1 !== h0;
    notes = [`Down: the drop-down's picture ${ok ? 'changed' : 'unchanged'}`];
  } else if (mode === 'down2') {
    await pressNamed(guest, 'ArrowDown');
    await sleep(500);
    const h1 = await hash();
    await pressNamed(guest, 'ArrowDown');
    await sleep(500);
    const h2 = await hash();
    ok = h1 !== h0 && h2 !== h1;
    notes = [`Down, Down: each changes the picture (${h1 !== h0}, ${h2 !== h1})`];
  } else {
    await pressNamed(guest, 'ArrowDown');
    await sleep(500);
    const h1 = await hash();
    await pressNamed(guest, 'ArrowDown');
    await sleep(500);
    const h2 = await hash();
    await pressNamed(guest, 'ArrowUp');
    await sleep(500);
    const h3 = await hash();
    ok = h2 !== h1 && h3 === h1;
    notes = [`Down, Down, Up: the picture after Up is the one after the first Down (${h3 === h1}); after the second Down it differs (${h2 !== h1}); after Up it differs from the second Down (${h3 !== h2})`];
  }
  const left = await closeFresh(guest, base);
  return { ok: ok && left.length === 0, engineState: await stateOf(guest), notes: [...notes, `shown after Escape ${left.length}`] };
}

/** the caret moves with Left (or with Home and Right): a letter typed after it goes in its place */
export async function arrowCaret({ guest, page }, dir) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, w);
  if (dir === 'left') {
    await pressNamed(guest, 'ArrowLeft');
    await pressNamed(guest, 'ArrowLeft');
  } else {
    await pressNamed(guest, 'Home');
    await pressNamed(guest, 'ArrowRight');
    await pressNamed(guest, 'ArrowRight');
  }
  await typeText(guest, 'x');
  await settle(page, 8000);
  const marker = `${w.slice(0, 2)}x${w.slice(2)}`;
  const inMem = await memHas(guest, marker);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`caret by ${dir}: "${marker}" in the guest's memory ${inMem}`] };
}

/** Home moves the caret to the start (a letter typed there goes first); End moves it to the end (a letter typed there goes last) */
export async function caretKeys({ guest, page }, which) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  if (which === 'home') {
    await typeText(guest, w.slice(1));
    await pressNamed(guest, 'Home');
    await typeText(guest, w[0]);
  } else {
    await typeText(guest, w.slice(0, 3));
    await pressNamed(guest, 'Home');
    await pressNamed(guest, 'End');
    await typeText(guest, w[3]);
  }
  await settle(page, 8000);
  const inMem = await memHas(guest, w);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`${which}: "${w}" in the guest's memory ${inMem}`] };
}

/** Tab (or Shift+Tab) leaves the desk: the focus moves off #desktop, and the key does not reach the engine */
export async function tabCase({ guest, page }, shift) {
  await guest.focusDesktop();
  await sleep(200);
  const before = await activeId(page);
  const mark = await markSeq(page);
  if (shift) await shiftKey(guest, true);
  await pressNamed(guest, 'Tab', shift ? 8 : 0);
  if (shift) await shiftKey(guest, false);
  await sleep(500);
  const after = await activeId(page);
  const recs = await recordsSince(page, mark);
  const tabSent = hasKey(recs, 0x09, true) || hasKey(recs, 0x09, false);
  return { ok: before === 'desktop' && after !== 'desktop' && !tabSent, engineState: await stateOf(guest), notes: [`${shift ? 'Shift+' : ''}Tab: focus "${before}" -> "${after}"; the key reached the engine ${tabSent}`] };
}

/** Escape closes the File drop-down */
export async function escapeMenu({ guest }) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, 'File');
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const before = dropDown(await windowTable(guest), base) !== null;
  await guest.escape();
  await sleep(700);
  const after = dropDown(await windowTable(guest), base) !== null;
  const left = await closeFresh(guest, base);
  return { ok: before && !after && left.length === 0, engineState: await stateOf(guest), notes: [`drop-down open ${before}, after Escape ${after}`] };
}

/** Escape cancels the rename box: the box is shown before and gone after */
export async function escapeRename({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  await typeText(guest, word(4));
  await settle(page, 8000);
  await guest.escape();
  await sleep(800);
  const after = freshShown(await windowTable(guest), r.base).length;
  const left = await closeFresh(guest, r.base);
  return { ok: after === 0, engineState: await stateOf(guest), notes: [`the box shown before Escape ${r.fresh.length}; after Escape ${after}; shown after cleanup ${left.length}`] };
}

/** Enter commits the rename box (its text is in the guest's memory) and closes it; `empty`: the box with no text closes too */
export async function enterRename({ guest, page }, empty) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(5);
  if (!empty) await typeText(guest, w);
  await settle(page, 8000);
  const inMem = empty ? true : await memHas(guest, w);
  const closed = await commitRename(guest, r.base, page);
  await closeFresh(guest, r.base);
  return { ok: inMem && closed, engineState: await stateOf(guest), notes: [`${empty ? 'empty box' : `"${w}"`}: in the guest's memory ${inMem}; the box closed on Enter ${closed}`] };
}

/** the File drop-down, one item down, Enter: the item is chosen, its drop-down closes, the engine stays idle and the main window stays */
export async function enterMenuItem({ guest }) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, 'File');
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const had = dropDown(await windowTable(guest), base) !== null;
  await pressNamed(guest, 'ArrowDown');
  await sleep(300);
  await pressNamed(guest, 'Enter');
  await sleep(1500);
  const t = await windowTable(guest);
  const pops = freshShown(t, base).filter((w) => w.cls === POPUP).length;
  const main = windowsOf(t).some((w) => w.cls === MAIN_FORM);
  const st = await stateOf(guest);
  const left = await closeFresh(guest, base);
  return { ok: had && pops === 0 && main && st === 'engine-idle' && left.length === 0, engineState: st, notes: [`File drop-down open ${had}; Down, Enter: drop-downs left ${pops}; the main window ${main}; the engine ${st}; windows the item opened, closed after Escape: ${left.length === 0}`] };
}

/** Backspace deletes the last letter typed; the committed text is the rest */
export async function backspaceCase({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, `${w}k`);
  await pressNamed(guest, 'Backspace');
  await settle(page, 8000);
  await commitRename(guest, r.base, page);
  const inMem = await memHas(guest, w);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`"${w}k" then Backspace, Enter: "${w}" in the guest's memory ${inMem}`] };
}

/** Delete removes the letter after the caret */
export async function deleteCase({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, w);
  await pressNamed(guest, 'Home');
  await pressNamed(guest, 'Delete');
  await settle(page, 8000);
  await commitRename(guest, r.base, page);
  const inMem = await memHas(guest, w.slice(1));
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`"${w}", Home, Delete, Enter: "${w.slice(1)}" in the guest's memory ${inMem}`] };
}

/** a key held down: three key downs (the repeats are autoRepeat), so the caret moves three letters */
export async function repeatArrow({ guest, page }) {
  const r = await openRename(guest);
  if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const w = word(4);
  await typeText(guest, w);
  await holdKey(guest, 'ArrowLeft', 3);
  await typeText(guest, 'x');
  await settle(page, 8000);
  const marker = `${w[0]}x${w.slice(1)}`;
  const inMem = await memHas(guest, marker);
  await closeFresh(guest, r.base);
  return { ok: inMem, engineState: await stateOf(guest), notes: [`ArrowLeft held for 3 key downs, then x: "${marker}" in the guest's memory ${inMem}`] };
}

/** the Down key held for three key downs moves the drop-down's highlight as three separate presses do */
export async function repeatMenu({ guest, page }) {
  const base = lidsOf(await windowTable(guest));
  const p = await labelPoint(guest, 'File');
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base), 3000);
  const popA = dropDown(await windowTable(guest), base);
  if (!popA) return fail(guest, ['no File drop-down']);
  for (let i = 0; i < 3; i++) { await pressNamed(guest, 'ArrowDown'); await sleep(250); }
  await sleep(400);
  const hA = await pictureSettled(guest, popA.rect, 3000);
  await closeFresh(guest, base);
  const base2 = lidsOf(await windowTable(guest));
  await press(guest, Math.floor(p.x), Math.floor(p.y));
  await waitFor(async () => dropDown(await windowTable(guest), base2), 3000);
  const popB = dropDown(await windowTable(guest), base2);
  if (!popB) return fail(guest, ['no File drop-down the second time']);
  const mark = await markSeq(page);
  await holdKey(guest, 'ArrowDown', 3);
  await settle(page, 8000);
  await sleep(400);
  const hB = await pictureSettled(guest, popB.rect, 3000);
  const downs = countKey((await recordsSince(page, mark)), 0x28, true);
  await closeFresh(guest, base2);
  return { ok: hA === hB && downs >= 3, engineState: await stateOf(guest), notes: [`three Down presses: the picture ${hA === hB ? 'the same as' : 'differs from'} the held key's (${downs} Down key downs processed)`] };
}

/** the IME composition (Input.imeSetComposition, Input.insertText) on the rename box: the text should reach the engine (compose, convert, commit), or a cancelled composition leaves none (cancel) */
export async function imeCase({ guest, page }, mode) {
  const r = await openRename(guest);
  const notes = [];
  const step = async (fn, what) => { try { await fn(); } catch (e) { notes.push(`${what} refused: ${String(e?.message ?? e).slice(0, 120)}`); } };
  const kana = Array.from({ length: 6 }, () => String.fromCharCode(0x30a1 + Math.floor(Math.random() * 85))).join('');
  const target = mode === 'compose' ? 'にほん' : kana;
  const before = await memHas(guest, target);
  let during = null;
  if (mode === 'compose') await step(() => imeSet(guest, 'にほん'), 'composition');
  if (mode === 'convert') { await step(() => imeSet(guest, kana.slice(0, 3)), 'composition'); await step(() => imeSet(guest, target), 'conversion'); }
  if (mode === 'commit') await step(() => imeInsert(guest, target), 'commit');
  if (mode === 'cancel') {
    await step(() => imeSet(guest, target), 'composition');
    await settle(page, 4000);
    during = await memHas(guest, target);
    await step(() => imeSet(guest, ''), 'cancel');
  }
  await settle(page, 6000);
  await sleep(400);
  const inMem = await memHas(guest, target);
  const left = await closeFresh(guest, r.base);
  const ok = mode === 'cancel' ? during === true && !inMem : !before && inMem;
  return { ok, engineState: await stateOf(guest), notes: [...notes, `${mode}: "${target}" in the guest's memory before ${before}${during === null ? '' : `, during the composition ${during}`}, after ${inMem}; the box was shown ${r.fresh.length > 0}; shown after cleanup ${left.length}`] };
}

/** the numeric keypad's keys go down and up (NumLock decides the digits: the digits are not read here) */
export async function numpadCase({ guest, page }) {
  await guest.focusDesktop();
  const mark = await markSeq(page);
  for (const d of [1, 2, 3]) await guest.key(`Numpad${d}`, String(d), 0x60 + d);
  await settle(page, 8000);
  await sleep(300);
  const recs = await recordsSince(page, mark);
  const pairs = [1, 2, 3].map((d) => hasKey(recs, 0x60 + d, true) && hasKey(recs, 0x60 + d, false));
  return { ok: pairs.every(Boolean) && (await stateOf(guest)) === 'engine-idle', engineState: await stateOf(guest), notes: [`Numpad1-3 each go down and up at the engine: ${pairs.join(', ')}`] };
}

/** a key goes down and up (F12): both reach the engine */
export async function releaseCase({ guest, page }) {
  await guest.focusDesktop();
  const mark = await markSeq(page);
  await fkeyPress(guest, 12);
  await settle(page, 8000);
  await sleep(400);
  const recs = await recordsSince(page, mark);
  const down = hasKey(recs, 0x7b, true), up = hasKey(recs, 0x7b, false);
  return { ok: down && up, engineState: await stateOf(guest), notes: [`F12 key down ${down}, key up ${up} processed`] };
}

/** Ctrl+N (File > New by its key) on the empty project: the engine takes the key, no box comes up, and the desk stays as it is */
export async function ctrlNCase({ guest, page }) {
  await guest.focusDesktop();
  const base = lidsOf(await windowTable(guest));
  const set0 = await shownSet(guest);
  const h0 = await pictureSettled(guest);
  const mark = await markSeq(page);
  await guest.ctrl('KeyN', 'n', 78);
  await settle(page, 8000);
  await sleep(1500);
  const recs = await recordsSince(page, mark);
  const h1 = await pictureSettled(guest);
  const set1 = await shownSet(guest);
  const left = await closeFresh(guest, base);
  const sent = hasKey(recs, 0x4e, true);
  const extra = [...set1].filter((k) => !set0.has(k)).slice(0, 3).join(' | '), lost = [...set0].filter((k) => !set1.has(k)).slice(0, 3).join(' | ');
  return { ok: sent && h1 === h0 && left.length === 0, engineState: await stateOf(guest), notes: [`Ctrl+N: the key reached the engine ${sent}; the desk ${h1 === h0 ? 'unchanged' : 'changed'}; boxes shown ${left.length}; windows shown since: ${extra || '-'}; no longer shown: ${lost || '-'}`] };
}

// ---- the clipboard (the page's Clipboard API; the permission is set for the page's origin for each scenario) -------------------------

/** Ctrl+C copies the selected rename text (Ctrl+A first): the page gets the text; the read back is the text when the permission is granted */
export async function clipCopy(ctx, setting) {
  const { guest, page } = ctx;
  await clipCounter(page);
  await setClip(ctx, setting);
  try {
    const r = await openRename(guest);
    if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
    const w = word();
    await typeText(guest, w);
    await settle(page, 8000);
    const c0 = await clipCounts(page);
    await guest.ctrl('KeyA', 'a', 0x41);
    await guest.ctrl('KeyC', 'c', 0x43);
    await settle(page, 8000);
    await sleep(600);
    const c1 = await clipCounts(page);
    const rec = await clipRecord(page);
    const readback = setting === 'denied' ? null : await clipReadFromPage(page);
    await closeFresh(guest, r.base);
    const writes = c1.writeText - c0.writeText;
    const ok = setting === 'granted' ? rec.lastWritten === w && readback === w
      : setting === 'denied' ? rec.lastWritten === w && rec.refusal === 'clipboard-write: denied' && writes === 0
        : rec.lastWritten === w && (readback === w || rec.refusal !== null);
    return { ok, engineState: await stateOf(guest), notes: [`copy under ${setting}: the page got the text ${rec.lastWritten === w}; writeText calls ${writes}; read back ${readback === w ? 'the text' : readback === null ? 'nothing' : 'another text'}; refusal ${rec.refusal ?? 'none'}`], results: [{ setting, writes, refusal: rec.refusal, readPermission: rec.readPermission, writePermission: rec.writePermission }] };
  } finally {
    await setClip(ctx, 'prompt');
  }
}

/** Ctrl+V pastes the clipboard text into the rename box: the text is seeded by the page while the permission is granted, then the scenario's permission applies */
export async function clipPaste(ctx, setting) {
  const { guest, page } = ctx;
  await clipCounter(page);
  const w = word();
  try {
    await setClip(ctx, 'granted');
    const seeded = await clipWriteFromPage(page, w);
    await setClip(ctx, setting);
    const r = await openRename(guest);
    if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
    const c0 = await clipCounts(page);
    await guest.ctrl('KeyV', 'v', 0x56);
    await settle(page, 8000);
    await sleep(600);
    const c1 = await clipCounts(page);
    const rec = await clipRecord(page);
    const inMem = await memHas(guest, w);
    await closeFresh(guest, r.base);
    const reads = c1.readText - c0.readText;
    const ok = setting === 'granted' ? inMem && reads >= 1
      : setting === 'denied' ? !inMem && reads === 0 && rec.refusal === 'clipboard-read: denied'
        : reads >= 1 && (inMem || rec.refusal !== null);
    return { ok, engineState: await stateOf(guest), notes: [`paste under ${setting}: the seed ${seeded === true ? 'written' : `refused (${seeded})`}; readText calls ${reads}; the text in the guest's memory ${inMem}; refusal ${rec.refusal ?? 'none'}`], results: [{ setting, reads, refusal: rec.refusal, readPermission: rec.readPermission }] };
  } finally {
    await setClip(ctx, 'prompt');
  }
}

/** Ctrl+X cuts the selected text: the page gets it (granted: the read back is the text) */
export async function clipCut(ctx) {
  const { guest, page } = ctx;
  await clipCounter(page);
  try {
    await setClip(ctx, 'granted');
    const r = await openRename(guest);
    if (r.fresh.length === 0) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
    const w = word();
    await typeText(guest, w);
    await settle(page, 8000);
    await guest.ctrl('KeyA', 'a', 0x41);
    await guest.ctrl('KeyX', 'x', 0x58);
    await settle(page, 8000);
    await sleep(600);
    const rec = await clipRecord(page);
    const readback = await clipReadFromPage(page);
    await closeFresh(guest, r.base);
    return { ok: rec.lastWritten === w && readback === w, engineState: await stateOf(guest), notes: [`cut under granted: the page got "${w}" ${rec.lastWritten === w}; read back ${readback === w}`] };
  } finally {
    await setClip(ctx, 'prompt');
  }
}

/** the permission changes between two pastes: under prompt the page asks for the text (readText is called); granted, the next paste gets the text */
export async function clipPermChange(ctx) {
  const { guest, page } = ctx;
  await clipCounter(page);
  const w = word();
  try {
    await setClip(ctx, 'granted');
    await clipWriteFromPage(page, w);
    await setClip(ctx, 'prompt');
    const p0 = await permissionPair(page);
    const r1 = await openRename(guest);
    if (r1.fresh.length === 0) { await closeFresh(guest, r1.base); return fail(guest, ['no box after F2']); }
    const a0 = await clipCounts(page);
    await guest.ctrl('KeyV', 'v', 0x56);
    await settle(page, 8000);
    await sleep(600);
    const a1 = await clipCounts(page);
    const p1 = await permissionPair(page);
    await closeFresh(guest, r1.base);
    await setClip(ctx, 'granted');
    const p2 = await permissionPair(page);
    const r2 = await openRename(guest);
    if (r2.fresh.length === 0) { await closeFresh(guest, r2.base); return fail(guest, ['no box after F2 the second time']); }
    const b0 = await clipCounts(page);
    await guest.ctrl('KeyV', 'v', 0x56);
    await settle(page, 8000);
    await sleep(600);
    const b1 = await clipCounts(page);
    const inMem = await memHas(guest, w);
    await closeFresh(guest, r2.base);
    const readsPrompt = a1.readText - a0.readText, readsGranted = b1.readText - b0.readText;
    return { ok: readsGranted >= 1 && inMem, engineState: await stateOf(guest), notes: [`prompt: readText calls ${readsPrompt}, permission ${p0.read} -> ${p1.read}; granted: readText calls ${readsGranted}, permission ${p2.read}; the text ${inMem ? 'pasted' : 'not pasted'}`] };
  } finally {
    await setClip(ctx, 'prompt');
  }
}

const LIVE = `(() => { const exempt = ['status', 'audio-status', 'taskbar']; const shown = []; for (const el of document.querySelectorAll('[role="status"],[role="alert"],[aria-live]')) { if (exempt.includes(el.id)) continue; if (!el.textContent.trim()) continue; const cs = getComputedStyle(el); const r = el.getBoundingClientRect(); if (el.hidden || cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0 || r.width <= 0 || r.height <= 0) continue; shown.push(el.id || el.getAttribute('role') || el.tagName); } return { shown, statusElement: !!document.querySelector('#clipboard-status') }; })()`;

/** under denied, a paste and a copy show no notice: no visible live region with text, and no #clipboard-status element */
export async function clipNotice(ctx) {
  const { guest, page } = ctx;
  try {
    await setClip(ctx, 'denied');
    await guest.focusDesktop();
    const samples = [];
    const sample = async (at) => samples.push({ at, ...(await page.evaluate(LIVE)) });
    await sample('before');
    await guest.ctrl('KeyV', 'v', 0x56);
    await sleep(400);
    await sample('after Ctrl+V');
    await guest.ctrl('KeyC', 'c', 0x43);
    await sleep(400);
    await sample('after Ctrl+C');
    await sleep(3000);
    await sample('later');
    const shownAny = samples.some((s) => s.shown.length > 0);
    const statusEl = samples.some((s) => s.statusElement);
    return { ok: !shownAny && !statusEl, engineState: await stateOf(guest), notes: [`visible notices per sample: ${samples.map((s) => `${s.at} ${s.shown.length}`).join(', ')}; #clipboard-status ${statusEl ? 'present' : 'absent'}`] };
  } finally {
    await setClip(ctx, 'prompt');
  }
}

// ---- the browser's context menu and the hint bar -----------------------------------------------------------------------

const CTX_INSTALL = `(() => { if (window.__e2eInputCtx) return true; const c = { seen: 0, prevented: 0 }; window.__e2eInputCtx = c; document.addEventListener('contextmenu', (e) => { c.seen++; if (e.defaultPrevented) c.prevented++; }, true); return true; })()`;
const CTX_READ = `(() => (window.__e2eInputCtx ? { ...window.__e2eInputCtx } : { seen: 0, prevented: 0 }))()`;
const REGIONS = { canvas: { sel: '#desktop canvas[data-desk="1"]', real: true }, desktop: { sel: '#desktop', real: true }, toolbar: { sel: '#toolbar', real: false }, sidebar: { sel: 'aside', real: false } };

/** the browser's own context menu is cancelled in a region: its right presses (or its contextmenu events, where the region is hidden) all come with defaultPrevented */
export async function ctxSuppressed({ guest, page }, region) {
  await page.evaluate(CTX_INSTALL);
  const R = REGIONS[region];
  const base = lidsOf(await windowTable(guest));
  const before = await page.evaluate(CTX_READ);
  const box = await page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return { shown: cs.display !== 'none' && cs.visibility !== 'hidden' && r.width > 0 && r.height > 0, x: r.left, y: r.top, w: r.width, h: r.height };
  }, R.sel);
  const N = 6;
  let mode = 'synthetic';
  if (R.real && box && box.shown) {
    mode = 'real';
    for (let i = 0; i < N; i++) {
      const x = box.x + (0.1 + Math.random() * 0.8) * box.w, y = box.y + (0.1 + Math.random() * 0.8) * box.h;
      await guest.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0, pointerType: 'mouse' });
      await guest.send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'right', buttons: 2, clickCount: 1, pointerType: 'mouse' });
      await guest.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'right', buttons: 0, clickCount: 1, pointerType: 'mouse' });
      await sleep(120);
    }
  } else if (box) {
    await page.evaluate((sel) => { const el = document.querySelector(sel); if (el) for (let i = 0; i < 6; i++) el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, button: 2, clientX: 1, clientY: 1 })); }, R.sel);
  }
  await sleep(400);
  const after = await page.evaluate(CTX_READ);
  const events = after.seen - before.seen, prevented = after.prevented - before.prevented;
  const left = await closeFresh(guest, base);
  return { ok: box !== null && events >= 1 && prevented === events && (mode === 'real' || events === N), engineState: await stateOf(guest), notes: [`${region} (${mode}): contextmenu events ${events}, cancelled ${prevented}; FL's windows shown after cleanup ${left.length}`] };
}

/** the top bar's hint bar: the pointer resting on the bar shows the hint, and the bar's picture changes; the pointer's leaving brings the picture back */
export async function hintCase({ guest, page }, mode, side = 'left') {
  const bar = findWin(await windowTable(guest), (w) => w.cls === 'TQuickControlBar' && w.vis !== false);
  if (!bar || !bar.rect) return fail(guest, ['no top bar (TQuickControlBar) in the window table']);
  const r = bar.rect;
  const d = await guest.desk();
  const away = { x: Math.floor(d.gw * 0.5), y: Math.floor(d.gh * 0.92) };
  await guest.move(await guest.pointAt(away.x, away.y));
  await sleep(1500);
  const base = (await snap(guest, r)).hash;
  const fracs = side === 'right' ? [0.92, 0.8, 0.7, 0.6] : [0.08, 0.2, 0.35, 0.5, 0.65];
  let shownAt = null;
  for (const f of fracs) {
    await guest.move(await guest.pointAt(Math.floor(r.x + f * r.w), Math.floor(r.y + r.h * 0.5)));
    await sleep(1600);
    if ((await snap(guest, r)).hash !== base) { shownAt = f; break; }
  }
  let restored = null;
  if (shownAt !== null) {
    await guest.move(await guest.pointAt(away.x, away.y));
    await sleep(1600);
    restored = (await snap(guest, r)).hash === base;
  }
  const ok = mode === 'shows' ? shownAt !== null : shownAt !== null && restored === true;
  return { ok, engineState: await stateOf(guest), notes: [`top bar ${r.w}x${r.h}: ${shownAt === null ? 'no pixel of the bar changed while the pointer rested on it' : `the pointer at ${shownAt} of its width changed the bar's picture`}; after the pointer left: ${restored === null ? 'not checked' : restored ? 'the picture is as before' : 'the picture differs'}`] };
}

// ---- the cursor ----------------------------------------------------------------------------------------------------

/** the pointer over the rename box (a text field): the cursor is the I-beam ('text') */
export async function cursorText({ guest, page }) {
  const r = await openRename(guest);
  const box = r.fresh.find((w) => w.rect && w.rect.w > 20 && w.rect.h > 8);
  if (!box) { await closeFresh(guest, r.base); return fail(guest, ['no box after F2']); }
  const x = Math.floor(box.rect.x + box.rect.w / 2), y = Math.floor(box.rect.y + box.rect.h / 2);
  const css = await cursorAt(guest, page, x, y);
  await closeFresh(guest, r.base);
  return { ok: css === 'text', engineState: await stateOf(guest), notes: [`the pointer over the box (${box.cls}, ${box.rect.w}x${box.rect.h}) at ${x},${y}: cursor ${css}`] };
}

/** the window edges of a sizable window: the cursor over each edge (2 px inside) is the resize cursor of its direction (the window must be on top there) */
export async function cursorEdges({ guest, page }, dirs) {
  const base = lidsOf(await windowTable(guest));
  const resizable = (desk) => windowsOf(desk.table).filter((x) => x.cls === 'TMIDIForm' && x.vis !== false && x.rect && x.rect.w >= 150 && x.rect.h >= 150);
  let desk = await deskNow(guest);
  if (resizable(desk).length === 0) {
    await guest.focusDesktop();
    await fkeyPress(guest, 10);
    await waitFor(async () => (resizable(await deskNow(guest)).length > 0 ? true : null), 5000);
    desk = await deskNow(guest);
  }
  const cands = resizable(desk);
  if (cands.length === 0) {
    await closeFresh(guest, base);
    return fail(guest, ['no resizable window (the Settings window, F10) in the window table']);
  }
  const w = cands[0];
  const root = rootOf(desk.table, w.lid)?.lid ?? 0;
  const rows = [];
  const measure = async (dir, requested) => {
    const p = edgePoint(w.rect, dir);
    const covered = topLidAt(desk.table.top, p.x, p.y) !== root;
    const css = await cursorAt(guest, page, p.x, p.y);
    const pic = css.startsWith('image-set(') ? (await appCursor(page)).picture : null;
    const id = pic ? `picture ${createHash('sha1').update(css).digest('hex').slice(0, 8)}` : css;
    rows.push({ dir, at: `${p.x},${p.y}`, css: id, expected: EDGE_CSS[dir], covered, requested, match: css === EDGE_CSS[dir] || pic !== null });
  };
  for (const dir of dirs) await measure(dir, true);
  const refs = ['N', 'E'].filter((d) => !dirs.includes(d));
  for (const dir of refs) await measure(dir, false);
  await closeFresh(guest, base);
  const at = (d) => rows.find((r) => r.dir === d)?.css;
  const pairs = [['N', 'S'], ['E', 'W'], ['NE', 'SW'], ['NW', 'SE']].filter(([a, b]) => at(a) !== undefined && at(b) !== undefined);
  const pairsSame = pairs.every(([a, b]) => at(a) === at(b));
  const distinct = new Set(pairs.map(([a]) => at(a))).size === pairs.length;
  const diagonals = rows.filter((r) => r.dir.length === 2);
  const diagOk = diagonals.every((r) => r.css !== at('N') && r.css !== at('E'));
  const vhOk = at('N') === undefined || at('E') === undefined || at('N') !== at('E');
  const pairNote = pairs.length ? `; opposite edges share a cursor ${pairsSame}, the pairs differ ${distinct}` : '';
  const refNote = diagonals.length && refs.length ? `; the diagonals differ from N and E (${diagOk})` : '';
  const requestedRows = rows.filter((r) => r.requested);
  return { ok: requestedRows.every((r) => r.match && !r.covered) && pairsSame && (pairs.length < 2 || distinct) && diagOk && vhOk, engineState: await stateOf(guest), notes: [`${w.child ? 'child' : 'top-level'} ${w.cls} "${w.label}" ${w.rect.w}x${w.rect.h} at ${w.rect.x},${w.rect.y}: ${rows.map((r) => `${r.dir} ${r.css}${r.match ? '' : ` (expected ${r.expected})`}${r.covered ? ' covered' : ''}${r.requested ? '' : ' (reference)'}`).join('; ')}${pairNote}${refNote}; N and E differ ${vhOk}`], results: rows };
}

/** the Piano roll's pencil (the Draw tool): the cursor is a picture (image-set) at the page's device pixel ratio; `size` also checks the picture's CSS size and hot spot */
export async function cursorPencil({ guest, page }, mode) {
  const w = await ensurePianoRoll(guest);
  if (!w) return fail(guest, ['no Piano roll window (F7)']);
  let css = '';
  let pressedD = false;
  let tries = 0;
  for (const [fx, fy] of PENCIL_POINTS) {
    tries++;
    css = await cursorAt(guest, page, Math.floor(w.rect.x + fx * w.rect.w), Math.floor(w.rect.y + fy * w.rect.h), 2500);
    if (css.includes('image-set(')) break;
    if (!pressedD) { await guest.focusDesktop(); await guest.key('KeyD', 'd', 0x44); pressedD = true; await sleep(500); }
  }
  const dpr = await page.evaluate(() => window.devicePixelRatio);
  const app = await appCursor(page);
  const pic = app.picture;
  if (!css.includes('image-set(') || !pic) return { ok: false, engineState: await stateOf(guest), notes: [`no picture cursor after ${tries} points (D pressed ${pressedD}): cursor ${css.slice(0, 60)}; the page: ${app.why}`] };
  const cssW = Math.ceil(pic.width / dpr), cssH = Math.ceil(pic.height / dpr);
  const hot = `${Math.round(pic.hotX / dpr)} ${Math.round(pic.hotY / dpr)}, default`;
  const shown = css.replace(/url\("[^"]*"\)/, 'url(…)');
  const shownDpr = Number(/image-set\(url\(…\) ([\d.]+)x\)/.exec(shown)?.[1]);
  const dprOk = Number.isFinite(shownDpr) && Math.abs(shownDpr - dpr) < 0.01;
  const hotOk = shown.endsWith(`) ${hot}`);
  const sizeOk = cssW >= 1 && cssH >= 1 && cssW <= 128 && cssH <= 128;
  const ok = mode === 'css' ? dprOk && hotOk : dprOk && hotOk && sizeOk;
  return { ok, engineState: await stateOf(guest), notes: [`pencil picture ${pic.width}x${pic.height} (source ${pic.source}) at device pixel ratio ${dpr}: image-set at ${dpr}x ${dprOk}; CSS ${cssW}x${cssH} (at most 128: ${sizeOk}); hot spot ${hot} ${hotOk}; the element's cursor ${shown.slice(0, 120)}; the page's record ${app.css.replace(/url\("[^"]*"\)/, 'url(…)').slice(0, 120)}`], results: [{ dpr, width: pic.width, height: pic.height, cssW, cssH, hot }] };
}

// ---- touch ---------------------------------------------------------------------------------------------------------

/** a tap (touch down and up) on a place: a left button down and up reach the engine; on a menu label its drop-down opens */
export async function touchTap({ guest, page }, where) {
  await touchOn(guest, true);
  try {
    let p;
    if (where === 'desk') {
      const d = await guest.desk();
      p = { x: Math.floor(d.gw * 0.5), y: Math.floor(d.gh * 0.5) };
    } else {
      const q = await labelPoint(guest, where);
      p = { x: Math.floor(q.x), y: Math.floor(q.y) };
    }
    const base = lidsOf(await windowTable(guest));
    const c = await guest.pointAt(p.x, p.y);
    const mark = await markSeq(page);
    await touchSend(guest, 'touchStart', [c]);
    await sleep(80);
    await touchSend(guest, 'touchEnd', []);
    await settle(page, 8000);
    await sleep(800);
    const recs = await recordsSince(page, mark);
    const down = hasDesc(recs, btnDesc('left', 'down', p.x, p.y)), up = hasDesc(recs, btnDesc('left', 'up', p.x, p.y));
    const pop = where === 'desk' ? null : dropDown(await windowTable(guest), base);
    const left = await closeFresh(guest, base);
    return { ok: down && up && (where === 'desk' || pop !== null), engineState: await stateOf(guest), notes: [`tap at ${p.x},${p.y} (${where}): button down ${down}, up ${up}; drop-down ${pop ? 'opened' : 'none'}; shown after cleanup ${left.length}`] };
  } finally {
    await touchOn(guest, false);
  }
}

/** a press held for 900 ms: the down and the up reach the engine, and no browser context menu shows (any contextmenu is cancelled) */
export async function touchLong({ guest, page }, where) {
  await touchOn(guest, true);
  try {
    await page.evaluate(CTX_INSTALL);
    let p;
    if (where === 'desk') {
      const d = await guest.desk();
      p = { x: Math.floor(d.gw * 0.5), y: Math.floor(d.gh * 0.5) };
    } else {
      const q = await labelPoint(guest, where);
      p = { x: Math.floor(q.x), y: Math.floor(q.y) };
    }
    const base = lidsOf(await windowTable(guest));
    const c = await guest.pointAt(p.x, p.y);
    const before = await page.evaluate(CTX_READ);
    const mark = await markSeq(page);
    await touchSend(guest, 'touchStart', [c]);
    await sleep(900);
    await touchSend(guest, 'touchEnd', []);
    await settle(page, 8000);
    await sleep(600);
    const after = await page.evaluate(CTX_READ);
    const recs = await recordsSince(page, mark);
    const down = hasDesc(recs, btnDesc('left', 'down', p.x, p.y)), up = hasDesc(recs, btnDesc('left', 'up', p.x, p.y));
    const notCancelled = (after.seen - before.seen) - (after.prevented - before.prevented);
    const left = await closeFresh(guest, base);
    return { ok: down && up && notCancelled === 0, engineState: await stateOf(guest), notes: [`long press at ${p.x},${p.y} (${where}): button down ${down}, up ${up}; contextmenu events ${after.seen - before.seen}, not cancelled ${notCancelled}; shown after cleanup ${left.length}`] };
  } finally {
    await touchOn(guest, false);
  }
}

/** a touch dragged from one place to another (8 moves): the down at the start, the moves and the up at the end reach the engine */
export async function touchDrag({ guest, page }, trace) {
  await touchOn(guest, true);
  try {
    if (trace) { await traceOn(guest); await sleep(300); }
    const d = await guest.desk();
    const a = { x: Math.floor(d.gw * 0.3), y: Math.floor(d.gh * 0.4) };
    const b = { x: Math.floor(d.gw * 0.42), y: Math.floor(d.gh * 0.5) };
    const lmark = await logMark(guest);
    const mark = await markSeq(page);
    await touchSend(guest, 'touchStart', [await guest.pointAt(a.x, a.y)]);
    for (let i = 1; i <= 8; i++) {
      const p = { x: Math.round(a.x + ((b.x - a.x) * i) / 8), y: Math.round(a.y + ((b.y - a.y) * i) / 8) };
      await touchSend(guest, 'touchMove', [await guest.pointAt(p.x, p.y)]);
      await sleep(40);
    }
    await touchSend(guest, 'touchEnd', []);
    await settle(page, 8000);
    await sleep(400);
    const recs = await recordsSince(page, mark);
    const down = hasDesc(recs, btnDesc('left', 'down', a.x, a.y)), up = hasDesc(recs, btnDesc('left', 'up', b.x, b.y));
    const moved = recs.some((r) => r.processed && r.desc === moveDesc(b.x, b.y));
    const hitDown = trace ? (await hitsSince(guest, lmark)).some((h) => h.what === 'down' && h.btn === 0 && h.x === a.x && h.y === a.y) : null;
    return { ok: down && up && moved && (hitDown === null || hitDown), engineState: await stateOf(guest), notes: [`touch drag ${a.x},${a.y} -> ${b.x},${b.y}: down ${down}, a move at the end ${moved}, up ${up}${trace ? `; the engine's hit line of the down ${hitDown}` : ''}`] };
  } finally {
    await touchOn(guest, false);
  }
}

/** two fingers: the first finger's moves reach the engine; the second finger's are not sent (the page takes the primary pointer only) */
export async function touchPinch({ guest, page }) {
  await touchOn(guest, true);
  try {
    const d = await guest.desk();
    const A = { x: Math.floor(d.gw * 0.3), y: Math.floor(d.gh * 0.3) };
    const C = { x: Math.floor(d.gw * 0.6), y: Math.floor(d.gh * 0.6) };
    const path = Array.from({ length: 6 }, (_, i) => ({ x: A.x + (i + 1) * 20, y: A.y + (i + 1) * 10 }));
    const mark = await markSeq(page);
    await touchSend(guest, 'touchStart', [await guest.pointAt(A.x, A.y), await guest.pointAt(C.x, C.y)]);
    await sleep(60);
    for (let i = 0; i < path.length; i++) {
      const f1 = await guest.pointAt(path[i].x, path[i].y);
      const f2 = await guest.pointAt(C.x - (i + 1) * 20, C.y - (i + 1) * 10);
      await touchSend(guest, 'touchMove', [f1, f2]);
      await sleep(60);
    }
    await touchSend(guest, 'touchEnd', []);
    await settle(page, 8000);
    await sleep(400);
    const recs = await recordsSince(page, mark);
    const moves = recs.filter((r) => r.processed && r.desc.startsWith('move at screen ('));
    const first = new Set([moveDesc(A.x, A.y), ...path.map((p) => moveDesc(p.x, p.y))]);
    const onlyFirst = moves.every((r) => first.has(r.desc));
    const down = hasDesc(recs, btnDesc('left', 'down', A.x, A.y));
    return { ok: down && moves.length >= 3 && onlyFirst, engineState: await stateOf(guest), notes: [`two fingers: the first finger's down ${down}; moves processed ${moves.length}, all of the first finger ${onlyFirst}`] };
  } finally {
    await touchOn(guest, false);
  }
}
