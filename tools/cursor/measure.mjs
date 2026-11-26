// The cursor measurement of #1390 (step 1: the sizing edges of the windows and the standard cursors). It records and measures only: the product is not changed, and the pointer only moves (and presses the buttons of the Welcome window and the Default row, as tools/smoke/welcome-flow.mjs does).
// Usage: node tools/cursor/measure.mjs <absolute path to the bundle directory raw/<name>> [--force-sizable <lid>[,<lid>...]] > cursor.json   (needs app/node_modules and app/dist, as boot-bundle.mjs does). Exit 1 when error is set, 2 when no bundle directory is given.
//   --force-sizable: a diagnostic for a bundle whose window table has no style of the child lines (the build before #1390): the windows with these lids are measured as sizable too (their style is then not read).
//   --pencil (#1392): at the Piano roll (step 3), the pointer is put over its grid until the page shows a picture cursor (url(...)); the Draw key D is pressed when none does. The page's PNG is compared with a
//     reference made outside the engine's pixel code: a resource's pixels are read from FLEngine_x64.dll (tools/cursor/resource-cursor.mjs); a cursor made by CreateIconIndirect has no resource, so its reference is
//     built here from the raw copy of its bitmaps that the engine gives (pencilRaw). The check is made at device pixel ratio 1, 2 and a random one in 1.01..3.00 (pencilDprs; the room is set again for each);
//     at every ratio the page shows image-set of its PNG at the ratio (#1480: one copy, the picture's own pixels) and the PNG is compared with the reference's pixels as they are. Fields: pencilOk, pencilResource, pencilCursorId, pencilSize, pencilHotspot(Match), pencilPixelsMatch, pencilMismatch, pencilInvertPixels, pencilAlpha, pencilCounts, pencilDprs, pencilTries, pencilCss, pencilRaw.
// Order (on the engine's first idle, bootBundle's afterIdle):
//   0. the room is set to 1836 x 1056 CSS px (the guest's screen; tools/smoke/welcome-flow.mjs calibrates it the same way);
//   1. the window table (window.__app.deskDumpBase64(false): one line per drawn top-level window with its style; one line per child window, and a "child style" line with the child's style) is read: the Welcome window (class TWelcomeWizard) is found at its position;
//   2. the usage-data box is closed with "No thanks" (the Welcome window's offset (512, 307)); the Default row is double-clicked after the start list is scrolled to its top and 3 notches down (as flow A of welcome-flow): the project opens, and the "Problems loading the project" box is closed with Escape;
//   3. F7 opens the Piano roll (the child window of class TEventEditForm with the caption "Piano roll", as tools/piano-roll-tools/measure.mjs);
//   4. a sweep of FL's own windows: F2, F3, F4, F5, F6, F8, F10, F11, F12, F1, Ctrl+O, Ctrl+E, Ctrl+P, Ctrl+F, Ctrl+I, each pressed once, then Escape. After each key the window table is read and every window of it is recorded with its style (windowStyles).
//      A window is SIZABLE when its style has WS_THICKFRAME (0x00040000), it is visible, and it is neither maximised (0x01000000) nor minimised (0x20000000): its 8 directions are measured while it is shown (5). A window that a key opened is also checked for the I-beam while it is shown (6);
//   5. for each sizable window, the 8 directions N, S, E, W, NE, NW, SE, SW: a point 2 guest px inside the edge (the band is SM_CXSIZEFRAME + SM_CXPADDEDBORDER = 8 px at 96 DPI). The point is taken where the window is the one on top (checked against the top-level windows: a child's top-level window must be the top one there), else the next point of the edge, and "covered" is false when none is. A CDP mouseMoved to the point, then #desktop's cursor (style.cursor) read until two reads in a row agree;
//   6. text: the visible children (at least 16 x 8, in table order, up to 60) of the windows that were opened, and of the main form, are tried, then a grid over the main form (64 x 48 guest px, moves only): the first point whose cursor is text (the I-beam) decides textOk;
//   7. diagnostics (not samples): the 8 directions of the main form (mainFormEdges); the 8 directions of every other visible top-level window (diagEdges: a window that is not sizable, the Welcome window and the missing-plug-ins box aside, 8 at most, read at the Piano roll's screen); every window style seen (windowStyles, thickframeAny); window.__app.cursorRecord (the standard cursors with no CSS equivalent, by name and value; the cursors the guest made itself; the handles with no identity) and window.__app.cursor at the end.
//   8. the window-API trace (window.__app.windowApiTrace(true) from the first idle on): each edge probe keeps the trace lines the engine printed for it (trace: the hit: line of the pointer, the wndproc: in / out / def lines of WM_NCHITTEST and WM_SETCURSOR, the SetCursor lines with the IDC, the GWL_STYLE and SetWindowPos lines); styleTrace is every GWL_STYLE change of the session (a sizing frame added later), setPosCount the number of SetWindowPos lines.
// The values are the runner's (nproc and loadavg are written with them), not a claim about any other machine.
// stdout: ONE JSON line: {error, state, standIn, nproc, loadavg, windowsAtStart, sizableAtStart, pianoRoll, childClasses, sizable, edgeWindows, edgeSamples, edgeMatches, edgeDetail, diagEdges, styleTrace, setPosCount, textOk, textFoundAt, textPoints, textWindows, textCandidates, textDetail, unresolved, unmapped, unmappedNames, madeByGuest, mainFormEdges, thickframeAny, windowStyles, cursorRecord, cursorAtEnd, windowsAtEnd}.
//   edgeSamples = 8 x edgeWindows (checked: error otherwise); edgeMatches = the samples whose cursor is the expected one: N, S -> ns-resize; E, W -> ew-resize; NE, SW -> nesw-resize; NW, SE -> nwse-resize.
//   unresolved = cursorRecord.unresolved (a handle that has no identity was reported); unmapped = the number of distinct standard cursors with no CSS equivalent, unmappedNames their names; madeByGuest = the reports of a cursor the guest made itself (not counted as unresolved).
// stderr: one line per step (the windows, each sample, each text check).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';
import { decodePngRgba, readPeResourceCursor, referenceFromRaw } from './resource-cursor.mjs';

