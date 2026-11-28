// piano-roll-tools: the measurement of the Piano roll's tools (#1386 chord progression, #1387 riff machine, #1430 the riff machine's Accept). It opens the default project, the Piano roll (F7), the main window's TOOLS menu and one tool; for the chord progression it presses "Start creating"; for the riff machine it searches the Accept.
// It records and measures only: the product is not changed, and no branch for the measurement is in it.
// Usage: node tools/piano-roll-tools/measure.mjs <absolute path to the bundle directory raw/<name>> --tool <chord-progression|riff-machine> --dprs <list> [--log <file>] [--shot <dir>] > tools.jsonl
//   --log <file>: the engine's whole log after each run, appended with a "# dpr" line (logLengthAtItem in the JSON is the log's length when the item was chosen).
//   --shot <dir>: the desk's picture as a PNG: <tool>-dpr<dpr>-opened.png (after the item, the Warning box closed, before the button), <tool>-dpr<dpr>-before-accept.png and <tool>-dpr<dpr>-after.png (riff machine: the picture just before and just after the press that closed the window with notes). For reading the screen by eye (no OCR).
//   --dprs: a comma-separated list of devicePixelRatio values; one start of the bundle (a new Chrome, a new engine) per value, one JSON line per value on stdout (JSONL). stderr: one line per step.
//   Needs app/node_modules and app/dist, as boot-bundle.mjs does. Exit 2 when the arguments are wrong, 1 when the tool itself failed (the JSON line then has "error"), 0 otherwise.
// The start: bootBundle (tools/boot-check/boot-bundle.mjs) with the dpr, and afterIdle (at the engine's first idle): the room is set to 1836 x 1056 CSS px (as tools/smoke/welcome-flow.mjs sets it), Escape closes the
//   Welcome window (the default project opens, as in welcome-flow's flow B), and Escape closes the "Problems loading the project" box that the default project shows (its buttons are not read).
// The places come from the window table (window.__app.deskDumpBase64: the top-level windows and the children, with class and caption):
//   - the Piano roll: the child window of class TEventEditForm whose caption starts with "Piano roll". F7 opens it for the selected channel of the default project.
//   - the TOOLS menu: the menu bar's TNewMenu (the top-left child), clicked at TOOLS_LABEL (a fraction of its rectangle: the label's place in the bar, measured from the picture at DPR 1).
//     Its drop-down is the new top-level TQuickPopupMenuWindow.
//   - the item: reached by Down presses (TOOL_ITEMS: the number of presses, the first item is 1 press), then Enter.
//   - "Start creating" (chord progression only): a button that the window table does not list. Its centre is START_CREATING, a fraction of the tool window's rectangle (the window is found by its class).
//   - riff machine: the item first opens a Warning box (TMsgForm; the guest's text is read with window.__app.guestText; the presets need Sytrus); Enter closes it (as a user does), and then the
//     window "Piano roll - riff machine" (TPRScoreCreatorForm) is the tool window.
//   - riff machine's Accept (#1430): the window has no place for it in the table and no fixed position in the code. searchAccept presses the centres of a grid over the window's rectangle (ACCEPT_GRID: the search's steps, not a place), one by one, each held down and released.
//     For each press: the window's rectangle is read while the button is held (pressedAt: the press changed the look of the window) and after the release (closed: the window is no longer in the table).
//     A press that closed the window is counted: countNotes(the picture before the press, the picture after it, the Piano roll's rectangle). Notes >= 1: that press is the Accept (found). Notes 0: a Cancel-like close, and the window is opened again for the next candidate.
//     The grid is scanned from its bottom-right corner (the order of the search only); a candidate that opens a popup gets Escape (a Warning box: Enter).
// Each JSON line: tool, dpr, error (the page's error, or the tool's failure; null when none), engine (the engine's state 3 s after the action), stopped (the state is stopped, exited or error, or a "== stop:" line is in the log),
//   presentedDelta (the desktop's presents in the 3 s after the action: window.__app.deskStats.presents), notes and noteRects (riff machine: the Piano roll's notes counted after the press that closed the window (acceptSearch.found); null when no candidate was found; null for the others), warnings (the texts of every Warning box that appeared, closed by the tool or not: [] when none), warningBox (the Warning box that came first, with its children and the guest's text near the markers; null when none), missingPaths (the missing-path: lines since the first idle),
//   acceptSearch (riff machine: { grid, candidates, tried (the candidates pressed), reopened (the window opened again after a close), pressedAt (the ratios of the candidates whose press changed the window's look), closed ({ fx, fy, notes } of each press that closed the window), popups, warnings, found ({ fx, fy } or null), foundWindows (the top-level windows after the Accept's press) }),
//   stopLines (the engine's stop lines: "== stop:" (also "returned from entry", which a run that goes on prints too), "fle:", "fruity: exit code", "runtime message:"), windowNew (the top-level windows that appeared with the action: { cls, title, x, y, w, h, visible, painted (its most common colour covers less than 95%), dominantShare, children }; null when none; children: { cls, text, vis, x, y, w, h } with x, y the guest's absolute position),
//   elapsedMs (the page's timeline: the first idle, in ms from the navigation), preBootBytes (the assetsReport bytes), nproc, loadavg, room (the guest's screen, window.__app.screen.guest),
//   logTail (the last 12 lines of the engine's log, in order: the order of the stop), steps (what the tool did, in order).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';

