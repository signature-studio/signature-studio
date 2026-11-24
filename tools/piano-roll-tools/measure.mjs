// piano-roll-tools: the measurement of the Piano roll's tools (#1386 chord progression, #1387 riff machine). It opens the default project, the Piano roll (F7), the main window's TOOLS menu and one tool; for the chord progression it presses "Start creating".
// It records and measures only: the product is not changed, and no branch for the measurement is in it.
// Usage: node tools/piano-roll-tools/measure.mjs <absolute path to the bundle directory raw/<name>> --tool <chord-progression|riff-machine> --dprs <list> > tools.jsonl
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
// Each JSON line: tool, dpr, error (the page's error, or the tool's failure; null when none), engine (the engine's state 3 s after the action), stopped (the state is stopped, exited or error, or a "== stop:" line is in the log),
//   presentedDelta (the desktop's presents in the 3 s after the action: window.__app.deskStats.presents), notes (null: not counted here; notesNote says why), missingPaths (the missing-path: lines since the first idle),
//   stopLines (the engine's stop lines: "== stop:" (also "returned from entry", which a run that goes on prints too), "fle:", "fruity: exit code", "runtime message:"), windowNew (the top-level windows that appeared with the action: { cls, title, w, h, visible, painted, paintedShare }; null when none),
//   elapsedMs (the page's timeline: the first idle, in ms from the navigation), preBootBytes (the assetsReport bytes), nproc, loadavg, room (the guest's screen, window.__app.screen.guest),
//   logTail (the last 12 lines of the engine's log, in order: the order of the stop), steps (what the tool did, in order).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
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
const KEY_ESCAPE = 27, KEY_ENTER = 13, KEY_DOWN = 40, KEY_F7 = 118;
// the label TOOLS in the menu bar, as fractions of the bar's rectangle (the 7th of its 8 labels)
const TOOLS_LABEL = { fx: 0.819, fy: 0.45 };
// the items of the TOOLS drop-down by the Down presses that reach them (the order: Browser find, One-click audio recording, Remix a song, Riff machine, Chord progression tool, ...)
const TOOL_ITEMS = {
  'chord-progression': { downs: 5, windowClass: 'TPRChordToolForm' },
  'riff-machine': { downs: 4, windowClass: null },
};
// "Start creating" in the chord progression tool's window: its centre as fractions of the window's rectangle (measured from the picture at DPR 1)
const START_CREATING = { fx: 0.5, fy: 0.847 };
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
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;

const log = (s) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${s}\n`);

/** the windows of a window-table text: { child, lid, parent, x, y, w, h, cls, text, vis } (x, y are relative to the parent for a child) */
export function parseWindows(text) {
  return text.split('\n').map((l) => {
    const m = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (!m) return null;
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    return { child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]), cls: cls ? cls[1] : '', text: cap ? cap[1] : '', vis: /vis 1/.test(m[8]) };
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

/** the share of a picture's rectangle whose pixels differ from its top-left pixel (0 .. 1): how much of the window has any drawing on it */
export function paintedShare(picture, rect) {
  const bytes = Buffer.from(picture.base64, 'base64');
  const stride = bytes.length / picture.height;
  const x0 = Math.max(0, rect.x), y0 = Math.max(0, rect.y);
  const x1 = Math.min(picture.width, rect.x + rect.w), y1 = Math.min(picture.height, rect.y + rect.h);
  if (x1 <= x0 || y1 <= y0) return 0;
  const bg = bytes.readUInt32BE(y0 * stride + x0 * 4);
  let n = 0, total = 0;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    total++;
    if (bytes.readUInt32BE(y * stride + x * 4) !== bg) n++;
  }
  return Math.round((n / total) * 1000) / 1000;
}

/** one start of the bundle at a devicePixelRatio, and the action of one tool; returns the JSON object of the run */
async function runOnce(bundleDir, tool, dpr) {
  const item = TOOL_ITEMS[tool];
  const steps = [];
  const res = {
    tool, dpr, error: null, engine: null, stopped: null, presentedDelta: null, notes: null,
    notesNote: 'not counted by this tool: the notes of the Piano roll are not read from the guest memory here',
    missingPaths: [], stopLines: [], windowNew: null, elapsedMs: null, preBootBytes: null, nproc: os.availableParallelism(), loadavg: null, room: null, logTail: [], steps,
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
        await press(KEY_ENTER, 'Enter');
        steps.push(`${item.downs} Down, Enter`);
        await sleep(ACTION_WAIT_MS);
        const afterItem = await wins();
        let toolWindow = null;
        if (item.windowClass) {
          toolWindow = afterItem.find((w) => w.cls === item.windowClass) ?? null;
          if (!toolWindow) throw new Error(`no ${item.windowClass} window after the item`);
          steps.push(`tool window: lid ${toolWindow.lid} "${toolWindow.text}" at ${toolWindow.x},${toolWindow.y} ${toolWindow.w}x${toolWindow.h}`);
        }
        if (tool === 'chord-progression') {
          const at = absolutePosition(afterItem, toolWindow);
          const c = { x: at.x + START_CREATING.fx * toolWindow.w, y: at.y + START_CREATING.fy * toolWindow.h };
          presents0 = (await evaluate(STATE)).presents;
          await clickAt(c.x, c.y);
          steps.push(`Start creating clicked at guest (${Math.round(c.x)}, ${Math.round(c.y)})`);
          await sleep(ACTION_WAIT_MS);
        }
        const end = await evaluate(STATE);
        res.presentedDelta = presents0 !== null && end.presents !== null ? end.presents - presents0 : null;
        const added = afterItem.filter((w) => !beforeItem.includes(w.lid) && !w.child);
        if (added.length) {
          const picture = await evaluate(PICTURE);
          res.windowNew = added.map((w) => {
            const share = paintedShare(picture, { x: w.x, y: w.y, w: w.w, h: w.h });
            return { cls: w.cls, title: w.text, w: w.w, h: w.h, visible: w.vis, painted: share > 0.05, paintedShare: share };
          });
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
  const positional = argv.filter((a, i) => !a.startsWith('--') && argv[i - 1] !== '--tool' && argv[i - 1] !== '--dprs');
  if (positional.length !== 1 || !TOOL_ITEMS[tool] || dprs.some((d) => !(d >= 0.5 && d <= 4))) {
    process.stderr.write('usage: node tools/piano-roll-tools/measure.mjs <absolute path to the bundle directory raw/<name>> --tool <chord-progression|riff-machine> --dprs <1,1.37,...>\n');
    process.exit(2);
  }
  const bundleDir = path.resolve(positional[0]);
  let failed = false;
  for (const dpr of dprs) {
    log(`start: tool ${tool}, dpr ${dpr}`);
    let line;
    try {
      line = await runOnce(bundleDir, tool, dpr);
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
