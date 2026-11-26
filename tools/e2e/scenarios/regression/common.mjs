// The helpers of the regression scenarios (Issue #1408): the window table (top-level windows and their children, with absolute places), the start of
// the default project, the menus and the settings pages found by their captions, the picture's pixels, and the engine's audio numbers.
// Every position is in guest pixels, mapped to the page by lib/guest.mjs; a layout offset measured at DPR 1 is scaled by the guest px per CSS px (the DPR).
import { closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

export { sleep, WELCOME };
export const MAIN = 'TFruityLoopsMainForm';
export const MENU_BAR = 'TNewMenu';
export const POPUP = 'TQuickPopupMenuWindow';
export const SETTINGS = 'TMIDIForm';
export const TOOL_CHORD = 'TPRChordToolForm';
export const PIANO_ROLL = 'TEventEditForm';
export const NAME_EDIT = 'TNameEditForm';
// the first channel's name in the Channel rack, from the main form's top-left corner (as tools/clipboard, at DPR 1)
export const CHANNEL_NAME_PLACE = { x: 520, y: 144 };
// the issues the mark (expectFail) may name: the open issues of the defects this set's scenarios meet (the table of Issue #1408). A failure outside this list is
// not marked (run.mjs refuses a mark outside it, and 'new' is the mark of a defect with no issue yet). A closed issue is never a mark (#1375, #1384, #1385,
// #1387, #1389, #1391, #1392, #1402, #1406, #1421, #1482 are closed on main)
export const OPEN_ISSUES = ['#1386', '#1395', '#1419', '#1429', '#1433', '#1502', '#1503', '#1505'];

const LINE = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/;
/** the window table: one row per window or child, with the absolute place (ax, ay) of a child (its parent's place added) */
export function parseTable(text) {
  const rows = [];
  const childStyle = new Map();
  for (const l of String(text).split('\n')) {
    const cs = /^child style lid (\d+) 0x([0-9a-fA-F]+)\s*$/.exec(l);
    if (cs) { childStyle.set(Number(cs[1]), parseInt(cs[2], 16)); continue; }
    const m = LINE.exec(l);
    if (!m) continue;
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    const st = /style 0x([0-9a-fA-F]+)/.exec(m[8]);
    rows.push({
      child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]),
      cls: cls ? cls[1] : '', text: cap ? cap[1] : '', vis: m[1] ? /vis 1/.test(m[8]) : !/vis 0/.test(m[8]), style: st ? parseInt(st[1], 16) : null,
    });
  }
  for (const r of rows) if (r.child && childStyle.has(r.lid)) r.style = childStyle.get(r.lid);
  const by = new Map(rows.map((r) => [r.lid, r]));
  for (const r of rows) {
    let x = r.x, y = r.y, p = r.parent, guard = 0;
    while (r.child && p !== null && by.has(p) && guard++ < 64) { const q = by.get(p); x += q.x; y += q.y; p = q.parent; }
    r.ax = x; r.ay = y;
  }
  return rows;
}

/** the window table now (retries while the picture is being drawn: "ask again") */
export async function table(guest) {
  for (const t0 = Date.now(); ;) {
    try {
      return parseTable(await guest.ev('(async () => (await window.__app.deskDumpBase64(false)).windows)()'));
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
      await sleep(100);
    }
  }
}
export const tops = (t) => t.filter((w) => !w.child);
/** the children of a window (any depth), in table order */
export function under(t, lid) {
  const out = [];
  const walk = (id, g) => { for (const w of t) if (w.child && w.parent === id && g < 16) { out.push(w); walk(w.lid, g + 1); } };
  walk(lid, 0);
  return out;
}
/** the new top-level windows (not in the list of lids before) */
export const newTops = (t, before) => tops(t).filter((w) => !before.includes(w.lid));
export const lidsOf = (t) => t.map((w) => w.lid);

/** the guest size and the DPR's guest px per CSS px (1 at DPR 1; 2 at DPR 2) */
export async function geometry(guest) {
  return guest.ev(`(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height, dpr: window.devicePixelRatio }; })()`);
}
export async function kOf(guest) { const g = await geometry(guest); return g.gw / g.w; }

/** a click at guest pixels (gx, gy) on the window's place: fractions of its rectangle */
export async function clickIn(guest, w, fx, fy, opts = {}) {
  await guest.click(w.ax + fx * w.w, w.ay + fy * w.h, opts);
}
export async function clickAt(guest, gx, gy, opts = {}) { await guest.click(gx, gy, opts); }