const ROOM_W = 1836, ROOM_H = 1056;
const VIEW_START = { width: 2176, height: 1092 };
const WELCOME_TITLE = 'Welcome to FL Studio - in ?trial mode?';
const PROBLEMS_CLASS = 'TMissingPluginsForm';
const PIANO_ROLL_CLASS = 'TEventEditForm';
const PIANO_ROLL_CAPTION = 'Piano roll';
const MENU_BAR_CLASS = 'TNewMenu';
const POPUP_CLASS = 'TQuickPopupMenuWindow';
const WARNING_CLASS = 'TMsgForm';
const KEY_ESCAPE = 27, KEY_ENTER = 13, KEY_DOWN = 40, KEY_F7 = 118;
// the label TOOLS in the menu bar, as fractions of the bar's rectangle (the 7th of its 8 labels)
const TOOLS_LABEL = { fx: 0.819, fy: 0.45 };
// the items of the TOOLS drop-down by the Down presses that reach them (the order: Browser find, One-click audio recording, Remix a song, Riff machine, Chord progression tool, ...)
const TOOL_ITEMS = {
  'chord-progression': { downs: 5, windowClass: 'TPRChordToolForm' },
  'riff-machine': { downs: 4, windowClass: 'TPRScoreCreatorForm' },
};
// "Start creating" in the chord progression tool's window: its centre as fractions of the window's rectangle (measured from the picture at DPR 1)
const START_CREATING = { fx: 0.5, fy: 0.847 };
// the riff machine's Accept is searched, not placed: the grid over the window whose centres are pressed (cols x rows presses at most; the ratios are the search's steps)
const ACCEPT_GRID = { cols: 12, rows: 32 };
const PRESS_HOLD_MS = 120;
const SETTLE_MS = 250;
const ACTION_WAIT_MS = 3000;
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;
// a "== stop:" line is a stop, except "returned from entry": the harness prints that one when the entry returns, in a run that goes on (idle)
// a guest thread that failed ("[js] fle thread N failed: ...", fle_thread_lib.js) is a stop too: the guest's threads stop answering
const STOP_LINE = /^(?:fruity: )?== stop: (?!returned from entry)|^\[js\] fle thread \d+ failed/;
const STOP_LINES = /^(?:fruity: == stop:|== stop:|fle: |fruity: exit code|fruity: runtime message|runtime message: |\[js\] fle thread \d+ failed)/;

const WINDOWS = `(async () => { for (let i = 0; ; i++) { try { return (await window.__app.deskDumpBase64(false)).windows; } catch (e) { if (!/ask again/.test(String(e?.message ?? e)) || i > 150) throw e; await new Promise((r) => setTimeout(r, 100)); } } })()`;
const PICTURE = `(async () => { const d = await window.__app.deskDumpBase64(true); return { width: d.width, height: d.height, base64: d.base64 }; })()`;
const STATE = `(() => { const a = window.__app; return { state: a.state, title: document.title, presents: a.deskStats && a.deskStats.presents !== undefined ? a.deskStats.presents : null, logLength: a.engineLog().length, error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 300) : null, guest: a.screen.guest }; })()`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const STAGE = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const READ_BYTES = `(() => (window.__app.assetsReport()?.items ?? []).reduce((n, i) => n + i.bytes, 0))()`;
const READ_IDLE = `(() => window.__app.timeline.idle ?? null)()`;
const LOG_ALL = `window.__app.engineLog()`;
// the text the guest's memory holds near the markers (window.__app.guestText, #1358): the first 60 places only
const GUEST_TEXT = (markers) => `(async () => { const r = await window.__app.guestText(${JSON.stringify(markers)}, 160); return { memBytes: r.memBytes, best: r.best, near: r.near.slice(0, 60) }; })()`;
// the words the Warning box of the riff machine may hold (the guest's text is read only where a marker is found)
const WARNING_MARKERS = ['Sytrus', 'plugin', 'Plugin', 'missing', 'Missing', 'riff', 'Riff', 'not found', 'Warning'];
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;
const GUEST_FAILED = `window.__app.engineLog().slice(-30).some((l) => /fle thread \\d+ failed/.test(l))`;