// the room: the page's stage is 1836 x 1056 CSS px, which is the guest's screen (tools/smoke/welcome-flow.mjs and tools/p2-frames calibrate it); the Welcome window is centred in it
const ROOM_W = 1836, ROOM_H = 1056;
// the Welcome window (class TWelcomeWizard) and the offsets of its controls from its top-left corner (tools/smoke/welcome-flow.mjs; the offsets are the picture's)
const WIZARD = 'TWelcomeWizard';
const NO_THANKS = { dx: 512, dy: 307 };
const LIST = { dx: 235, dy: 180 };
const DEFAULT_ROW = { dx: 235, dy: 117 };
const NOTCH_UP = 12, NOTCH_DOWN = 3, NOTCH_DELTA = 120;
const PROBLEMS = 'TMissingPluginsForm';
const PIANO_ROLL = 'TEventEditForm';
const KEY_ESCAPE = 27, KEY_F7 = 118, KEY_D = 68;
// #1392 (--pencil): the Piano roll's rectangle, as fractions of it, where the pointer is put in turn (the note grid is the middle; a point under another window is skipped)
const PENCIL_POINTS = [[0.5, 0.5], [0.3, 0.5], [0.7, 0.5], [0.5, 0.75], [0.3, 0.75], [0.7, 0.75], [0.5, 0.3], [0.8, 0.9]];
// the trace lines of a pencil probe: the hit test of the pointer, the cursor the guest set (SetCursor, with its handle and IDC) and the window messages of the cursor
const PENCIL_TRACE_RE = '/hit: |SetCursor|WM_SETCURSOR|LoadCursor|CreateIconIndirect/';
// the lifted DLL of the engine: the resources of FL's cursors are in its PE file (lift/out-app/host/binaries/win64/FLEngine_x64.dll, the image mapped at FLE_EXE_HANDLE)
const FL_ENGINE_DLL = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../lift/out-app/host/binaries/win64/FLEngine_x64.dll');
// the F-keys that open FL's own windows (a sweep: each key once, the windows it shows are looked at, then Escape closes them); a sizable window found there is measured while it is open
const CTRL = 2;
const SWEEP = [
  { name: 'F2', vk: 113 }, { name: 'F3', vk: 114 }, { name: 'F4', vk: 115 }, { name: 'F5', vk: 116 }, { name: 'F6', vk: 117 }, { name: 'F8', vk: 119 }, { name: 'F10', vk: 121 }, { name: 'F11', vk: 122 }, { name: 'F12', vk: 123 },
  { name: 'F1', vk: 112 }, { name: 'Ctrl+O', vk: 79, ctrl: true }, { name: 'Ctrl+E', vk: 69, ctrl: true }, { name: 'Ctrl+P', vk: 80, ctrl: true }, { name: 'Ctrl+F', vk: 70, ctrl: true }, { name: 'Ctrl+I', vk: 73, ctrl: true },
];
const OPEN_LIMIT_MS = 120000;
const POLL_MS = 500;
const WS_THICKFRAME = 0x00040000, WS_MAXIMIZE = 0x01000000, WS_MINIMIZE = 0x20000000;
const INSET = 2;
const TEXT_MIN_W = 16, TEXT_MIN_H = 8, TEXT_TRIES = 60;
const GRID_STEP_X = 64, GRID_STEP_Y = 48;   // the fallback of the text check: a grid over the main form (moves only)
const EXPECTED = { N: 'ns-resize', S: 'ns-resize', E: 'ew-resize', W: 'ew-resize', NE: 'nesw-resize', SW: 'nesw-resize', NW: 'nwse-resize', SE: 'nwse-resize' };
const DIRS = ['N', 'S', 'E', 'W', 'NE', 'SW', 'NW', 'SE'];

