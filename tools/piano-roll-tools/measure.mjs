// piano-roll-tools: the measurement of the Piano roll's tools (#1386 chord progression, #1387 riff machine). It opens the default project, the Piano roll (F7), the main window's TOOLS menu and one tool; for the chord progression it presses "Start creating".
// It records and measures only: the product is not changed, and no branch for the measurement is in it.
// Usage: node tools/piano-roll-tools/measure.mjs <absolute path to the bundle directory raw/<name>> --tool <chord-progression|riff-machine> --dprs <list> [--log <file>] [--shot <dir>] > tools.jsonl
//   --log <file>: the engine's whole log after each run, appended with a "# dpr" line (logLengthAtItem in the JSON is the log's length when the item was chosen).
//   --shot <dir>: the desk's picture as a PNG: <tool>-dpr<dpr>-opened.png (after the item, the Warning box closed, before the button) and <tool>-dpr<dpr>-after.png (after the button). For reading the screen by eye (no OCR).
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
//   - riff machine: the item first opens a Warning box (TMsgForm: "The plugin Sytrus couldn't be found or opened." in the guest's memory, read with window.__app.guestText; the presets need Sytrus); Enter closes it (as a user does), and then the
//     window "Piano roll - riff machine" (TPRScoreCreatorForm) is the tool window. Its main button ("Accept", bottom right of the Prog step: MAIN_BUTTON, a fraction of the window's rectangle, from the picture at DPR 1) is pressed.
// Each JSON line: tool, dpr, error (the page's error, or the tool's failure; null when none), engine (the engine's state 3 s after the action), stopped (the state is stopped, exited or error, or a "== stop:" line is in the log),
//   presentedDelta (the desktop's presents in the 3 s after the action: window.__app.deskStats.presents), notes and noteRects (riff machine: the Piano roll's notes after the button, countNotes; null for the others), warnings (the texts of every Warning box that appeared, closed by the tool or not: [] when none), warningBox (the Warning box that came first, with its children and the guest's text near the markers; null when none), missingPaths (the missing-path: lines since the first idle),
//   stopLines (the engine's stop lines: "== stop:" (also "returned from entry", which a run that goes on prints too), "fle:", "fruity: exit code", "runtime message:"), windowNew (the top-level windows that appeared with the action: { cls, title, x, y, w, h, visible, painted (its most common colour covers less than 95%), dominantShare, children }; null when none),
//   elapsedMs (the page's timeline: the first idle, in ms from the navigation), preBootBytes (the assetsReport bytes), nproc, loadavg, room (the guest's screen, window.__app.screen.guest),
//   logTail (the last 12 lines of the engine's log, in order: the order of the stop), steps (what the tool did, in order).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';
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
// the riff machine's main button ("Accept", the bottom-right button of the "Prog" step; the window has no caption for it in the window table): its centre as fractions of the window's rectangle (measured from the picture at DPR 1)
const MAIN_BUTTON = { fx: 0.856, fy: 0.929 };
const ACTION_WAIT_MS = 3000;
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;
// a "== stop:" line is a stop, except "returned from entry": the harness prints that one when the entry returns, in a run that goes on (idle)
const STOP_LINE = /^(?:fruity: )?== stop: (?!returned from entry)/;
const STOP_LINES = /^(?:fruity: == stop:|== stop:|fle: |fruity: exit code|fruity: runtime message|runtime message: )/;

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