const log = (s) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${s}\n`);
const round3 = (v) => Math.round(v * 1000) / 1000;

/** the windows of a window-table text: { child, lid, parent, x, y, w, h, cls, text, vis } (x, y are relative to the parent for a child) */
export function parseWindows(text) {
  return text.split('\n').map((l) => {
    const m = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (!m) return null;
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    // a top-level line has no "vis": the desk lists only the windows it draws (fle_desk.c build_desc), so such a line is visible
    return { child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]), cls: cls ? cls[1] : '', text: cap ? cap[1] : '', vis: m[1] ? /vis 1/.test(m[8]) : true };
  }).filter(Boolean);
}

/** the guest position of a window: a child's position is added to its parents' (the table gives a child's position relative to its parent) */
export function absolutePosition(list, win) {
  let x = win.x, y = win.y, cur = win;
  while (cur.child && cur.parent !== null) {
    const up = list.find((w) => w.lid === cur.parent);
    if (!up) break;
    x += up.x; y += up.y;
    cur = up;
  }
  return { x, y };
}

/** the child windows of one window (all levels) in a window list: { cls, text, vis, x, y, w, h } (x, y the guest's absolute position), in the table's order (at most 60) */
export function descendants(list, lid) {
  const out = [];
  const walk = (at) => {
    for (const c of list) {
      if (!c.child || c.parent !== at || out.length >= 60) continue;
      const p = absolutePosition(list, c);
      out.push({ cls: c.cls, text: c.text, vis: c.vis, x: p.x, y: p.y, w: c.w, h: c.h });
      walk(c.lid);
    }
  };
  walk(lid);
  return out;
}

/** a PNG (8-bit RGBA, no filter) of a picture as deskDumpBase64 gives it: base64 of the R, G, B, A bytes, row by row. For reading the screen by eye (no OCR) */
export function pngOf(picture) {
  const stride = picture.width * 4;
  const raw = Buffer.from(picture.base64, 'base64');
  const rows = Buffer.alloc((stride + 1) * picture.height);
  for (let y = 0; y < picture.height; y++) raw.copy(rows, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  const chunk = (type, data) => {
    const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const len = Buffer.alloc(4), crc = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    crc.writeUInt32BE(zlib.crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(picture.width, 0);
  ihdr.writeUInt32BE(picture.height, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(rows)), chunk('IEND', Buffer.alloc(0))]);
}

// a note is a connected part of the note colour at least this many pixels wide and high
const NOTE_MIN_PX = 4;
// a window is painted when its most common colour covers less than this share of its rectangle
const PAINTED_BELOW = 0.95;

const decoded = new WeakMap();
/** the rectangle of a picture (clipped to the picture) as { x0, y0, x1, y1, w, h, stride, bytes } (RGBA, 4 bytes a pixel), or null when nothing of it is inside */
function areaOf(picture, rect) {
  const stride = picture.width * 4;
  const x0 = Math.max(0, rect.x), y0 = Math.max(0, rect.y);
  const x1 = Math.min(picture.width, rect.x + rect.w), y1 = Math.min(picture.height, rect.y + rect.h);
  if (x1 <= x0 || y1 <= y0) return null;
  if (!decoded.has(picture)) decoded.set(picture, Buffer.from(picture.base64, 'base64'));
  return { x0, y0, x1, y1, w: x1 - x0, h: y1 - y0, stride, bytes: decoded.get(picture) };
}

/** the colours of an area with their pixel counts (a colour is the RGBA word that readUInt32BE gives) */
function colourCounts(area) {
  const counts = new Map();
  for (let y = area.y0; y < area.y1; y++) {
    let at = y * area.stride + area.x0 * 4;
    for (let x = area.x0; x < area.x1; x++, at += 4) {
      const c = area.bytes.readUInt32BE(at);
      counts.set(c, (counts.get(c) ?? 0) + 1);
    }
  }
  return counts;
}

/** the share (0 .. 1) of a picture's rectangle that its most common colour covers, and that colour (0 share when the rectangle is empty) */
export function dominantShare(picture, rect) {
  const area = areaOf(picture, rect);
  if (!area) return { share: 0, colour: null };
  let colour = null, n = 0;
  for (const [c, k] of colourCounts(area)) if (k > n) { n = k; colour = c; }
  return { share: n / (area.w * area.h), colour };
}

/** a hash of a picture's rectangle (clipped to the picture; null when nothing of it is inside): the same pixels give the same hash. The look of a window before and during a press */
export function areaHash(picture, rect) {
  const area = areaOf(picture, rect);
  if (!area) return null;
  const h = crypto.createHash('sha256');
  for (let y = area.y0; y < area.y1; y++) h.update(area.bytes.subarray(y * area.stride + area.x0 * 4, y * area.stride + area.x1 * 4));
  return h.digest('hex');
}

/** the notes of the Piano roll in a rectangle (guest pixels): the 4-connected parts of the note colour in `after` that are NOTE_MIN_PX wide and high.
 *  The note colour is measured, not a table: the most common colour of `after`'s rectangle that `before`'s rectangle does not have (noteColour null when there is none). */
export function countNotes(before, after, rect) {
  const A = areaOf(after, rect);
  if (!A) return { notes: 0, noteRects: [], noteColour: null };
  const B = areaOf(before, rect);
  const beforeColours = B ? colourCounts(B) : new Map();
  let colour = null, n = 0;
  for (const [c, k] of colourCounts(A)) if (!beforeColours.has(c) && k > n) { n = k; colour = c; }
  if (colour === null) return { notes: 0, noteRects: [], noteColour: null };
  const w = A.w, h = A.h;
  const mask = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (A.bytes.readUInt32BE((A.y0 + y) * A.stride + (A.x0 + x) * 4) === colour) mask[y * w + x] = 1;
  const seen = new Uint8Array(w * h);
  const stack = new Int32Array(w * h);
  const noteRects = [];
  for (let start = 0; start < w * h; start++) {
    if (!mask[start] || seen[start]) continue;
    let top = 0, minX = w, minY = h, maxX = -1, maxY = -1;
    stack[top++] = start;
    seen[start] = 1;
    while (top > 0) {
      const i = stack[--top];
      const x = i % w, y = (i - x) / w;
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      if (y < minY) minY = y;
      if (y > maxY) maxY = y;
      for (const j of [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, y > 0 ? i - w : -1, y < h - 1 ? i + w : -1]) {
        if (j >= 0 && mask[j] && !seen[j]) { seen[j] = 1; stack[top++] = j; }
      }
    }
    const cw = maxX - minX + 1, ch = maxY - minY + 1;
    if (cw >= NOTE_MIN_PX && ch >= NOTE_MIN_PX) noteRects.push({ x: A.x0 + minX, y: A.y0 + minY, w: cw, h: ch });
  }
  return { notes: noteRects.length, noteRects, noteColour: `#${(colour >>> 8).toString(16).padStart(6, '0')}` };
}