/** waits until a window table row satisfies the test (or the time is up); returns the row or null */
export async function waitRow(guest, pred, ms) {
  const t0 = Date.now();
  for (;;) {
    const t = await table(guest);
    const hit = t.find(pred);
    if (hit) return { row: hit, t };
    if (Date.now() - t0 >= ms) return { row: null, t };
    await sleep(300);
  }
}

/** the clipboard permissions of the page's origin (a test aid: the page may read and write its clipboard, as the Copy and Paste scenarios need) */
export async function grantClipboard(browserCdp, browserContextId, origin) {
  for (const name of ['clipboard-read', 'clipboard-write']) {
    await browserCdp.send('Browser.setPermission', { permission: { name }, setting: 'granted', origin, browserContextId });
  }
}

/** the usual start: the first idle, the usage-data box and the Welcome closed (No thanks, then Escape: the Default project opens), the problems box closed */
export async function startDefault(guest, log) {
  const first = await waitStable(guest, 180000);
  if (first.state !== 'engine-idle') return { ok: false, state: first.state, title: first.title, notes: [`first state ${first.state}`] };
  if (await guest.findWindow((w) => w.cls === WELCOME)) {
    await noThanks(guest);
    await closeWelcome(guest);
  }
  const end = await waitProjectOpen(guest, first.title, 120000);
  if (!end.opened) return { ok: false, state: end.state, title: end.title, notes: ['the default project did not open'] };
  const notes = [];
  if (await dismissProblems(guest)) notes.push('the "Problems loading the project" box was closed');
  await closeRemix(guest);
  log(`default project open: "${end.title}" (idle after ${first.waitedMs} ms)`);
  return { ok: true, state: end.state, title: end.title, firstTitle: first.title, notes };
}

/** the "Remix a song" dialog of the default project takes the clicks of the rack: Escape closes it */
export async function closeRemix(guest) {
  const t = await table(guest);
  if (!tops(t).some((w) => w.cls === 'TDemucsForm')) return false;
  await guest.escape();
  await guest.waitFor(async () => (tops(await table(guest)).some((w) => w.cls === 'TDemucsForm') ? null : true), 5000, 'the Remix dialog closes');
  return true;
}

/** a menu-bar drop-down: the bar's label (fraction of the bar), then the drop-down window (new top-level); returns it or null */
export async function openMenuLabel(guest, fx, fy = 0.45) {
  const t0 = await table(guest);
  const bar = t0.find((w) => w.cls === MENU_BAR && w.child && w.ay < 60);
  if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
  const before = lidsOf(t0);
  await guest.click(bar.ax + fx * bar.w, bar.ay + fy * bar.h);
  for (let i = 0; i < 12; i++) {
    await sleep(300);
    const pop = newTops(await table(guest), before).find((w) => w.cls === POPUP);
    if (pop) return pop;
  }
  return null;
}

/** the settings window (F10): opened, and its General/other page chosen by the tab bar's slots (the caption of the visible sheet) */
export async function openSettings(guest) {
  await guest.key('F10', 'F10', 121);
  const r = await waitRow(guest, (w) => !w.child && w.cls === SETTINGS, 15000);
  if (!r.row) return null;
  return r.row;
}
/** the caption of the settings window's visible page (the TVectorSheet child) */
export function settingsPageOf(t, st) {
  return under(t, st.lid).find((w) => w.vis && w.cls === 'TVectorSheet') ?? null;
}
/** presses the tab bar's slots (ten of them) until the visible page's caption is `caption`; returns the settings window and the captions met */
export async function settingsPage(guest, caption) {
  const st0 = (await openSettings(guest)) ?? null;
  if (!st0) return { st: null, captions: [], why: 'the Settings window (TMIDIForm) did not open within 15 s of F10' };
  const captions = [];
  for (let s = 0; s < 10; s++) {
    const t = await table(guest);
    const st = tops(t).find((w) => w.lid === st0.lid);
    const bar = st ? under(t, st.lid).find((w) => w.cls === 'TQuickSheetSelector') : null;
    const page = st ? settingsPageOf(t, st) : null;
    captions.push(page ? page.text : null);
    if (page && page.text === caption) return { st, bar, page, captions, slot: s };
    if (!bar) return { st, captions, why: 'no tab bar (TQuickSheetSelector)' };
    await guest.click(bar.ax + ((s + 0.5) * bar.w) / 10, bar.ay + bar.h / 2);
    await sleep(400);
  }
  return { st: st0, captions, why: `the page "${caption}" was not reached (captions ${JSON.stringify(captions)})` };
}