const ROOM = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const STATE = `(() => { const a = window.__app; if (!a) return null; return { state: a.state, title: document.title, standIn: document.body.innerText.includes('STAND-IN') }; })()`;
// the whole picture drawn again from the window table (not the peek: the peek reads back a WebGL canvas, which a headless Chrome without WebGL2 has not)
const DUMP = `window.__app.deskDumpBase64(false).then((d) => ({ width: d.width, height: d.height, windows: d.windows }))`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; })()`;
const CURSOR = `(() => { const d = document.getElementById('desktop'); return d.style.cursor || getComputedStyle(d).cursor; })()`;
const RECORD = `(() => { const a = window.__app; return { cursorRecord: a.cursorRecord, cursorAtEnd: a.cursor }; })()`;
const TRACE_ON = `(() => { window.__app.windowApiTrace(true); return true; })()`;
const LOG_LEN = `window.__app.engineLog().length`;
/** the lines of the window-API trace that a probe reads (the pointer's hit line, the messages of the cursor, SetCursor, the style and position changes) */
const TRACE_RE = '/hit: |WM_NCHITTEST|WM_SETCURSOR|SetCursor|GWL_STYLE|SetWindowPos|WM_NCLBUTTON|WM_NCMOUSEMOVE/';
/** the trace lines printed since engine-log index n that match re (at most cap, the first ones); [] when the log was trimmed since (it keeps 50000 lines) */
const traceFrom = (n, re = TRACE_RE, cap = 40) => `(() => { const L = window.__app.engineLog(); return (L.length >= ${n} ? L.slice(${n}) : []).filter((l) => ${re}.test(l)).slice(0, ${cap}); })()`;
const countFrom = (n, re) => `(() => { const L = window.__app.engineLog(); return (L.length >= ${n} ? L.slice(${n}) : []).filter((l) => ${re}.test(l)).length; })()`;

const hex8 = (v) => `0x${(v >>> 0).toString(16).padStart(8, '0')}`;

/** the window table: the top-level lines (drawn, with a style) and the child lines (with their style, from the "child style" lines), each child with its absolute position (a child's position is relative to its parent: nc_zero makes the client origin the window's origin) and the top-level window it lies in */
export function parseTable(text) {
  const top = [], child = [], styles = new Map();
  for (const l of text.split('\n')) {
    let m = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) style 0x([0-9a-fA-F]+)(.*)$/.exec(l);
    if (m) {
      const cls = /class "([^"]*)"/.exec(m[7]), title = /title "([^"]*)"\s*$/.exec(m[7]);
      top.push({ lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), style: parseInt(m[6], 16) >>> 0, cls: cls ? cls[1] : '', title: title ? title[1] : '', child: false });
      continue;
    }
    m = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "([^"]*)"\s*$/.exec(l);
    if (m) { child.push({ lid: Number(m[1]), parent: Number(m[2]), px: Number(m[3]), py: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9], child: true }); continue; }
    m = /^child style lid (\d+) 0x([0-9a-fA-F]+)\s*$/.exec(l);
    if (m) styles.set(Number(m[1]), parseInt(m[2], 16) >>> 0);
  }
  for (const c of child) c.style = styles.get(c.lid) ?? 0;
  const abs = new Map(top.map((t) => [t.lid, { x: t.x, y: t.y }]));
  const parentOf = new Map(child.map((c) => [c.lid, c.parent]));
  for (let changed = true; changed;) {
    changed = false;
    for (const c of child) if (!abs.has(c.lid) && abs.has(c.parent)) { const p = abs.get(c.parent); abs.set(c.lid, { x: p.x + c.px, y: p.y + c.py }); changed = true; }
  }
  const rootOf = (lid) => { let l = lid; for (let k = 0; k < 64 && parentOf.has(l); k++) l = parentOf.get(l); return l; };
  for (const c of child) { const a = abs.get(c.lid); if (a) { c.x = a.x; c.y = a.y; } c.root = rootOf(c.lid); }
  for (const t of top) t.root = t.lid;
  return { top, child };
}

/** a sizable window: WS_THICKFRAME, visible (a top-level line is drawn; a child line has vis 1), not maximised, not minimised */
export const isSizable = (t) => (t.style & WS_THICKFRAME) !== 0 && (t.style & (WS_MAXIMIZE | WS_MINIMIZE)) === 0 && (t.child ? t.vis : true);

/** the top-level window on top at (x, y): the table lists the drawn top-level windows bottom first, so the last one that holds the point is the top one; 0 when none */
export function topAt(top, x, y) {
  let r = 0;
  for (const t of top) if (x >= t.x && y >= t.y && x < t.x + t.w && y < t.y + t.h) r = t.lid;
  return r;
}

/** the points of a direction: the first at the middle of the edge, the others along the edge (used when the first is covered) */
export function edgePoints(t, dir) {
  const x0 = t.x, y0 = t.y, x1 = t.x + t.w - 1, y1 = t.y + t.h - 1;
  const fx = [0.5, 0.25, 0.75].map((f) => Math.round(x0 + f * (x1 - x0)));
  const fy = [0.5, 0.25, 0.75].map((f) => Math.round(y0 + f * (y1 - y0)));
  switch (dir) {
    case 'N': return fx.map((x) => ({ x, y: y0 + INSET }));
    case 'S': return fx.map((x) => ({ x, y: y1 - INSET }));
    case 'W': return fy.map((y) => ({ x: x0 + INSET, y }));
    case 'E': return fy.map((y) => ({ x: x1 - INSET, y }));
    case 'NW': return [{ x: x0 + INSET, y: y0 + INSET }];
    case 'NE': return [{ x: x1 - INSET, y: y0 + INSET }];
    case 'SW': return [{ x: x0 + INSET, y: y1 - INSET }];
    default: return [{ x: x1 - INSET, y: y1 - INSET }];
  }
}

const log = (s) => process.stderr.write(`${s}\n`);
const cssAt = (r, pic, p) => ({ x: r.x + (p.x + 0.5) * (r.w / pic.width), y: r.y + (p.y + 0.5) * (r.h / pic.height) });

async function mouse(page, type, c, button, buttons, clickCount) {
  await page.send('Input.dispatchMouseEvent', { type, x: c.x, y: c.y, button, buttons, clickCount, pointerType: 'mouse' });
}

/** the cursor after the pointer is moved to the point: read until two reads in a row agree (the engine answers the move at its next idle) */
async function cursorAt(page, pic, rect, p) {
  await mouse(page, 'mouseMoved', cssAt(rect, pic, p), 'none', 0, 0);
  let last = null;
  for (let i = 0; i < 8; i++) {
    await sleep(i === 0 ? 500 : 250);
    const v = await page.evaluate(CURSOR);
    if (v === last) return v;
    last = v;
  }
  return last;
}

async function waitFor(pred, ms, what) {
  const t0 = Date.now();
  for (;;) {
    const v = await pred();
    if (v) return v;
    if (Date.now() - t0 >= ms) throw new Error(`timeout ${ms} ms: ${what}`);
    await sleep(POLL_MS);
  }
}

/** the window table and the size of the picture; the engine answers "the picture is being shown: ask again" while it presents, so the question is asked again */
async function dump(page) {
  for (let k = 0; ; k++) {
    try {
      const d = await page.evaluate(DUMP);
      return { width: d.width, height: d.height, text: d.windows, table: parseTable(d.windows) };
    } catch (e) {
      if (k >= 20 || !/ask again|no picture yet/.test(String(e?.message))) throw e;
      await sleep(300);
    }
  }
}

/** the room of the guest (1836 x 1056 CSS px) at the device scale factor `dsf` (1 unless the pencil check asks for another); returns the page's device pixel ratio then */
async function setRoom(page, dsf = 1) {
  let dpr = null;
  for (let i = 0; i < 4; i++) {
    const r = await page.evaluate(ROOM);
    dpr = await page.evaluate('window.devicePixelRatio');
    log(`room: stage ${r.sw}x${r.sh}, window ${r.iw}x${r.ih}, device pixel ratio ${dpr}`);
    if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5 && Math.abs(dpr - dsf) < 1e-9) break;
    await page.send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: dsf, mobile: false });
    await sleep(1500);
  }
  await sleep(1500);
  return dpr === null ? null : page.evaluate('window.devicePixelRatio');
}

async function clickGuest(page, pic, p, clicks = 1) {
  const r = await page.evaluate(RECT);
  const c = cssAt(r, pic, p);
  await mouse(page, 'mouseMoved', c, 'none', 0, 0);
  await sleep(60);
  for (let k = 1; k <= clicks; k++) {
    await mouse(page, 'mousePressed', c, 'left', 1, k);
    await sleep(50);
    await mouse(page, 'mouseReleased', c, 'left', 0, k);
    await sleep(100);
  }
}

async function wheelGuest(page, pic, p, notches, deltaY) {
  const r = await page.evaluate(RECT);
  const c = cssAt(r, pic, p);
  for (let i = 0; i < notches; i++) {
    await page.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY, pointerType: 'mouse' });
    await sleep(300);
  }
}

/** one key press (with Control held when ctrl is set); the desktop has the focus */
async function press(page, vk, name, ctrl = false) {
  await page.evaluate("document.getElementById('desktop').focus({ preventScroll: true }); true");
  await sleep(100);
  const mods = ctrl ? CTRL : 0;
  if (ctrl) await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17, modifiers: CTRL });
  await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: mods });
  await sleep(40);
  await page.send('Input.dispatchKeyEvent', { type: 'keyUp', code: name, key: name, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: mods });
  if (ctrl) await page.send('Input.dispatchKeyEvent', { type: 'keyUp', code: 'ControlLeft', key: 'Control', windowsVirtualKeyCode: 17, nativeVirtualKeyCode: 17 });
}

/** the Welcome window: "No thanks" of the usage-data box (which is drawn over it; the window stays), then the Default row is double-clicked after the list is scrolled: the project opens */
async function openDefault(page, pic, wiz) {
  const before = await page.evaluate(STATE);
  await clickGuest(page, pic, { x: wiz.x + NO_THANKS.dx, y: wiz.y + NO_THANKS.dy });
  log(`No thanks clicked at guest (${wiz.x + NO_THANKS.dx}, ${wiz.y + NO_THANKS.dy})`);
  await sleep(2500);
  await wheelGuest(page, pic, { x: wiz.x + LIST.dx, y: wiz.y + LIST.dy }, NOTCH_UP, -NOTCH_DELTA);
  await wheelGuest(page, pic, { x: wiz.x + LIST.dx, y: wiz.y + LIST.dy }, NOTCH_DOWN, NOTCH_DELTA);
  await sleep(400);
  await clickGuest(page, pic, { x: wiz.x + DEFAULT_ROW.dx, y: wiz.y + DEFAULT_ROW.dy }, 2);
  await waitFor(async () => {
    const s = await page.evaluate(STATE);
    if (s && (s.state === 'error' || s.state === 'exited')) throw new Error(`state ${s.state} while the default project opens`);
    return s && s.state === 'engine-idle' && s.title !== before.title;
  }, OPEN_LIMIT_MS, 'the default project (a new title, then engine-idle)');
  await sleep(2000);
  const after = await page.evaluate(STATE);
  if (after.state !== 'engine-idle') throw new Error(`state ${after.state} 2 s after the default project opened`);
  log(`default project open: title "${before.title}" -> "${after.title}"`);
}

/** the Piano roll: the window of class TEventEditForm whose caption starts with "Piano roll" (top-level or child: the table lists both) */
const pianoRollOf = (table) => table.top.find((t) => t.cls === PIANO_ROLL && t.title.startsWith('Piano roll')) ?? table.child.find((c) => c.cls === PIANO_ROLL && c.text.startsWith('Piano roll')) ?? null;

const describeWin = (t) => `${t.child ? 'child ' : ''}lid ${t.lid} at (${t.x}, ${t.y}) ${t.w}x${t.h} style ${hex8(t.style)} class "${t.cls}" ${t.child ? `text "${t.text}"` : `title "${t.title}"`}`;

async function edgesOf(page, win, pic) {
  const rect = await page.evaluate(RECT);
  const out = [];
  for (const dir of DIRS) {
    const pts = edgePoints(win, dir);
    let p = pts[0], covered = false;
    for (const q of pts) if (topAt(pic.table.top, q.x, q.y) === win.root) { p = q; covered = true; break; }
    const n0 = await page.evaluate(LOG_LEN);
    const actual = await cursorAt(page, pic, rect, p);
    await sleep(400);
    const trace = await page.evaluate(traceFrom(n0));
    out.push({ lid: win.lid, child: win.child, cls: win.cls, title: win.title ?? win.text, style: hex8(win.style), dir, at: p, covered, expected: EXPECTED[dir], actual, match: actual === EXPECTED[dir], trace });
    log(`edge ${dir} of ${win.child ? 'child ' : ''}lid ${win.lid} at (${p.x}, ${p.y}): expected ${EXPECTED[dir]}, actual ${actual}${covered ? '' : ' (not on top)'}`);
    for (const l of trace) log(`  trace: ${l}`);
  }
  return out;
}

/** the text check: the centres of the visible children (in table order), then a grid over the main form; the first point whose cursor is text (the I-beam) decides */
/** the text check of the window `root` (its visible children, then the grid when `grid` is set): the first point whose cursor is text decides */
async function textCheck(page, pic, root = null, grid = true) {
  const rect = await page.evaluate(RECT);
  const cands = pic.table.child.filter((c) => c.vis && c.w >= TEXT_MIN_W && c.h >= TEXT_MIN_H && c.x !== undefined && (root === null || c.root === root));
  const out = [];
  let found = null;
  const tryAt = async (p, what) => {
    const covered = topAt(pic.table.top, p.x, p.y) > 0;
    const actual = await cursorAt(page, pic, rect, p);
    out.push({ what, at: p, covered, actual });
    if (actual === 'text') found = { what, at: p };
    return actual === 'text';
  };
  for (const c of cands.slice(0, TEXT_TRIES)) {
    if (await tryAt({ x: Math.round(c.x + c.w / 2), y: Math.round(c.y + c.h / 2) }, `child ${c.cls} lid ${c.lid}`)) break;
  }
  const main = pic.table.top.find((t) => t.cls === 'TFruityLoopsMainForm');
  if (!found && main && grid) {
    for (let y = main.y + GRID_STEP_Y / 2; y < main.y + main.h && !found; y += GRID_STEP_Y) {
      for (let x = main.x + GRID_STEP_X / 2; x < main.x + main.w && !found; x += GRID_STEP_X) {
        await tryAt({ x: Math.round(x), y: Math.round(y) }, 'grid');
      }
    }
  }
  if (found) log(`text: the I-beam at (${found.at.x}, ${found.at.y}) (${found.what})`);
  log(`text check: ${out.length} points tried (${cands.length} children), ${found ? 'an I-beam was shown' : 'no I-beam'}`);
  return { found: found !== null, candidates: cands.length, points: out.length, found_at: found, out };
}

/** #1392 (--pencil): the pencil of the Piano roll, at three device pixel ratios: 1, 2, and a random one in 1.01..3.00 (2 decimals). The pointer is put over the grid (PENCIL_POINTS) until a point
 * shows a picture cursor (image-set(...) at every ratio, #1480); at 1, when none does, the Draw key D is pressed and the points are tried again. The page's picture (its PNG and the hot spot in
 * its css) is compared with the reference: the resource's own pixels read from FL_ENGINE_DLL by resource-cursor.mjs (never from the engine), or for a made cursor the copy of its bitmaps (raw).
 * The page's PNG is the reference's pixels as they are (one copy, not scaled); its hot spot is the reference's ÷ the ratio, rounded (Chromium reads the hot spot as a whole CSS px). */
const isPicture = (css) => css.startsWith('image-set(');
/** the PNG candidates of a cursor value: [{ scale, data }] (the declared resolution of the image-set entry; a url without one is 1x) */
function pngCandidates(css) {
  const out = [];
  const re = /url\("?(data:image\/png;base64,[A-Za-z0-9+/=]+)"?\)(?:\s+(\d+(?:\.\d+)?)(?:dppx|x))?/g;
  for (let m; (m = re.exec(css)) !== null;) out.push({ scale: m[2] === undefined ? 1 : Number(m[2]), data: m[1] });
  return out;
}
/** the hot spot of a cursor value (the two numbers before its comma, after the image or the image-set) */
const hotOf = (css) => { const m = /\)\s*(-?\d+)\s+(-?\d+)\s*,/.exec(css); return m ? [Number(m[1]), Number(m[2])] : null; };

async function pencilCheck(page, pic, pr) {
  const rect = await page.evaluate(RECT);
  const tries = [];
  const probe = async (round, rr) => {
    for (const [fx, fy] of PENCIL_POINTS) {
      const p = { x: Math.round(rr.x + fx * rr.w), y: Math.round(rr.y + fy * rr.h) };
      if (topAt(pic.table.top, p.x, p.y) !== rr.root) { tries.push({ round, at: p, covered: true }); continue; }
      const n0 = await page.evaluate(LOG_LEN);
      const css = await cursorAt(page, pic, rect, p);
      const trace = await page.evaluate(traceFrom(n0, PENCIL_TRACE_RE, 12));
      tries.push({ round, at: p, covered: false, css: css.slice(0, 48), trace });
      log(`pencil (${round}): cursor at (${p.x}, ${p.y}) is ${css.slice(0, 48)}`);
      for (const l of trace) log(`  trace: ${l}`);
      if (isPicture(css)) return { at: p, css };
    }
    return null;
  };
  /** the page's picture at the point, compared with the reference pixel for pixel (#1480: no copy is made at a ratio) */
  const judge = async (hit, dpr) => {
    const r = { dpr, pageDpr: null, ok: false, at: null, css: null, scale: null, source: null, cursorId: null, cursorIdMatch: null, size: null, hotspot: null, pageHotspot: null, hotMatch: false, pixelsMatch: false, mismatch: null, invert: null, counts: null, alpha: null, pageSize: null, raw: null, error: null };
    if (!hit) { r.error = 'no point of the Piano roll shows a picture cursor'; return r; }
    r.ok = hit.css.startsWith('image-set(');
    if (!r.ok) { r.error = `the cursor is not a picture at ${dpr}: ${hit.css.slice(0, 60)}`; return r; }
    r.at = hit.at;
    r.css = hit.css;
    const info = await page.evaluate('window.__app.cursor.picture ?? null');
    if (!info) { r.error = 'the page has no picture record for the cursor'; return r; }
    r.source = info.source;
    const made = /^made by /.test(info.source);
    r.cursorId = made ? null : info.cursorId;
    r.raw = info.raw ?? null;
    let ref;
    if (made) {
      if (!info.raw) { r.error = 'the made cursor has no raw copy of its bitmaps'; return r; }
      ref = referenceFromRaw(new Uint8Array(Buffer.from(info.raw, 'base64')), info.width, info.height, { hotX: info.hotX, hotY: info.hotY }, info.alpha === true);
    } else {
      const ord = /^ordinal (\d+)$/.exec(info.source);
      ref = readPeResourceCursor(FL_ENGINE_DLL, ord ? { ordinal: Number(ord[1]) } : { name: info.source });
    }
    r.cursorIdMatch = made ? null : info.cursorId === ref.cursorId;
    r.pageDpr = await page.evaluate('window.devicePixelRatio');
    r.size = [ref.width, ref.height];
    r.hotspot = [ref.hotX, ref.hotY];
    r.alpha = ref.alpha;
    r.invert = ref.counts.invert;
    r.counts = ref.counts;
    r.scale = dpr;
    const cand = pngCandidates(hit.css)[0];
    r.pageHotspot = hotOf(hit.css);
    r.hotMatch = r.pageHotspot !== null && r.pageHotspot[0] === Math.round(ref.hotX / dpr) && r.pageHotspot[1] === Math.round(ref.hotY / dpr);
    if (!cand) { r.error = `the cursor has no PNG: ${hit.css.slice(0, 60)}`; return r; }
    const png = decodePngRgba(new Uint8Array(Buffer.from(cand.data.slice(cand.data.indexOf(',') + 1), 'base64')));
    r.pageSize = [png.width, png.height];
    let mismatch = 0;
    if (png.width !== ref.width || png.height !== ref.height) mismatch = png.width * png.height;
    else for (let i = 0; i < ref.rgba.length; i += 4) if (ref.rgba[i] !== png.rgba[i] || ref.rgba[i + 1] !== png.rgba[i + 1] || ref.rgba[i + 2] !== png.rgba[i + 2] || ref.rgba[i + 3] !== png.rgba[i + 3]) mismatch++;
    r.mismatch = mismatch;
    r.pixelsMatch = mismatch === 0;
    log(`pencil at ${dpr}: ${info.source}${made ? '' : ` (RT_CURSOR ${ref.cursorId})`}, ${ref.width} x ${ref.height}, hot spot ${ref.hotX},${ref.hotY} (page ${r.pageHotspot}); png ${png.width} x ${png.height}; pixels mismatched ${mismatch}; inverted ${ref.counts.invert}`);
    return r;
  };
  let hit = await probe('the tool as the Piano roll shows it', pr);
  let keyed = false;
  if (!hit) {
    await press(page, KEY_D, 'D');
    keyed = true;
    await sleep(800);
    pic = await dump(page);
    pr = pianoRollOf(pic.table) ?? pr;
    hit = await probe('after the Draw key D', pr);
  }
  const first = await judge(hit, Number(await page.evaluate('window.devicePixelRatio')));
  const dprs = [first];
  const randomDpr = Math.round((1.01 + Math.random() * 1.99) * 100) / 100;
  for (const want of [2, randomDpr]) {
    const got = Math.round((await setRoom(page, want)) * 100) / 100;
    pic = await dump(page);
    const prd = pianoRollOf(pic.table);
    if (Math.abs(got - want) > 0.005 || !prd) { dprs.push({ dpr: got, ok: false, error: !prd ? 'the Piano roll did not show at this ratio' : `the device pixel ratio is ${got}, not ${want}` }); continue; }
    const h = await probe(`at device pixel ratio ${want}`, prd);
    dprs.push(await judge(h, got));
  }
  await setRoom(page, 1);
  pic = await dump(page);
  const out = { ok: first.ok, tries, keyed, dprs, randomDpr, at: first.at, source: first.source, cursorId: first.cursorId, cursorIdMatch: first.cursorIdMatch, size: first.size, hotspot: first.hotspot, pageHotspot: first.pageHotspot, hotMatch: first.hotMatch, pixelsMatch: first.pixelsMatch, mismatch: first.mismatch, invert: first.invert, counts: first.counts, alpha: first.alpha, css: first.css, raw: first.raw, error: first.error };
  return out;
}

async function session(page, res) {
  await setRoom(page);
  await page.evaluate(TRACE_ON);
  const nStart = await page.evaluate(LOG_LEN);
  let pic = await dump(page);
  res.windowsAtStart = pic.text.split('\n').filter(Boolean).slice(0, 60);
  res.sizableAtStart = pic.table.top.filter(isSizable).map(describeWin);
  log(`windows at the first idle: ${pic.table.top.length} top-level`);
  for (const t of pic.table.top) log(`  ${describeWin(t)}`);
  const wiz = pic.table.top.find((t) => t.cls === WIZARD);
  if (!wiz) throw new Error('no Welcome window (TWelcomeWizard) at the first idle');
  await openDefault(page, pic, wiz);
  for (let k = 0; k < 20; k++) {
    pic = await dump(page);
    if (!pic.table.top.some((t) => t.cls === PROBLEMS)) break;
    await press(page, KEY_ESCAPE, 'Escape');
    await sleep(1500);
  }
  await sleep(1500);
  await press(page, KEY_F7, 'F7');
  let pr = null;
  for (let k = 0; k < 16 && !pr; k++) {
    await sleep(500);
    pic = await dump(page);
    pr = pianoRollOf(pic.table);
  }
  res.pianoRoll = pr ? { lid: pr.lid, child: pr.child, style: hex8(pr.style), text: pr.title ?? pr.text } : null;
  log(`Piano roll: ${JSON.stringify(res.pianoRoll)}`);
  if (res.pencilOn) {
    try { res.pencil = pr ? await pencilCheck(page, pic, pr) : { ok: false, error: 'the Piano roll did not open' }; } catch (e) { res.pencilError = String(e?.message ?? e); }
    pic = await dump(page);
  }
  res.childClasses = [...new Set(pic.table.child.map((c) => c.cls))];
  log(`child classes: ${res.childClasses.join(', ')}`);
  res.sizable = [];
  res.edgeDetail = [];
  const measured = new Set();
  /** the sizable windows of the table that were not measured yet: their edges are measured now, while they are shown */
  res.styles = new Map();
  const sizeNow = async (tag, table) => {
    for (const t of [...table.top, ...table.child]) if (!res.styles.has(t.lid)) res.styles.set(t.lid, { tag, child: t.child, cls: t.cls, style: hex8(t.style), frame: (t.style & WS_THICKFRAME) !== 0 });
    const now = [...table.top, ...table.child].filter((t) => (isSizable(t) || (res.force ?? []).includes(t.lid)) && !measured.has(t.lid));
    for (const t of now) {
      measured.add(t.lid);
      log(`sizable (${tag}): ${describeWin(t)}`);
      res.sizable.push({ found: tag, lid: t.lid, child: t.child, cls: t.cls, title: t.title ?? t.text, style: hex8(t.style), x: t.x, y: t.y, w: t.w, h: t.h });
      res.edgeDetail.push(...await edgesOf(page, t, pic));
    }
  };
  pic = await dump(page);
  await sizeNow('Piano roll screen', pic.table);
  res.diagEdges = [];
  for (const t of pic.table.top.filter((t) => t.cls !== WIZARD && t.cls !== PROBLEMS && !isSizable(t)).slice(0, 8)) {
    log(`diagnostic edges (not sizable): ${describeWin(t)}`);
    res.diagEdges.push(...await edgesOf(page, t, pic));
  }
  const before = new Set(pic.table.top.map((t) => t.lid));
  res.textWindows = [];
  for (const { name, vk, ctrl } of SWEEP) {
    await press(page, vk, name.replace(/^Ctrl\+/, ''), !!ctrl);
    await sleep(1500);
    pic = await dump(page);
    const fresh = pic.table.top.filter((t) => !before.has(t.lid));
    log(`after ${name}: ${pic.table.top.length} top-level: ${pic.table.top.map((t) => `${t.lid} ${t.cls}`).join(', ')}`);
    await sizeNow(name, pic.table);
    // a window the key opened is also checked for the I-beam while it is shown (a text field of it)
    for (const w of fresh) {
      const tw = await textCheck(page, pic, w.lid, false);
      res.textWindows.push({ key: name, lid: w.lid, cls: w.cls, candidates: tw.candidates, points: tw.points, found: tw.found });
      if (tw.found && !res.textFound) res.textFound = tw.found_at;
    }
    await press(page, KEY_ESCAPE, 'Escape');
    await sleep(1000);
  }
  pic = await dump(page);
  // the diagnostic edge read of the main form (its frame is not WS_THICKFRAME: not an acceptance sample)
  const mainForm = pic.table.top.find((t) => t.cls === 'TFruityLoopsMainForm');
  if (mainForm) res.mainFormEdges = await edgesOf(page, mainForm, pic);
  res.styleTrace = await page.evaluate(traceFrom(nStart, '/GWL_STYLE/', 200));
  res.setPosCount = await page.evaluate(countFrom(nStart, '/SetWindowPos/'));
  // the sizable windows that the table lists, whatever their visibility (a diagnostic of the child lines)
  for (const t of [...pic.table.top, ...pic.table.child]) if (!res.styles.has(t.lid)) res.styles.set(t.lid, { tag: 'end', child: t.child, cls: t.cls, style: hex8(t.style), frame: (t.style & WS_THICKFRAME) !== 0 });
  res.thickframeAny = [...res.styles.entries()].filter(([, v]) => v.frame).map(([lid, v]) => ({ lid, ...v }));
  res.windowStyles = [...res.styles.entries()].map(([lid, v]) => ({ lid, ...v }));
  const tc = await textCheck(page, pic);
  // textOk: an I-beam was shown at some point of any search (the sweep's windows, then this last one); textEnd: this last search found one (the two can differ: the reason goes in the report)
  res.textFoundAt = res.textFound ?? tc.found_at ?? null;
  res.textOk = res.textFoundAt !== null;
  res.textEnd = tc.found;
  res.textPoints = tc.points;
  res.textCandidates = tc.candidates;
  res.textDetail = tc.out;
  const rec = await page.evaluate(RECORD);
  res.cursorRecord = rec.cursorRecord;
  res.cursorAtEnd = rec.cursorAtEnd;
  res.windowsAtEnd = pic.text.split('\n').filter(Boolean).slice(0, 120);
}

/** #1392 (--pencil): the pencil fields of the JSON line; null when the pencil was not asked for (--pencil absent) */
function pencilFields(res) {
  const p = res.pencil ?? null;
  /** the device pixel ratios: [0] 1, [1] 2, [2] a random ratio in 1.01..3.00; each with the page's cursor and its match with the reference (the judged fields are ok at [0] and [2], and dpr at [1]) */
  const dprs = (p?.dprs ?? []).map((d) => ({ dpr: d.dpr, pageDpr: d.pageDpr ?? null, ok: d.ok === true, at: d.at ?? null, scale: d.scale ?? null, source: d.source ?? null, pageSize: d.pageSize ?? null, hotspotMatch: d.hotMatch === true, pixelsMatch: d.pixelsMatch === true, mismatch: d.mismatch ?? null, css: d.css ? d.css.slice(0, 160) : null, error: d.error ?? null }));
  return {
    pencilOk: res.pencilOn ? p?.ok === true : null,
    pencilResource: p?.source ?? null,
    pencilCursorId: p?.cursorId ?? null,
    pencilCursorIdMatch: p?.cursorIdMatch ?? null,
    pencilSize: p?.size ?? null,
    pencilHotspot: p?.hotspot ?? null,
    pencilPageHotspot: p?.pageHotspot ?? null,
    pencilHotspotMatch: p ? p.hotMatch === true : null,
    pencilPixelsMatch: p ? p.pixelsMatch === true : null,
    pencilMismatch: p?.mismatch ?? null,
    pencilInvertPixels: p?.invert ?? null,
    pencilAlpha: p?.alpha ?? null,
    pencilCounts: p?.counts ?? null,
    pencilAt: p?.at ?? null,
    pencilKeyed: p ? p.keyed === true : null,
    pencilTries: p?.tries ?? [],
    pencilDprs: res.pencilOn ? dprs : null,
    pencilRandomDpr: p?.randomDpr ?? null,
    /** the page's cursor as it was shown (the css, with its PNG) and the raw copy of the made cursor's bitmaps (base64, six bytes per pixel) */
    pencilCss: p?.css ? p.css.slice(0, 160) : null,
    pencilRaw: p?.raw ?? null,
    pencilError: p?.error ?? res.pencilError ?? null,
  };
}

async function main() {
  if (!process.argv[2]) {
    process.stderr.write('usage: node tools/cursor/measure.mjs <absolute path to the bundle directory>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(process.argv[2]);
  const res = {};
  const f = process.argv.indexOf('--force-sizable');
  res.force = f > 0 ? String(process.argv[f + 1] ?? '').split(',').filter(Boolean).map(Number) : [];
  res.pencilOn = process.argv.includes('--pencil');
  let ran = false;
  const { out } = await bootBundle(bundleDir, {
    afterIdle: async (page) => {
      ran = true;
      await session(page, res);
    },
  });
  if (!ran && !out.error) out.error = `the session did not run: state ${out.state}`;
  const edgeDetail = res.edgeDetail ?? [];
  const edgeWindows = (res.sizable ?? []).length;
  const unmappedNames = [...new Set((res.cursorRecord?.unmapped ?? []).map((u) => u.name))];
  const final = {
    error: out.error, state: out.state, standIn: out.standIn, nproc: os.availableParallelism(), loadavg: os.loadavg(),
    windowsAtStart: res.windowsAtStart ?? [], sizableAtStart: res.sizableAtStart ?? [], pianoRoll: res.pianoRoll ?? null, childClasses: res.childClasses ?? [],
    sizable: res.sizable ?? [], edgeWindows, edgeSamples: edgeDetail.length, edgeMatches: edgeDetail.filter((d) => d.match).length, edgeDetail, diagEdges: res.diagEdges ?? [], styleTrace: res.styleTrace ?? [], setPosCount: res.setPosCount ?? 0,
    textOk: res.textOk ?? false, textFoundAt: res.textFoundAt ?? null, textEnd: res.textEnd ?? false, textPoints: res.textPoints ?? 0, textCandidates: res.textCandidates ?? 0, textDetail: res.textDetail ?? [], textWindows: res.textWindows ?? [],
    mainFormEdges: res.mainFormEdges ?? null, thickframeAny: res.thickframeAny ?? [], windowStyles: res.windowStyles ?? [],
    unresolved: res.cursorRecord?.unresolved ?? null, unmapped: unmappedNames.length, unmappedNames, madeByGuest: res.cursorRecord?.made ?? null,
    cursorRecord: res.cursorRecord ?? null, cursorAtEnd: res.cursorAtEnd ?? null, windowsAtEnd: res.windowsAtEnd ?? [],
    ...pencilFields(res),
  };
  if (edgeDetail.length !== 8 * edgeWindows) final.error = final.error ?? `edge samples ${edgeDetail.length} for ${edgeWindows} windows`;
  process.stdout.write(JSON.stringify(final) + '\n');
  process.exitCode = final.error ? 1 : 0;
}

// the measurement runs when this file is the program
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) await main();