/** the search's candidates: the centres of the grid over a window, from its bottom-right corner (the ratios are the search's steps) */
function gridCandidates() {
  const out = [];
  for (let row = ACCEPT_GRID.rows - 1; row >= 0; row--) for (let col = ACCEPT_GRID.cols - 1; col >= 0; col--) out.push({ fx: (col + 0.5) / ACCEPT_GRID.cols, fy: (row + 0.5) / ACCEPT_GRID.rows });
  return out;
}

/** one start of the bundle at a devicePixelRatio, and the action of one tool; returns the JSON object of the run */
async function runOnce(bundleDir, tool, dpr, { logFile, shotDir }) {
  const item = TOOL_ITEMS[tool];
  const steps = [];
  const res = {
    tool, dpr, error: null, engine: null, stopped: null, presentedDelta: null, notes: null, noteRects: null, noteColour: null, noteWindow: null, acceptSearch: null,
    notesNote: 'riff machine only: the Accept is searched (acceptSearch); countNotes on the Piano roll window (table rectangle) after the press that closed the window, against the picture before that press; chord-progression: not counted. NOT a count of the notes on this screen: the rule takes a window highlight or background colour (pm-work/findings/riff-accept-1430.md)',
    missingPaths: [], stopLines: [], windowNew: null, warnings: [], warningBox: null, elapsedMs: null, preBootBytes: null, nproc: os.availableParallelism(), loadavg: null, room: null, logTail: [], logLengthAtItem: null, steps,
  };
  const { out } = await bootBundle(bundleDir, {
    dpr,
    afterIdle: async ({ send, evaluate }) => {
      const wins = async () => parseWindows(await evaluate(WINDOWS));
      const press = async (vk, name) => {
        await evaluate("document.getElementById('desktop').focus({ preventScroll: true }); true");
        await sleep(100);
        await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
        await sleep(40);
        await send('Input.dispatchKeyEvent', { type: 'keyUp', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
      };
      const guestPoint = async (gx, gy) => {
        const r = await evaluate(RECT);
        return { x: r.x + (gx + 0.5) * (r.w / r.gw), y: r.y + (gy + 0.5) * (r.h / r.gh) };
      };
      const mouse = (c, type) => send('Input.dispatchMouseEvent', type === 'mouseMoved'
        ? { type, x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' }
        : { type, x: c.x, y: c.y, button: 'left', buttons: type === 'mousePressed' ? 1 : 0, clickCount: 1, pointerType: 'mouse' });
      // the button down at a guest point (moved, then pressed); the release is upAt
      const downAt = async (gx, gy) => {
        const c = await guestPoint(gx, gy);
        await mouse(c, 'mouseMoved');
        await sleep(60);
        await mouse(c, 'mousePressed');
        return c;
      };
      const upAt = async (c) => {
        await mouse(c, 'mouseReleased');
        await sleep(100);
      };
      const clickAt = async (gx, gy) => {
        const c = await downAt(gx, gy);
        await sleep(50);
        await upAt(c);
      };
      const firstLog = (await evaluate(STATE)).logLength;
      // the room: 1836 x 1056 CSS px, set as tools/smoke/welcome-flow.mjs sets it (the stage is measured until it has that size)
      await send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: dpr, mobile: false });
      await sleep(1500);
      for (let i = 0; i < 4; i++) {
        const r = await evaluate(STAGE);
        if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) break;
        await send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: dpr, mobile: false });
        await sleep(1500);
      }
      res.preBootBytes = await evaluate(READ_BYTES);
      const idle = await evaluate(READ_IDLE);
      res.elapsedMs = idle === null ? null : Math.round(idle);
      res.room = (await evaluate(STATE)).guest;
      steps.push('room set');
      const collect = async () => {
        const end = await evaluate(STATE);
        res.engine = end.state;
        res.error = res.error ?? end.error;
        const lines = (await evaluate(LOG_ALL)).map((l) => l.replace(/^\[err\] /, ''));
        res.missingPaths = (await evaluate(LOG_FROM(firstLog))).map((l) => l.replace(/^\[err\] /, '')).filter((l) => l.startsWith('missing-path:'));
        res.stopLines = lines.filter((l) => STOP_LINES.test(l));
        res.stopped = ['stopped', 'exited', 'error'].includes(end.state) || lines.some((l) => STOP_LINE.test(l));
        res.logTail = lines.slice(-12);
        if (logFile) fs.appendFileSync(logFile, `# dpr ${dpr}: the engine's log (${lines.length} lines; logLengthAtItem ${res.logLengthAtItem})\n${lines.join('\n')}\n`);
      };
      // a Warning box: the text is read from the guest's memory (where a marker is found); Enter closes it (its default button), as a user does
      const closeWarning = async (w, first) => {
        const guestText = await evaluate(GUEST_TEXT(WARNING_MARKERS));
        if (first && !res.warningBox) res.warningBox = { windows: [{ title: w.text, visible: w.vis, children: descendants(await wins(), w.lid) }], guestText };
        res.warnings.push(`${w.text || WARNING_CLASS}: ${guestText.best ? guestText.best.lines.join(' / ') : '(no text read)'}`);
        await press(KEY_ENTER, 'Enter');
        steps.push('Enter (the Warning box: the default button, as a user closes it)');
        await sleep(ACTION_WAIT_MS);
      };
      // the item of the TOOLS menu: the menu, the Down presses, Enter (and a Warning box that comes after it is closed). Returns the windows before and after the item.
      const openItem = async (first) => {
        const list0 = await wins();
        const bar = list0.find((w) => w.cls === MENU_BAR_CLASS && w.child && absolutePosition(list0, w).y < 60) ?? null;
        if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
        const barAt = absolutePosition(list0, bar);
        const toolsAt = { x: barAt.x + TOOLS_LABEL.fx * bar.w, y: barAt.y + TOOLS_LABEL.fy * bar.h };
        const beforeMenu = list0.map((w) => w.lid);
        await clickAt(toolsAt.x, toolsAt.y);
        steps.push(`${first ? '' : 'opened again: '}TOOLS clicked at guest (${Math.round(toolsAt.x)}, ${Math.round(toolsAt.y)})`);
        let popup = null;
        for (let i = 0; i < 10 && !popup; i++) {
          await sleep(300);
          popup = (await wins()).find((w) => w.cls === POPUP_CLASS && !beforeMenu.includes(w.lid)) ?? null;
        }
        if (!popup) throw new Error('the TOOLS drop-down did not open');
        if (first) steps.push(`TOOLS drop-down: lid ${popup.lid}, ${popup.w}x${popup.h}`);
        for (let i = 0; i < item.downs; i++) { await press(KEY_DOWN, 'ArrowDown'); await sleep(250); }
        const beforeItem = (await wins()).map((w) => w.lid);
        if (first) res.logLengthAtItem = (await evaluate(STATE)).logLength;
        await press(KEY_ENTER, 'Enter');
        steps.push(`${item.downs} Down, Enter`);
        await sleep(ACTION_WAIT_MS);
        let afterItem = await wins();
        const warnings = afterItem.filter((w) => w.cls === WARNING_CLASS && !w.child && !beforeItem.includes(w.lid));
        for (const w of warnings) await closeWarning(w, first);
        if (warnings.length) afterItem = await wins();
        return { beforeItem, afterItem };
      };
      // Accept (#1430): the grid's centres are pressed in the window one by one; the press that closes the window with notes is the Accept
      const searchAccept = async () => {
        const search = { grid: `${ACCEPT_GRID.cols}x${ACCEPT_GRID.rows}`, candidates: ACCEPT_GRID.cols * ACCEPT_GRID.rows, tried: 0, reopened: 0, pressedAt: [], closed: [], popups: 0, warnings: 0, found: null, foundWindows: null, guestFailedAfter: null };
        const presents0 = (await evaluate(STATE)).presents;
        let hit = null;
        for (const cand of gridCandidates()) {
          // a guest thread that failed (a stop): the window does not answer any more, so the search ends here
          if (await evaluate(GUEST_FAILED)) { search.guestFailedAfter = search.tried; steps.push(`the search ends: a guest thread failed after ${search.tried} press(es)`); break; }
          let list = await wins();
          const strays = list.filter((w) => w.cls === WARNING_CLASS && !w.child);
          for (const w of strays) { search.warnings++; await closeWarning(w, false); list = await wins(); }
          let tw = list.find((w) => w.cls === item.windowClass && !w.child) ?? null;
          if (!tw) {
            // a press that closed the window with no notes (Cancel-like): the item is opened again for the next candidate
            await openItem(false);
            search.reopened++;
            list = await wins();
            tw = list.find((w) => w.cls === item.windowClass && !w.child) ?? null;
            if (!tw) throw new Error(`no ${item.windowClass} window after opening it again`);
          }
          search.tried++;
          const pr = list.find((w) => w.cls === PIANO_ROLL_CLASS && w.text.startsWith(PIANO_ROLL_CAPTION)) ?? null;
          if (!pr) throw new Error('the Piano roll is not in the window table during the search');
          const at = absolutePosition(list, tw);
          const rect = { x: at.x, y: at.y, w: tw.w, h: tw.h };
          const prAt = absolutePosition(list, pr);
          const prRect = { x: prAt.x, y: prAt.y, w: pr.w, h: pr.h };
          const topBefore = list.filter((w) => !w.child).map((w) => w.lid);
          const before = await evaluate(PICTURE);
          const c = await downAt(at.x + cand.fx * tw.w, at.y + cand.fy * tw.h);
          await sleep(PRESS_HOLD_MS);
          const held = await evaluate(PICTURE);
          const look = areaHash(held, rect) !== areaHash(before, rect);
          await upAt(c);
          await sleep(SETTLE_MS);
          if (look) search.pressedAt.push([round3(cand.fx), round3(cand.fy)]);
          list = await wins();
          const closed = !list.some((w) => w.lid === tw.lid);
          if (!closed) {
            const extra = list.filter((w) => !w.child && !topBefore.includes(w.lid));
            for (const w of extra) {
              if (w.cls === WARNING_CLASS) { search.warnings++; res.warnings.push(`${w.text || WARNING_CLASS}: (after a press of the search; its text not read)`); await press(KEY_ENTER, 'Enter'); }
              else { search.popups++; await press(KEY_ESCAPE, 'Escape'); }
              await sleep(400);
            }
            continue;
          }
          await sleep(ACTION_WAIT_MS);
          const afterPic = await evaluate(PICTURE);
          const counted = countNotes(before, afterPic, prRect);
          search.closed.push({ fx: round3(cand.fx), fy: round3(cand.fy), notes: counted.notes });
          if (shotDir) {
            fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-close${search.closed.length}-before.png`), pngOf(before));
            fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-close${search.closed.length}-after.png`), pngOf(afterPic));
          }
          steps.push(`candidate (${round3(cand.fx)}, ${round3(cand.fy)}) closed the window: notes ${counted.notes}`);
          if (counted.notes >= 1) {
            const late = (await wins()).filter((w) => w.cls === WARNING_CLASS && !w.child && !topBefore.includes(w.lid));
            for (const w of late) res.warnings.push(`${w.text || WARNING_CLASS}: (after the Accept; its text not read)`);
            search.foundWindows = (await wins()).filter((w) => !w.child).map((w) => ({ cls: w.cls, title: w.text }));
            hit = { cand, counted, prRect, caption: pr.text, beforePic: before, afterPic };
            break;
          }
        }
        if (hit) {
          search.found = { fx: round3(hit.cand.fx), fy: round3(hit.cand.fy) };
          res.notes = hit.counted.notes;
          res.noteRects = hit.counted.noteRects;
          res.noteColour = hit.counted.noteColour;
          res.noteWindow = { caption: hit.caption, ...hit.prRect };
          steps.push(`Accept found at the ratios (${search.found.fx}, ${search.found.fy}) of the window after ${search.tried} press(es); notes counted in "${hit.caption}" at ${hit.prRect.x},${hit.prRect.y} ${hit.prRect.w}x${hit.prRect.h}: ${hit.counted.notes}`);
          if (shotDir) {
            fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-before-accept.png`), pngOf(hit.beforePic));
            fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-after.png`), pngOf(hit.afterPic));
          }
        } else {
          steps.push(`no press closed the window with notes (${search.tried} pressed)`);
        }
        const presents1 = (await evaluate(STATE)).presents;
        res.presentedDelta = presents0 !== null && presents1 !== null ? presents1 - presents0 : null;
        res.acceptSearch = search;
      };
      try {
        await press(KEY_ESCAPE, 'Escape');
        steps.push('Escape (Welcome window)');
        const t0 = Date.now();
        for (;;) {
          const s = await evaluate(STATE);
          if (s.state !== 'engine-idle') throw new Error(`state ${s.state} while the default project opens`);
          if (s.title !== WELCOME_TITLE && s.title !== '') break;
          if (Date.now() - t0 >= OPEN_LIMIT_MS) throw new Error('the default project did not open within the limit');
          await sleep(POLL_MS);
        }
        for (let i = 0; i < 40; i++) {
          if (!(await wins()).find((w) => w.cls === PROBLEMS_CLASS)) break;
          await press(KEY_ESCAPE, 'Escape');
          steps.push('Escape (Problems box)');
          await sleep(1500);
        }
        await sleep(2000);
        const opened = await evaluate(STATE);
        steps.push(`project open: title "${opened.title}"`);
        if (opened.state !== 'engine-idle') throw new Error(`state ${opened.state} before the Piano roll`);
        await press(KEY_F7, 'F7');
        let pr = null;
        for (let i = 0; i < 12 && !pr; i++) {
          await sleep(500);
          pr = (await wins()).find((w) => w.cls === PIANO_ROLL_CLASS && w.text.startsWith(PIANO_ROLL_CAPTION)) ?? null;
        }
        if (!pr) throw new Error('the Piano roll did not open (F7)');
        steps.push(`Piano roll: lid ${pr.lid} "${pr.text}"`);
        const first = await openItem(true);
        let toolWindow = null;
        if (item.windowClass) {
          toolWindow = first.afterItem.find((w) => w.cls === item.windowClass) ?? null;
          if (!toolWindow) throw new Error(`no ${item.windowClass} window after the item`);
          steps.push(`tool window: lid ${toolWindow.lid} "${toolWindow.text}" at ${toolWindow.x},${toolWindow.y} ${toolWindow.w}x${toolWindow.h}`);
        }
        // what the item opened, as the picture shows it before the button is pressed
        const added = first.afterItem.filter((w) => !first.beforeItem.includes(w.lid) && !w.child);
        // the picture before the button (the new window as it is drawn; the Piano roll with the window over it)
        const openedPic = added.length || shotDir || tool === 'riff-machine' ? await evaluate(PICTURE) : null;
        if (openedPic && shotDir) fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-opened.png`), pngOf(openedPic));
        if (added.length) res.windowNew = added.map((w) => {
          const dom = dominantShare(openedPic, { x: w.x, y: w.y, w: w.w, h: w.h });
          return { cls: w.cls, title: w.text, x: w.x, y: w.y, w: w.w, h: w.h, visible: w.vis, painted: dom.share < PAINTED_BELOW, dominantShare: round3(dom.share), children: descendants(first.afterItem, w.lid) };
        });
        if (tool === 'chord-progression') {
          const at = absolutePosition(first.afterItem, toolWindow);
          const c = { x: at.x + START_CREATING.fx * toolWindow.w, y: at.y + START_CREATING.fy * toolWindow.h };
          const presents0 = (await evaluate(STATE)).presents;
          await clickAt(c.x, c.y);
          steps.push(`Start creating clicked at guest (${Math.round(c.x)}, ${Math.round(c.y)})`);
          await sleep(ACTION_WAIT_MS);
          const end = await evaluate(STATE);
          res.presentedDelta = presents0 !== null && end.presents !== null ? end.presents - presents0 : null;
          if (shotDir) fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-after.png`), pngOf(await evaluate(PICTURE)));
        } else {
          await searchAccept();
        }
      } finally {
        await collect();
      }
    },
  });
  res.loadavg = out.loadavg ?? os.loadavg();
  if (out.error && res.error === null) res.error = out.error;
  return res;
}