/** the picture: the RGBA bytes with its size (deskDumpBase64 peek); the bytes are the page's compositor picture as it is */
export async function picture(guest) {
  const d = await guest.ev('(async () => { const d = await window.__app.deskDumpBase64(true); return { w: d.width, h: d.height, b64: d.base64, windows: d.windows }; })()');
  const bytes = Buffer.from(d.b64, 'base64');
  return { w: d.w, h: d.h, bytes, stride: bytes.length / d.h, windows: parseTable(d.windows) };
}
/** the pixels of a rectangle (guest px) that differ between two pictures (R, G, B), and the rectangle's area */
export function changedIn(a, b, r) {
  const x0 = Math.max(0, Math.floor(r.x)), y0 = Math.max(0, Math.floor(r.y));
  const x1 = Math.min(a.w, Math.ceil(r.x + r.w)), y1 = Math.min(a.h, Math.ceil(r.y + r.h));
  let n = 0, total = 0;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    total++;
    const i = y * a.stride + 4 * x, j = y * b.stride + 4 * x;
    if (a.bytes[i] !== b.bytes[j] || a.bytes[i + 1] !== b.bytes[j + 1] || a.bytes[i + 2] !== b.bytes[j + 2]) n++;
  }
  return { n, total };
}
/** the changed pixels of the whole picture that lie outside the rectangle */
export function changedOutside(a, b, r) {
  let n = 0;
  for (let y = 0; y < a.h; y++) {
    for (let x = 0; x < a.w; x++) {
      if (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h) continue;
      const i = y * a.stride + 4 * x, j = y * b.stride + 4 * x;
      if (a.bytes[i] !== b.bytes[j] || a.bytes[i + 1] !== b.bytes[j + 1] || a.bytes[i + 2] !== b.bytes[j + 2]) n++;
    }
  }
  return n;
}
/** the share of the rectangle's pixels that are black (the sum of R, G, B under 24: not painted) */
export function blankShare(a, r) {
  let n = 0, total = 0;
  for (let y = Math.max(0, Math.floor(r.y)); y < Math.min(a.h, r.y + r.h); y++) for (let x = Math.max(0, Math.floor(r.x)); x < Math.min(a.w, r.x + r.w); x++) {
    total++;
    const i = y * a.stride + 4 * x;
    if (a.bytes[i] + a.bytes[i + 1] + a.bytes[i + 2] < 24) n++;
  }
  return total === 0 ? 0 : n / total;
}
/** the SHA-256 of the whole picture (the page's own count) */
export const pictureHash = (guest) => guest.ev('window.__app.deskDumpHash().then((d) => d.sha256)');

/** the engine's audio numbers (the page's output): the underrun quanta and the quanta played; err is the output's own error (null when none) */
export const audioNow = (guest) => guest.ev('(() => { const a = window.__app.audio; const d = a ? a.detail() : null; return d ? { ur: d.underrunQuanta, playing: d.quantaWhilePlaying, status: d.status, ctx: d.ctxState, since: d.underrunQuantaSinceFirstIdle, err: d.error } : null; })()');
/** a window that the user can size: WS_THICKFRAME, drawn, not maximised, not minimised (the window table's style) */
export const sizable = (w) => w.style !== null && (w.style & 0x40000) !== 0 && (w.style & (0x1000000 | 0x20000000)) === 0 && w.vis;
/** the engine's log lines (text) */
export const engineLines = (guest) => guest.ev('window.__app.engineLog().map((l) => String(l))');
/** the window-API trace on (a test aid: the engine's log lines of the clipboard and the registry) */
export const traceOn = (guest) => guest.ev('(() => { window.__app.windowApiTrace(true); return true; })()');
/** the page's rAF-style counter */
export const engineState = (guest) => guest.state();

/** the result of a scenario: ok, the engine's state at the end, the notes, and the measured values (results, written to the JSON as they are) */
export function result(ok, state, notes, extra = {}) {
  return { ok: ok === true, engineState: state ?? null, notes, results: extra };
}
export const rect = (w) => ({ x: w.ax, y: w.ay, w: w.w, h: w.h });