const log = (s) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${s}\n`);

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

/** the child windows of one window (all levels) in a window list: { cls, text, vis, w, h }, in the table's order (at most 60) */
export function descendants(list, lid) {
  const out = [];
  const walk = (at) => {
    for (const c of list) {
      if (!c.child || c.parent !== at || out.length >= 60) continue;
      out.push({ cls: c.cls, text: c.text, vis: c.vis, w: c.w, h: c.h });
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

/** one start of the bundle at a devicePixelRatio, and the action of one tool; returns the JSON object of the run */
async function runOnce(bundleDir, tool, dpr, { logFile, shotDir }) {
  const item = TOOL_ITEMS[tool];
  const steps = [];
  const res = {
    tool, dpr, error: null, engine: null, stopped: null, presentedDelta: null, notes: null, noteRects: null, noteColour: null, noteWindow: null,
    notesNote: 'riff machine only: countNotes on the Piano roll window (table rectangle) after the button, against the picture before it; chord-progression: not counted',
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
      const clickAt = async (gx, gy) => {
        const r = await evaluate(RECT);
        const c = { x: r.x + (gx + 0.5) * (r.w / r.gw), y: r.y + (gy + 0.5) * (r.h / r.gh) };
        await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
        await sleep(60);
        await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
        await sleep(50);
        await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
        await sleep(100);
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
        const list0 = await wins();
        const bar = list0.find((w) => w.cls === MENU_BAR_CLASS && w.child && absolutePosition(list0, w).y < 60) ?? null;
        if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
        const barAt = absolutePosition(list0, bar);
        const toolsAt = { x: barAt.x + TOOLS_LABEL.fx * bar.w, y: barAt.y + TOOLS_LABEL.fy * bar.h };
        const beforeMenu = list0.map((w) => w.lid);
        await clickAt(toolsAt.x, toolsAt.y);
        steps.push(`TOOLS clicked at guest (${Math.round(toolsAt.x)}, ${Math.round(toolsAt.y)})`);
        let popup = null;
        for (let i = 0; i < 10 && !popup; i++) {
          await sleep(300);
          popup = (await wins()).find((w) => w.cls === POPUP_CLASS && !beforeMenu.includes(w.lid)) ?? null;
        }
        if (!popup) throw new Error('the TOOLS drop-down did not open');
        steps.push(`TOOLS drop-down: lid ${popup.lid}, ${popup.w}x${popup.h}`);
        for (let i = 0; i < item.downs; i++) { await press(KEY_DOWN, 'ArrowDown'); await sleep(250); }
        const beforeItem = (await wins()).map((w) => w.lid);
        let presents0 = (await evaluate(STATE)).presents;
        res.logLengthAtItem = (await evaluate(STATE)).logLength;
        await press(KEY_ENTER, 'Enter');
        steps.push(`${item.downs} Down, Enter`);
        await sleep(ACTION_WAIT_MS);
        let afterItem = await wins();
        const warnings = afterItem.filter((w) => w.cls === WARNING_CLASS && !w.child && !beforeItem.includes(w.lid));
        if (warnings.length) {
          const guestText = await evaluate(GUEST_TEXT(WARNING_MARKERS));
          res.warningBox = {
            windows: warnings.map((w) => ({ title: w.text, visible: w.vis, children: descendants(afterItem, w.lid) })),
            guestText,
          };
          // every Warning box that appears is recorded, closed here to go on or not: the acceptance wants none
          res.warnings.push(...warnings.map((w) => `${w.text || WARNING_CLASS}: ${guestText.best ? guestText.best.lines.join(' / ') : '(no text read)'}`));
          await press(KEY_ENTER, 'Enter');
          steps.push('Enter (the Warning box: the default button, as a user closes it)');
          await sleep(ACTION_WAIT_MS);
          afterItem = await wins();
        }
        let toolWindow = null;
        if (item.windowClass) {
          toolWindow = afterItem.find((w) => w.cls === item.windowClass) ?? null;
          if (!toolWindow) throw new Error(`no ${item.windowClass} window after the item`);
          steps.push(`tool window: lid ${toolWindow.lid} "${toolWindow.text}" at ${toolWindow.x},${toolWindow.y} ${toolWindow.w}x${toolWindow.h}`);
        }
        // what the item opened, as the picture shows it before the button is pressed
        const added = afterItem.filter((w) => !beforeItem.includes(w.lid) && !w.child);
        // the picture before the button (the new window as it is drawn; the Piano roll with the window over it)
        const openedPic = added.length || shotDir || tool === 'riff-machine' ? await evaluate(PICTURE) : null;
        if (openedPic && shotDir) fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-opened.png`), pngOf(openedPic));
        if (added.length) res.windowNew = added.map((w) => {
          const dom = dominantShare(openedPic, { x: w.x, y: w.y, w: w.w, h: w.h });
          return { cls: w.cls, title: w.text, x: w.x, y: w.y, w: w.w, h: w.h, visible: w.vis, painted: dom.share < PAINTED_BELOW, dominantShare: Math.round(dom.share * 1000) / 1000, children: descendants(afterItem, w.lid) };
        });
        if (tool === 'chord-progression' || tool === 'riff-machine') {
          const at = absolutePosition(afterItem, toolWindow);
          const b = tool === 'chord-progression' ? START_CREATING : MAIN_BUTTON;
          const c = { x: at.x + b.fx * toolWindow.w, y: at.y + b.fy * toolWindow.h };
          presents0 = (await evaluate(STATE)).presents;
          await clickAt(c.x, c.y);
          steps.push(`${tool === 'chord-progression' ? 'Start creating' : 'main button (Accept)'} clicked at guest (${Math.round(c.x)}, ${Math.round(c.y)})`);
          await sleep(ACTION_WAIT_MS);
        }
        const end = await evaluate(STATE);
        res.presentedDelta = presents0 !== null && end.presents !== null ? end.presents - presents0 : null;
        const afterPic = tool === 'riff-machine' || shotDir ? await evaluate(PICTURE) : null;
        if (afterPic && shotDir) fs.writeFileSync(path.join(shotDir, `${tool}-dpr${dpr}-after.png`), pngOf(afterPic));
        if (tool === 'riff-machine') {
          // the Warning boxes that came after the item (the button may open one): recorded, not read
          const late = (await wins()).filter((w) => w.cls === WARNING_CLASS && !w.child && !beforeItem.includes(w.lid) && !warnings.some((x) => x.lid === w.lid));
          res.warnings.push(...late.map((w) => `${w.text || WARNING_CLASS}: (after the button; its text not read)`));
          const list = await wins();
          const pianoRoll = list.find((w) => w.cls === PIANO_ROLL_CLASS && w.text.startsWith(PIANO_ROLL_CAPTION)) ?? null;
          if (!pianoRoll) throw new Error('the Piano roll window is not in the window table after the button');
          const at = absolutePosition(list, pianoRoll);
          const rect = { x: at.x, y: at.y, w: pianoRoll.w, h: pianoRoll.h };
          const counted = countNotes(openedPic, afterPic, rect);
          res.notes = counted.notes;
          res.noteRects = counted.noteRects;
          res.noteColour = counted.noteColour;
          res.noteWindow = { caption: pianoRoll.text, ...rect };
          steps.push(`notes counted in "${pianoRoll.text}" at ${rect.x},${rect.y} ${rect.w}x${rect.h}: ${counted.notes}`);
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