async function main() {
  const argv = process.argv.slice(2);
  const at = (flag) => { const i = argv.indexOf(flag); return i >= 0 ? argv[i + 1] : undefined; };
  const tool = at('--tool');
  const dprs = (at('--dprs') ?? '1').split(',').map(Number);
  const logFile = at('--log') ? path.resolve(at('--log')) : null;
  const shotDir = at('--shot') ? path.resolve(at('--shot')) : null;
  const positional = argv.filter((a, i) => !a.startsWith('--') && !['--tool', '--dprs', '--log', '--shot'].includes(argv[i - 1]));
  if (positional.length !== 1 || !TOOL_ITEMS[tool] || dprs.some((d) => !(d >= 0.5 && d <= 4))) {
    process.stderr.write('usage: node tools/piano-roll-tools/measure.mjs <absolute path to the bundle directory raw/<name>> --tool <chord-progression|riff-machine> --dprs <1,1.37,...> [--log <file>] [--shot <dir>]\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  let failed = false;
  for (const dpr of dprs) {
    log(`start: tool ${tool}, dpr ${dpr}`);
    let line;
    try {
      line = await runOnce(bundleDir, tool, dpr, { logFile, shotDir });
    } catch (e) {
      failed = true;
      line = { tool, dpr, error: `tool: ${String(e?.message ?? e).slice(0, 300)}` };
    }
    process.stdout.write(JSON.stringify(line) + '\n');
    log(`done: dpr ${dpr}: error ${line.error ?? 'null'}, engine ${line.engine ?? null}, presentedDelta ${line.presentedDelta ?? null}`);
  }
  process.exitCode = failed ? 1 : 0;
}

// the measurement runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
