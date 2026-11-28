// crash-hunt (Issue #1405, extended by #1417): walks the FL screen's operations in the lifted engine's page and records what each one did, one probe per path and DPR.
// Usage: node tools/crash-hunt/hunt.mjs --out <directory> [--budget 50m] [--bundle <raw/<name>>] [--seed 20251125] [--resume] [--only <id>[,<id>...]]   (run through tools/crash-hunt/run.sh; needs app/node_modules and app/dist, as boot-bundle.mjs does).
//   --budget: a number with s, m or h (50m). --bundle: the bundle directory (default lift/out-app/bundle/raw/fl-Oz). --seed: the seed of the random DPR (1.01 to 2.24).
//   --resume: continues from <out>/crash-hunt-state.json. --skip-startup: a test aid (the startup check is not run; startup is then null).
//   --only <id>[,<id>...] (e.g. View/11/1 or File/9/11,View/11/1): probes those items only (and the items on their paths, which open their submenus), in the DPR 1 walk, and stops after that walk.
// Walk: two walks, one per DPR (1, then the random one). Each walk: (a) the menus: the 8 top labels of the bar (their places are the bar's fractions, the bar is found in the window table),
//   every item of each drop-down, and the submenus opened by an item (up to 3 levels); the right-click menus of the four windows (Channel rack, Mixer, Playlist, Browser) and of the Mixer's master insert (group plugin: its submenus);
//   (b) the keys F5 to F12, Alt+F8, Ctrl+O/E/P/F/I, Space; (c) the top bar's parts (left, right, wheel, drag: group top), the Settings window's tabs (F10: group settings) and the sections of each page,
//   the Browser's tab strip (eight equal slots: group browser). The places are read from the window table each session; the item texts are NOT read (no OCR: the paths say "#n", the position in its menu).
//   The File menu's last item (Exit) is the last probe of a walk. Items are chosen by the keys: Down presses (the count of the Down presses to reach an item) and Enter.
//   The new groups and the re-checks of the four fingerprints of #1405 are put at the front of the queue (extraTasks; a walk enumerated before #1417 is extended by extend()).
// A session is one start of the bundle (boot-bundle's bootBundle): the walk runs in its afterIdle; a stop (exited, error, or no idle within 20 s = hung) ends the session, and the next session starts the bundle again and goes on from the next path (restartFailures counts starts that failed after a stop).
// An expected exit (expectedExit: the File menu's last item ends the engine with no stop line) is recorded with expected: true and is not a stop (stopProbes, fingerprints).
// The state is written to <out>/crash-hunt-state.json after every probe. The findings are <out>/crash-hunt-<UTC date>.json and .md (the fingerprints: the engine's last "fle: " line, first 80 characters, numbers and addresses replaced with #; "(fle 無し)" without one; "(停滞)" for a hang).
// A probe's observation: after the action, 2 s, then the engine state up to 20 s after the action (exited: stopped; error or the page's error: error; running: hung), the windows that are new (they are closed with Escape, then Enter), and the engine log's new lines.
// Not done here (remaining, with the reason in the findings): the Language and Theme switches (no window in the table per language or theme), the Mixer's other inserts, and the built-in plugins one by one (#1371's tool is not in the repository: the Mixer insert's submenus are walked instead).
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { bootBundle } from '../boot-check/boot-bundle.mjs';
import { sleep } from '../boot-check/cdp.mjs';
import { engineStopFacts, sanitizeEngineText } from '../../app/src/perf/exit-message.ts';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const ROOM_W = 1836, ROOM_H = 1056;
const VIEW_START = { width: 2176, height: 1092 };
const WELCOME_TITLE = 'Welcome to FL Studio - in ?trial mode?';
const PROBLEMS_CLASS = 'TMissingPluginsForm';
const POPUP = 'TQuickPopupMenuWindow';
const MENU_BAR = 'TNewMenu';
const LABELS = [['File', 0.051], ['Edit', 0.147], ['Add', 0.235], ['Patterns', 0.375], ['View', 0.525], ['Options', 0.666], ['Tools', 0.819], ['Help', 0.94]];
const LABEL_FY = 0.45;
// the right-click targets: a window of the main form by its class (and caption); the centre of its rectangle is clicked
const RIGHT_TARGETS = [
  { name: 'Channel rack', cls: 'TStepSeqForm' },
  { name: 'Mixer', cls: 'TFXForm' },
  { name: 'Playlist', cls: 'TEventEditForm', text: 'Playlist' },
  { name: 'Browser', cls: 'TSampleListForm' },
];
const INSERT_ROOT = 'right-click Mixer insert';
// the Mixer's insert strips (the TVectorPanel of the mixer that holds them: 584 x 465 px, the first strip is a column of 1/13 of its width, its name is at 15 % of its height; the Playlist covers the mixer's right part)
const STRIPS_W = 400, STRIPS_H = 300, STRIPS_N = 13, STRIP_NAME_FY = 0.15;
// the fingerprints of #1405 (the re-check at the front of the queue): File > #9 > #11 (#1415), Add > #14 (#1411, ok now), View > #11 > #1 (#1416), File > #10 (Exit: the expected end)
const RECHECK = [['File', 9, 11], ['Add', 14], ['View', 11, 1], ['File', 10]];
const TOP_ACTIONS = ['left', 'right', 'wheel', 'drag'];
const TOP_CLASSES = new Set(['TQuickEditToolbarPanel', 'TNewMenu', 'TWPControl']);
const TOP_BAR_CLASS = 'TQuickControlBar';
const SETTINGS_CLASS = 'TMIDIForm', SETTINGS_TABS = 16, SECTION_MAX = 6;
const BROWSER_CLASS = 'TSampleListForm', BROWSER_TABS = 8;
const MIXER_CLASS = 'TFXForm';
const DRAG_PX = 24, WHEEL_DELTA = -120;
// the keys: F5 to F12, Alt+F8, Ctrl+O, Ctrl+E, Ctrl+P, Ctrl+F, Ctrl+I, Space (modifiers: 1 Alt, 2 Ctrl)
const KEYS = [
  ...[116, 117, 118, 119, 120, 121, 122, 123].map((vk) => ({ name: `F${vk - 111}`, code: `F${vk - 111}`, key: `F${vk - 111}`, vk, mods: 0 })),
  { name: 'Alt+F8', code: 'F8', key: 'F8', vk: 119, mods: 1 },
  ...[['O', 79], ['E', 69], ['P', 80], ['F', 70], ['I', 73]].map(([c, vk]) => ({ name: `Ctrl+${c}`, code: `Key${c}`, key: c.toLowerCase(), vk, mods: 2 })),
  { name: 'Space', code: 'Space', key: ' ', vk: 32, mods: 0 },
];
const KEY_F10 = KEYS.find((k) => k.name === 'F10');
const KEY_ESC = { code: 'Escape', key: 'Escape', vk: 27, mods: 0 };
const KEY_ENTER = { code: 'Enter', key: 'Enter', vk: 13, mods: 0 };
const KEY_DOWN = { code: 'ArrowDown', key: 'ArrowDown', vk: 40, mods: 0 };
const DOWN_WAIT_MS = 200, NAV_WAIT_MS = 120, OPEN_WAIT_MS = 600, OPEN_SETTLE_MS = 500, OBSERVE_MS = 2000, SETTLE_MS = 20000, POLL_MS = 400, SETTINGS_OPEN_MS = 2500, PAGE_WAIT_MS = 1500;
const MAX_ITEMS = 90, MAX_DEPTH = 3;
const FILE_LABEL = 'File';
const STARTUP_LIMIT = { elapsedMs: 12500, preBootBytes: 330000000 };
const TIMEOUT_EVAL = /^timeout \d+ ms: Runtime\.evaluate/;
const TIMEOUT_ANY = /^timeout \d+ ms:/;
const EXIT_REASON = 'File の最後の項目 Exit: FL が終了する(正しい終了。停止に数えない)';

const SNAP = `(async () => { const d = await window.__app.deskDumpBase64(true); return { windows: d.windows, width: d.width, height: d.height, base64: d.base64 }; })()`;
const STATE = `(() => { const a = window.__app; if (!a) return { state: 'unset', error: null, logLength: 0, title: null }; return { state: a.state, error: a.error ? String(a.error.what ?? a.error.message ?? a.error).slice(0, 300) : null, logLength: a.engineLog().length, title: document.title }; })()`;
const RECT = `(() => { const r = document.getElementById('desktop').getBoundingClientRect(); const g = window.__app.screen.guest; return { x: r.x, y: r.y, w: r.width, h: r.height, gw: g.width, gh: g.height }; })()`;
const STAGE = `(() => { const s = document.getElementById('stage').getBoundingClientRect(); return { sw: s.width, sh: s.height, iw: window.innerWidth, ih: window.innerHeight }; })()`;
const FOCUS = `(() => { document.getElementById('desktop').focus({ preventScroll: true }); return true; })()`;
const LOG_FROM = (n) => `window.__app.engineLog().slice(${n})`;
const LOG_TAIL = `window.__app.engineLog().slice(-20)`;
const LOG_ALL = `window.__app.engineLog()`;
const PAGE_DIALOG = `!!document.getElementById('filedlg-input') || !!document.getElementById('fx-input-files')`;
const PAGE_DIALOG_CANCEL = `(() => { const b = document.getElementById('filedlg-cancel'); if (b) b.click(); return !!b; })()`;

const log = (...a) => process.stderr.write(`${new Date().toISOString().slice(11, 19)} ${a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' ')}\n`);

/** the windows of a window table: { child, lid, parent, x, y, w, h, cls, text, vis } (a child's x, y are relative to its parent) */
export function parseWindows(text) {
  return text.split('\n').map((l) => {
    const m = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (!m) return null;
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    return { child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]), cls: cls ? cls[1] : '', text: cap ? cap[1] : '', vis: /vis 1/.test(m[8]) };
  }).filter(Boolean);
}

/** the guest position of a window (a child's position is added to its parents') */
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

/** the windows inside a window (its children, their children ...) */
export function under(list, lid) {
  const out = [];
  let frontier = [lid];
  while (frontier.length) {
    const kids = list.filter((w) => frontier.includes(w.parent) && !out.some((o) => o.lid === w.lid));
    out.push(...kids);
    frontier = kids.map((k) => k.lid);
  }
  return out;
}

/** the top bar's parts: the visible windows of the top bar's classes that lie inside the top bar (a TQuickControlBar at the top of the room: its height is the bar's) */
export function topBarParts(list) {
  const bar = list.find((w) => w.cls === TOP_BAR_CLASS && w.vis && w.h > 0 && w.w > 1000 && absolutePosition(list, w).y === 0);
  if (!bar) return [];
  return list.filter((w) => w.vis && w.w > 0 && w.h > 0 && w.lid !== bar.lid && TOP_CLASSES.has(w.cls)).filter((w) => {
    const p = absolutePosition(list, w);
    return p.y >= 0 && p.y + w.h <= bar.h;
  });
}

/** the fingerprint of a stop: the last "fle: " line without digits and addresses, 80 characters at most */
export function fingerprint(result, stop) {
  if (result === 'hung') return '(停滞)';
  const fle = stop && stop.lastFle;
  if (!fle) return '(fle 無し)';
  return fle.replace(/0x[0-9a-fA-F]+/g, '#').replace(/\d+/g, '#').replace(/\s+/g, ' ').slice(0, 80);
}

/** the expected end: the File menu's last item (the count of File's items, from the walk's counts) ends the engine with no stop line (state exited) */
export function expectedExit(p, nodes) {
  const n = nodes?.[FILE_LABEL]?.n;
  return !!n && p.result === 'stopped' && p.state === 'exited' && !p.stop && p.path === `File > #${n}`;
}

/** the random DPR of a seed: 1.01 to 2.24 with two decimals (mulberry32) */
export function randomDpr(seed) {
  let a = seed >>> 0;
  a = (a + 0x6d2b79f5) >>> 0;
  let t = Math.imul(a ^ (a >>> 15), 1 | a);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  const r = ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  return Math.round((1.01 + r * 1.23) * 100) / 100;
}

const pathText = (p) => p.map((x, i) => (i === 0 ? x : `#${x}`)).join(' > ');
const nodeKey = (p) => p.join('/');
/** the group of a menu path: a (the menus of the bar), c (the right-click menus), plugin (the submenus of the Mixer's insert), top (the insert's own items) */
const groupOf = (p) => {
  if (p[0] === INSERT_ROOT) return p.length >= 3 ? 'plugin' : 'top';
  return p[0].startsWith('right-click') ? 'c' : 'a';
};
const taskId = (t) => t.id ?? (t.kind === 'key' ? `key ${t.key.name}` : nodeKey(t.path));
const taskLabel = (t) => (t.kind === 'key' ? `key ${t.key.name}` : t.kind === 'menu' ? pathText(t.path) : t.label);
const taskGroup = (t) => (t.kind === 'key' ? 'b' : t.kind === 'menu' ? groupOf(t.path) : t.group);

function parseArgs(argv) {
  const get = (f) => { const i = argv.indexOf(f); return i >= 0 ? argv[i + 1] : undefined; };
  const budgetText = get('--budget') ?? '50m';
  const m = /^(\d+(?:\.\d+)?)(s|m|h)$/.exec(budgetText);
  if (!m) throw new Error(`--budget ${budgetText}: use a number with s, m or h`);
  const out = get('--out');
  if (!out) throw new Error('--out is required');
  const only = get('--only');
  if (only !== undefined && !only.split(',').every((id) => /^[^/,]+(\/\d+)+$/.test(id))) throw new Error(`--only ${only}: use menu item ids such as View/11/1 (a comma list of them)`);
  return {
    budgetMs: Number(m[1]) * { s: 1000, m: 60000, h: 3600000 }[m[2]],
    out: path.resolve(out),
    bundle: path.resolve(get('--bundle') ?? path.join(repo, 'lift/out-app/bundle/raw/fl-Oz')),
    seed: Number(get('--seed') ?? 20251125),
    resume: argv.includes('--resume'),
    skipStartup: argv.includes('--skip-startup'),
    only: only ?? null,
  };
}

/** --only <id>[,<id>...] (menu items such as View/11/1): the one walk probes the items on the path to each of them (their submenus open the next level) and the item itself; the other items are not probed. */
export function onlyKeep(only, id) { return !only || only.split(',').some((o) => o === id || o.startsWith(`${id}/`)); }

/** the startup check: welcome-flow.mjs flow A (its line only); ok when not trapped, the first idle within 12.5 s and the start-up bytes within the limit */
function startupCheck(bundle) {
  const r = spawnSync(process.execPath, [path.join(repo, 'tools/smoke/welcome-flow.mjs'), bundle], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, timeout: 1500000 });
  const line = String(r.stdout ?? '').split('\n').find((l) => l.startsWith('{') && l.includes('"flow":"A"'));
  if (!line) return { flow: 'A', ok: false, error: `no flow A line (exit ${r.status}, signal ${r.signal})`, trapped: null, elapsedMs: null, preBootBytes: null };
  const a = JSON.parse(line);
  const ok = a.trapped === false && a.elapsedMs !== null && a.elapsedMs <= STARTUP_LIMIT.elapsedMs && a.preBootBytes !== null && a.preBootBytes <= STARTUP_LIMIT.preBootBytes;
  return { flow: 'A', ok, error: a.error, trapped: a.trapped, elapsedMs: a.elapsedMs, preBootBytes: a.preBootBytes, state: a.state };
}

class Session {
  constructor({ send, evaluate, ctx, walk, deadline }) {
    this.send = send;
    this.evaluateRaw = evaluate;
    this.ctx = ctx;
    this.walk = walk;
    this.deadline = deadline;
    this.base = new Set();
    this.roots = new Map();
    this.clean = false;
    this.strip = null;
  }
  /** an evaluate; a timeout of the Runtime.evaluate is retried once (counted as cdpRetries) */
  async ev(expression) {
    for (let busy = 0; ; busy++) {
      try { return await this.evaluateRaw(expression); } catch (e) {
        const msg = String(e?.message ?? e);
        if (/ask again/.test(msg) && busy < 100) { await sleep(100); continue; }
        if (!TIMEOUT_EVAL.test(msg)) throw e;
        this.ctx.cdpRetries++;
        log(`cdp retry: ${msg.slice(0, 80)}`);
        return await this.evaluateRaw(expression);
      }
    }
  }
  /** the windows of the page's picture (one read: the picture comes with the list, so a read costs a picture) */
  async picture() { const d = await this.ev(SNAP); return { wins: parseWindows(d.windows), width: d.width, height: d.height, base64: d.base64 }; }
  async windows() { return (await this.picture()).wins; }
  /** the SHA-1 of the picture's pixels in a window's rectangle (the other parts of the picture can change with time: the clocks and the meters) */
  async rectHash(box) {
    const pic = await this.picture();
    const bytes = Buffer.from(pic.base64, 'base64');
    const stride = bytes.length / pic.height;
    const x0 = Math.max(0, box.x), y0 = Math.max(0, box.y), x1 = Math.min(pic.width, box.x + box.w), y1 = Math.min(pic.height, box.y + box.h);
    const h = crypto.createHash('sha1');
    for (let y = y0; y < y1; y++) h.update(bytes.subarray(y * stride + 4 * x0, y * stride + 4 * x1));
    return h.digest('hex');
  }
  async state() { return this.ev(STATE); }
  async press(k) {
    const p = { code: k.code, key: k.key, windowsVirtualKeyCode: k.vk, nativeVirtualKeyCode: k.vk, modifiers: k.mods ?? 0 };
    await this.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...p });
    await sleep(40);
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', ...p });
  }
  /** a guest point (its pixel's centre) in CSS pixels of the page */
  async guestPoint(gx, gy) {
    const r = await this.ev(RECT);
    return { x: r.x + (gx + 0.5) * (r.w / r.gw), y: r.y + (gy + 0.5) * (r.h / r.gh) };
  }
  async clickGuest(gx, gy, button = 'left') {
    const c = await this.guestPoint(gx, gy);
    const mask = button === 'right' ? 2 : 1;
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button, buttons: mask, clickCount: 1, pointerType: 'mouse' });
    await sleep(50);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button, buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(100);
  }
  async wheelGuest(gx, gy) {
    const c = await this.guestPoint(gx, gy);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: c.x, y: c.y, deltaX: 0, deltaY: WHEEL_DELTA, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(200);
  }
  async dragGuest(gx, gy) {
    const a = await this.guestPoint(gx, gy);
    const b = await this.guestPoint(gx, gy + DRAG_PX);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: a.x, y: a.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: a.x, y: a.y, button: 'left', buttons: 1, clickCount: 1, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, button: 'left', buttons: 1, pointerType: 'mouse' });
    await sleep(40);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: b.x, y: b.y, button: 'left', buttons: 1, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: b.x, y: b.y, button: 'left', buttons: 0, clickCount: 1, pointerType: 'mouse' });
    await sleep(100);
  }
  /** the windows that are new since the baseline: the top-level ones, and the child ones that are shown; and the drop-downs among them (the newest is the one that is open last) */
  newOf(wins) {
    const newWins = wins.filter((w) => !this.base.has(w.lid) && (!w.child || w.vis));
    const pops = newWins.filter((w) => w.cls === POPUP).sort((a, b) => a.lid - b.lid);
    return { newWins, pops };
  }
  async openRoot(root) {
    const r = this.roots.get(root);
    await this.clickGuest(r.x, r.y, r.kind === 'label' ? 'left' : 'right');
    await sleep(OPEN_WAIT_MS);
  }
  /** Escape closes the menus and the windows that are new (Enter after the third Escape); returns what is still open after eight rounds (a page dialog is named too) */
  async closeAll() {
    await this.ev(FOCUS);
    for (let i = 0; i < 8; i++) {
      const { newWins } = this.newOf(await this.windows());
      const dlg = await this.pageDialog();
      if (newWins.length === 0 && !dlg) { this.clean = true; return []; }
      if (dlg) await this.ev(PAGE_DIALOG_CANCEL);
      else await this.press(KEY_ESC);
      await sleep(500);
      if (i >= 3 && !dlg) await this.press(KEY_ENTER);
      await sleep(300);
    }
    const { newWins } = this.newOf(await this.windows());
    const left = [...newWins];
    if (await this.pageDialog()) left.push({ lid: -1, cls: 'page dialog (the file box)', child: false, text: '' });
    this.clean = false;
    return left;
  }
  async pageDialog() { return this.ev(PAGE_DIALOG); }
  /** how many items a drop-down has: the Down presses until a picture comes back (the highlight went round). The picture before any Down is the first. The distance between the first repeated picture and its first occurrence is the count (the first presses can differ from the cycle: the repeat is looked for from the start). pre: the first picture itself came back (the highlight was on the first item before any Down). */
  async countPopup(pop) {
    await sleep(OPEN_SETTLE_MS);
    const box = { x: pop.x, y: pop.y, w: pop.w, h: pop.h };
    const seen = [await this.rectHash(box)];
    for (let k = 1; k <= MAX_ITEMS; k++) {
      await this.press(KEY_DOWN);
      await sleep(DOWN_WAIT_MS);
      const h = await this.rectHash(box);
      const i = seen.indexOf(h);
      if (i < 0) { seen.push(h); continue; }
      return { n: k - i, pre: i === 0 };
    }
    return { n: null, pre: false, note: `no repeat within ${MAX_ITEMS} Down presses` };
  }
  /** the Down presses and the Enters that reach the item path p (p[0] the root, p[1..] the item numbers). The last Enter is pressed when lastEnter is true (it is the action). */
  async navigate(p, lastEnter) {
    if (!this.clean) await this.closeAll();
    await this.openRoot(p[0]);
    for (let d = 1; d < p.length; d++) {
      const pre = this.walk.nodes[nodeKey(p.slice(0, d))]?.pre ?? false;
      const presses = p[d] - (pre ? 1 : 0);
      for (let j = 0; j < presses; j++) { await this.press(KEY_DOWN); await sleep(NAV_WAIT_MS); }
      if (d < p.length - 1 || lastEnter) { await this.press(KEY_ENTER); await sleep(OPEN_WAIT_MS); }
    }
  }
  async settle(untilMs) {
    const t0 = Date.now();
    for (;;) {
      const st = await this.state();
      if (st.state === 'exited' || st.state === 'error' || st.error || st.state === 'engine-idle') return st;
      if (Date.now() - t0 >= untilMs) return st;
      await sleep(POLL_MS);
    }
  }
  /** the observation after an action: the engine (2 s, then up to 20 s in all), the windows that are new, and the engine's log lines since the action */
  async observe(item) {
    await sleep(OBSERVE_MS);
    const st = await this.settle(SETTLE_MS - OBSERVE_MS);
    let result = 'ok';
    if (st.state === 'exited') result = 'stopped';
    else if (st.state === 'error' || st.error) result = 'error';
    else if (st.state !== 'engine-idle') result = 'hung';
    const lines = await this.ev(LOG_FROM(item.logFrom));
    const full = await this.ev(LOG_ALL);
    const stopFacts = result === 'ok' ? null : engineStopFacts(full);
    const { newWins, pops } = this.newOf(await this.windows());
    const dlg = await this.pageDialog();
    item.result = result;
    item.state = st.state;
    item.stop = stopFacts ? stopFacts.stop : null;
    item.fingerprint = result === 'ok' ? null : fingerprint(result, stopFacts?.stop ?? null);
    item.newWindows = newWins.map((w) => ({ cls: w.cls, title: sanitizeEngineText(w.text, 80) }));
    item.pops = pops;
    item.logLines = lines.length;
    item.logTail = (await this.ev(LOG_TAIL)).map((l) => sanitizeEngineText(l, 200));
    item.dialog = dlg;
    this.clean = newWins.length === 0 && !dlg;
    return item;
  }
  async setupRoots() {
    const all = await this.windows();
    const bar = all.find((w) => w.cls === MENU_BAR && w.child && absolutePosition(all, w).y < 60);
    if (!bar) throw new Error('the menu bar (TNewMenu) is not in the window table');
    const at = absolutePosition(all, bar);
    this.roots.clear();
    for (const [name, fx] of LABELS) this.roots.set(name, { kind: 'label', x: at.x + fx * bar.w, y: at.y + LABEL_FY * bar.h });
    for (const t of RIGHT_TARGETS) {
      const w = all.find((x) => x.cls === t.cls && (t.text === undefined || x.text.startsWith(t.text)));
      if (!w) continue;
      const p = absolutePosition(all, w);
      this.roots.set(`right-click ${t.name}`, { kind: 'right', x: p.x + w.w / 2, y: p.y + w.h / 2 });
    }
    const mixer = all.find((w) => w.cls === MIXER_CLASS && w.vis);
    const strips = mixer && under(all, mixer.lid).find((w) => w.cls === 'TVectorPanel' && w.vis && w.w >= STRIPS_W && w.h >= STRIPS_H);
    if (strips) {
      const p = absolutePosition(all, strips);
      this.roots.set(INSERT_ROOT, { kind: 'right', x: p.x + strips.w / (2 * STRIPS_N), y: p.y + strips.h * STRIP_NAME_FY });
    }
  }
  /** the baseline: the windows that are open with no action (the main form and the panels) */
  async setBaseline() { this.base = new Set((await this.windows()).map((w) => w.lid)); this.clean = true; }
  /** the roots' drop-downs are counted (a root not counted yet is counted now; newOnly: only those roots' items are returned); the items of the roots are returned in order */
  async countRoots(newOnly) {
    const w = this.walk;
    const items = [];
    for (const root of this.roots.keys()) {
      if (w.only && !w.only.split(',').some((o) => o.split('/')[0] === root)) continue;
      const fresh = !w.nodes[root];
      if (fresh) {
        await this.closeAll();
        await this.openRoot(root);
        const { pops } = this.newOf(await this.windows());
        w.nodes[root] = pops.length === 0 ? { n: 0, pre: false, note: 'no drop-down after the press' } : await this.countPopup(pops[pops.length - 1]);
        await this.closeAll();
      }
      if (newOnly && !fresh) continue;
      const c = w.nodes[root];
      const n = c.n ?? 0;
      for (let j = 1; j <= n; j++) items.push({ kind: 'menu', path: [root, j], deferred: root === FILE_LABEL && j === n });
      log(`dpr ${w.dpr}: ${root}: ${c.n} items${c.pre ? ' (pre)' : ''}`);
    }
    return items;
  }
  /** the probes that are not menu items: the re-checks, the top bar's parts (4 actions each), the Browser's tab strip (8 slots), the Settings' tabs (10 slots); their places are read from the window table */
  async extraTasks() {
    const all = await this.windows();
    const tasks = RECHECK.map((p) => ({ kind: 'menu', path: [...p], recheck: true }));
    for (const p of topBarParts(all)) {
      for (const action of TOP_ACTIONS) tasks.push({ kind: 'at', lid: p.lid, action, group: 'top', id: `top ${p.lid} ${action}`, label: `top > ${p.cls}${p.text ? ` "${p.text}"` : ''} #${p.lid} > ${action}` });
    }
    const browser = all.find((x) => x.cls === BROWSER_CLASS && x.vis);
    const strip = browser && all.find((x) => x.parent === browser.lid && x.vis && x.w > 300 && x.h >= 30 && x.h <= 40);
    if (strip) {
      for (let s = 0; s < BROWSER_TABS; s++) tasks.push({ kind: 'at', lid: strip.lid, slot: s, n: BROWSER_TABS, action: 'left', group: 'browser', id: `browser ${strip.lid} tab ${s}`, label: `browser > strip #${strip.lid} > tab ${s + 1} of ${BROWSER_TABS}` });
    }
    for (let s = 0; s < SETTINGS_TABS; s++) tasks.push({ kind: 'settings', slot: s, n: SETTINGS_TABS, group: 'settings', id: `settings tab ${s}`, label: `settings > tab ${s + 1} of ${SETTINGS_TABS}` });
    return tasks;
  }
  /** the first enumeration of a walk: the roots' items, the keys, the probes of the new groups in front of them */
  async enumerate() {
    const w = this.walk;
    w.queue = [];
    const items = await this.countRoots(false);
    const extra = await this.extraTasks();
    for (const k of KEYS) items.push({ kind: 'key', key: k });
    const insert = items.filter((t) => t.path?.[0] === INSERT_ROOT);
    w.queue.push(...extra.filter((t) => t.recheck), ...insert, ...extra.filter((t) => !t.recheck), ...items.filter((t) => !t.deferred && t.path?.[0] !== INSERT_ROOT), ...items.filter((t) => t.deferred));
    w.enumerated = true;
    w.extended = true;
  }
  /** a walk enumerated before the new groups (#1417): the re-checks, then the new roots' items, then the new groups are put in front of the queue */
  async extend() {
    const w = this.walk;
    const items = await this.countRoots(true);
    const extra = await this.extraTasks();
    w.queue.unshift(...extra.filter((t) => t.recheck), ...items, ...extra.filter((t) => !t.recheck));
    w.extended = true;
  }
  /** one at-probe: the action at the window's place (its centre, or the slot's centre on the strip); a strip's picture is compared before and after (notes) */
  async atAction(task, probe) {
    const all = await this.windows();
    const win = all.find((w) => w.lid === task.lid);
    if (!win) { probe.notes.push(`lid ${task.lid} is not in the window table`); return; }
    const at = absolutePosition(all, win);
    const fx = task.n ? (task.slot + 0.5) / task.n : 0.5;
    const gx = at.x + fx * win.w, gy = at.y + win.h / 2;
    if (task.n) this.strip = { box: { x: at.x, y: at.y, w: win.w, h: win.h }, before: await this.rectHash({ x: at.x, y: at.y, w: win.w, h: win.h }) };
    if (task.action === 'left' || task.action === 'right') await this.clickGuest(gx, gy, task.action);
    else if (task.action === 'wheel') await this.wheelGuest(gx, gy);
    else await this.dragGuest(gx, gy);
  }
  /** the Settings window: F10 when it is not open; the tab at slot n of the tab bar (equal slots of the bar's width); the page's name and its sections (new pages only: their first six panels are queued in front) */
  async settingsTab(task, probe) {
    const w = this.walk;
    let all = await this.windows();
    for (let i = 0; i < 3 && !all.some((x) => x.cls === SETTINGS_CLASS); i++) {
      await this.ev(FOCUS);
      await this.press(KEY_F10);
      await sleep(SETTINGS_OPEN_MS);
      all = await this.windows();
    }
    const top = all.find((x) => x.cls === SETTINGS_CLASS);
    if (!top) { probe.notes.push('the Settings window (TMIDIForm) did not open with F10 (3 tries)'); return; }
    const bar = all.find((x) => x.cls === 'TQuickSheetSelector' && x.parent === top.lid && x.vis);
    if (!bar) { probe.notes.push('no tab bar (TQuickSheetSelector) in the Settings window'); return; }
    const at = absolutePosition(all, bar);
    await this.clickGuest(at.x + ((task.slot + 0.5) * bar.w) / task.n, at.y + bar.h / 2, 'left');
    await sleep(PAGE_WAIT_MS);
    all = await this.windows();
    const top2 = all.find((x) => x.cls === SETTINGS_CLASS);
    const sheet = top2 && all.find((x) => x.cls === 'TVectorSheet' && x.parent === top2.lid && x.vis);
    const page = sheet ? sheet.text : '-';
    probe.notes.push(`page: ${page}; title: ${top2 ? top2.text : '-'}`);
    w.pages = w.pages ?? [];
    if (!sheet || w.pages.includes(page)) return;
    w.pages.push(page);
    // the page's sections, one click each (a window that the click opens is closed with Escape); the engine's state after each click is noted
    const secs = all.filter((x) => x.parent === sheet.lid && x.vis && x.cls === 'TVectorPanel' && x.w >= 100 && x.h >= 16).slice(0, SECTION_MAX);
    for (const x of secs) {
      const before = new Set((await this.windows()).map((y) => y.lid));
      const p = absolutePosition(all, x);
      await this.clickGuest(p.x + x.w / 2, p.y + x.h / 2, 'left');
      await sleep(PAGE_WAIT_MS);
      const st = await this.state();
      if (st.state !== 'engine-idle' || st.error) { probe.notes.push(`section "${x.text}": engine ${st.state}`); break; }
      const opened = this.newOf(await this.windows()).newWins.filter((y) => !before.has(y.lid));
      if (opened.length) {
        probe.notes.push(`section "${x.text}" opened: ${opened.map((y) => y.cls).join(', ')}`);
        await this.press(KEY_ESC);
        await sleep(PAGE_WAIT_MS);
      }
    }
    probe.notes.push(`sections of ${page}: ${secs.length} clicked`);
  }
  /** the probe of one queue item */
  async runTask(task) {
    const w = this.walk;
    const probe = { id: taskId(task), path: taskLabel(task), dpr: w.dpr, group: taskGroup(task), result: null, submenu: null, notes: [], newWindows: [], state: null, stop: null, fingerprint: null, logTail: [], logLines: 0, elapsedMs: 0, expected: false, recheck: !!task.recheck };
    const t0 = Date.now();
    try {
      if (!this.clean) {
        const left = await this.closeAll();
        if (left.length) {
          probe.notes.push(`left open before the path: ${left.map((x) => x.cls).join(', ')}`);
          probe.leftover = left.some((x) => !x.child);
          probe.result = 'ok';
          probe.elapsedMs = Date.now() - t0;
          return probe;
        }
      }
      const item = { logFrom: (await this.state()).logLength };
      if (task.kind === 'key') {
        await this.press(task.key);
      } else if (task.kind === 'menu') {
        if (task.deferred) probe.notes.push('deferred: the last item of File (Exit in the default order) is probed last');
        await this.navigate(task.path, true);
      } else if (task.kind === 'at') {
        this.strip = null;
        await this.atAction(task, probe);
      } else if (task.kind === 'settings') {
        await this.settingsTab(task, probe);
      }
      await this.observe(item);
      if (this.strip) {
        const after = await this.rectHash(this.strip.box);
        probe.notes.push(`strip picture ${after === this.strip.before ? 'unchanged' : 'changed'}`);
        this.strip = null;
      }
      Object.assign(probe, { result: item.result, state: item.state, stop: item.stop, fingerprint: item.fingerprint, newWindows: item.newWindows, logTail: item.logTail, logLines: item.logLines });
      probe.expected = expectedExit(probe, w.nodes);
      if (task.kind === 'menu' && probe.result === 'ok' && item.pops.length && !item.dialog) {
        const key = nodeKey(task.path);
        if (!w.nodes[key]) w.nodes[key] = await this.countPopup(item.pops[item.pops.length - 1]);
        const node = w.nodes[key];
        probe.submenu = node.n ?? null;
        if (task.path.length < MAX_DEPTH && node.n && !w.expanded.includes(key)) {
          w.expanded.push(key);
          const kids = [];
          for (let j = 1; j <= node.n; j++) kids.push({ kind: 'menu', path: [...task.path, j] });
          w.queue.unshift(...kids);
        }
      }
    } catch (e) {
      const msg = String(e?.message ?? e).slice(0, 300);
      probe.result = TIMEOUT_ANY.test(msg) ? 'hung' : 'error';
      probe.notes.push(`tool: ${msg}`);
      probe.fingerprint = fingerprint(probe.result, null);
    }
    const left = this.clean ? [] : await this.closeAll().catch(() => []);
    if (left.length) { probe.notes.push(`left open: ${left.map((x) => x.cls).join(', ')}`); probe.leftover = left.some((x) => !x.child); }
    for (const x of left) if (x.child) this.base.add(x.lid);
    probe.elapsedMs = Date.now() - t0;
    return probe;
  }
}

async function walkInSession({ send, evaluate, ctx, walk, deadline, saveProbe }) {
  const s = new Session({ send, evaluate, ctx, walk, deadline });
  await send('Emulation.setDeviceMetricsOverride', { width: VIEW_START.width, height: VIEW_START.height, deviceScaleFactor: walk.dpr, mobile: false });
  await sleep(1500);
  for (let i = 0; i < 4; i++) {
    const r = await s.ev(STAGE);
    if (Math.abs(r.sw - ROOM_W) <= 0.5 && Math.abs(r.sh - ROOM_H) <= 0.5) break;
    await send('Emulation.setDeviceMetricsOverride', { width: Math.round(r.iw + (ROOM_W - r.sw)), height: Math.round(r.ih + (ROOM_H - r.sh)), deviceScaleFactor: walk.dpr, mobile: false });
    await sleep(1500);
  }
  const room = await s.ev(RECT);
  if (room.gw <= 0) throw new Error('no guest screen');
  // the Welcome window and the Problems box of the default project are closed with Escape (as the other tools do)
  await s.ev(FOCUS);
  for (let i = 0; i < 30; i++) {
    const st = await s.state();
    if (st.state !== 'engine-idle') return { outcome: 'stopped', note: `state ${st.state} before the project opened` };
    if (st.title && st.title !== WELCOME_TITLE) break;
    await s.press(KEY_ESC);
    await sleep(1500);
  }
  await sleep(1500);
  for (let i = 0; i < 3; i++) {
    if (!(await s.windows()).some((w) => w.cls === PROBLEMS_CLASS)) break;
    await s.press(KEY_ESC);
    await sleep(1500);
  }
  await sleep(1000);
  await s.setupRoots();
  await s.setBaseline();
  if (!walk.enumerated) await s.enumerate();
  else if (!walk.extended) await s.extend();
  while (walk.queue.length) {
    if (Date.now() >= deadline) return { outcome: 'budget' };
    const st = await s.state();
    if (st.state !== 'engine-idle') {
      const p = { id: 'before the next path', path: 'before the next path', dpr: walk.dpr, group: '-', result: st.state === 'exited' ? 'stopped' : 'hung', state: st.state, stop: null, fingerprint: st.state === 'exited' ? '(fle 無し)' : '(停滞)', notes: [`state ${st.state} between two probes`], newWindows: [], logTail: [], logLines: 0, elapsedMs: 0, expected: false };
      saveProbe(p);
      return { outcome: 'stopped' };
    }
    const task = walk.queue.shift();
    const id = taskId(task);
    if (!onlyKeep(walk.only, id)) continue;
    if (walk.doneIds.includes(id)) continue;
    walk.doneIds.push(id);
    const probe = await s.runTask(task);
    saveProbe(probe);
    if (probe.result !== 'ok') return { outcome: probe.expected ? 'expected-exit' : 'stopped' };
    if (probe.leftover) return { outcome: 'reset' };
  }
  return { outcome: 'complete' };
}

function summarize(state, opts) {
  const probes = state.probes;
  const nodesOf = (dpr) => state.walks.find((w) => w.dpr === dpr)?.nodes ?? {};
  for (const p of probes) p.expected = expectedExit(p, nodesOf(p.dpr));
  const byDpr = {}, byGroup = {}, byResult = {};
  for (const p of probes) {
    const d = String(p.dpr);
    byDpr[d] = (byDpr[d] ?? 0) + 1;
    byGroup[p.group] = (byGroup[p.group] ?? 0) + 1;
    byResult[p.result] = (byResult[p.result] ?? 0) + 1;
  }
  const fps = new Map();
  for (const p of probes) {
    if (p.result === 'ok' || p.expected) continue;
    const f = p.fingerprint ?? '(fle 無し)';
    if (!fps.has(f)) fps.set(f, { fingerprint: f, count: 0, results: new Set(), dprs: new Set(), paths: [] });
    const e = fps.get(f);
    e.count++;
    e.results.add(p.result);
    e.dprs.add(p.dpr);
    if (e.paths.length < 3 && !e.paths.includes(p.path)) e.paths.push(p.path);
  }
  const fpRows = [...fps.values()].sort((a, b) => b.count - a.count).map((e) => ({ fingerprint: e.fingerprint, count: e.count, results: [...e.results], dprs: [...e.dprs], representativePaths: e.paths }));
  const walk1 = state.walks[0];
  const menuItemsTotal = walk1 ? Object.values(walk1.nodes).reduce((a, n) => a + (n.n ?? 0), 0) : 0;
  const menuItemsByDpr = state.walks.map((w) => ({ dpr: w.dpr, total: Object.values(w.nodes).reduce((a, n) => a + (n.n ?? 0), 0), counted: Object.keys(w.nodes).length }));
  const remaining = [];
  for (const w of state.walks) for (const t of w.queue) remaining.push(`dpr ${w.dpr}: ${taskLabel(t)}`);
  const walksWanted = state.walks.length && state.walks[0].only ? 1 : 2;
  if (state.walks.length < 2 && walksWanted === 2) remaining.push(`dpr (random): not started (the walk of DPR 1 is not complete)`);
  const notStarted = [
    { group: 'd', text: 'the Language (7) and Theme (13) switches: not done (the window table has no window per language or theme; the Settings tabs and their sections are walked)' },
    { group: 'c', text: 'the Mixer inserts other than the master insert\'s first slot (the rack is found in the window table; its first slot\'s menu is walked)' },
    { group: 'f', text: 'the built-in plugins one by one (#1371 tool not in the repository): the Mixer insert\'s submenus are walked (group plugin)' },
  ];
  const complete = state.walks.length === walksWanted && state.walks.every((w) => w.complete);
  const expectedList = probes.filter((p) => p.expected).map((p) => ({ id: p.id, path: p.path, dpr: p.dpr, state: p.state, reason: EXIT_REASON }));
  const fourIds = ['File/9/11', 'Add/14', 'View/11/1', 'File/10'];
  const fourFingerprints = fourIds.map((id) => ({ id, probes: probes.filter((p) => p.id === id).map((p) => ({ dpr: p.dpr, recheck: !!p.recheck, result: p.result, state: p.state, expected: !!p.expected, fingerprint: p.fingerprint, notes: p.notes })) }));
  return {
    startup: state.startup ? { ...state.startup, ok: !!state.startup.ok } : null,
    dprs: state.walks.map((w) => w.dpr),
    budgetMs: opts.budgetMs,
    elapsedMs: Date.now() - state.startedAtMs,
    complete,
    menuItemsTotal,
    menuItemsByDpr,
    cdpRetries: state.cdpRetries,
    restartFailures: state.restartFailures,
    startFailures: state.startFailures,
    sessions: state.sessions,
    stops: state.stops,
    expectedExits: state.expectedExits ?? 0,
    resets: state.resets,
    skipped: [],
    skippedNote: 'the items are not named (their texts are not read: no OCR, and the guest memory is not read for the item texts), so an external item (web help, update, registration, account) is not skipped by name; each one is probed and its result is recorded',
    remaining: [...remaining, ...notStarted.map((x) => `(${x.group}) not run: ${x.text}`)],
    probeCount: probes.length,
    stopProbes: probes.filter((p) => p.result !== 'ok' && !p.expected).length,
    expectedProbes: expectedList.length,
    expectedExitsList: expectedList,
    probesByDpr: byDpr,
    probesByGroup: byGroup,
    probesByResult: byResult,
    fingerprints: fpRows,
    fourFingerprints,
    walks: state.walks.map((w) => ({ dpr: w.dpr, complete: w.complete, queueLeft: w.queue.length, pages: w.pages ?? [], counted: Object.fromEntries(Object.entries(w.nodes).map(([k, v]) => [k, { n: v.n, pre: v.pre, note: v.note ?? null }])) })),
    probes: probes.map((p) => ({ id: p.id, path: p.path, dpr: p.dpr, group: p.group, result: p.result, state: p.state, submenu: p.submenu, stop: p.stop, fingerprint: p.fingerprint, newWindows: p.newWindows, notes: p.notes, logLines: p.logLines, logTail: p.logTail, elapsedMs: p.elapsedMs, leftover: !!p.leftover, expected: !!p.expected, recheck: !!p.recheck })),
    nproc: os.availableParallelism(),
    loadavg: os.loadavg(),
  };
}

function markdown(sum, date) {
  const esc = (s) => String(s).replace(/\|/g, '\\|');
  const code = (s) => `\`${esc(s)}\``;
  const rows = sum.fingerprints.map((f) => `| ${code(f.fingerprint)} | ${f.count} | ${f.results.join(', ')} | ${f.dprs.join(', ')} | ${f.representativePaths.map(code).join('<br>')} |`).join('\n');
  const four = sum.fourFingerprints.map((f) => {
    const cells = f.probes.map((p) => `DPR ${p.dpr}${p.recheck ? '(再走査)' : ''}: ${p.result}${p.expected ? '(期待される終了)' : ''}${p.state ? ` / ${p.state}` : ''}${p.fingerprint ? ` / ${code(p.fingerprint)}` : ''}`);
    return `| ${code(f.id)} | ${cells.join('<br>') || '未実施'} |`;
  }).join('\n');
  const expected = sum.expectedExitsList.map((e) => `- ${code(e.path)} (DPR ${e.dpr}): ${e.reason}`).join('\n') || '- なし';
  const pm = (o) => Object.entries(o).map(([k, v]) => `${k}: ${v}`).join(', ');
  const menus = sum.menuItemsByDpr.map((m) => `${m.dpr} → ${m.total}`).join(', ');
  const remainingPaths = sum.remaining.filter((r) => !r.startsWith('('));
  const remainingHead = remainingPaths.slice(0, 10).map(code).join(', ') || 'なし';
  const remainingNotes = sum.remaining.filter((r) => r.startsWith('(')).map((r) => `- ${r}`).join('\n');
  return `# crash-hunt ${date} (Issue #1405, #1417)

実エンジン: \`tools/crash-hunt/run.sh\` で、fe-build の bundle(raw/fl-Oz)の engine を、headless Chrome で起動し、FL の画面の操作(メニュー・キー・右クリック・上部バー・Settings・Browser・Mixer のインサート)を 1 件ずつ踏んだ。runner の値。この機械とは別枠。

## 概要

- probe: ${sum.probeCount} 件(DPR 別: ${pm(sum.probesByDpr)})。グループ別: ${pm(sum.probesByGroup)}。グループは a メニュー・b キー・c 右クリック・top 上部バーと Mixer インサートの右クリック・settings Settings の頁・browser Browser の頁の帯・plugin Mixer インサートの下位のメニュー。
- 結果: ${pm(sum.probesByResult)}。停止(期待される終了を除く): ${sum.stopProbes} 件。期待される終了: ${sum.expectedProbes} 件(File の最後の項目 Exit)。
- 予算: ${sum.budgetMs / 60000} 分。経過: ${(sum.elapsedMs / 60000).toFixed(1)} 分。完了: ${sum.complete ? 'はい' : 'いいえ(remaining を見る)'}
- DPR: ${sum.dprs.join(', ')}。停止の後の起動: ${sum.sessions} 回(停止 ${sum.stops}、期待される終了 ${sum.expectedExits}、窓の残り ${sum.resets})。起動し直しの失敗: ${sum.restartFailures}。CDP の Runtime.evaluate の再試行: ${sum.cdpRetries}。
- menuItemsTotal(DPR 1 の走査): ${sum.menuItemsTotal}。DPR 別: ${menus}

## 指紋ごとの表(期待される終了を除く)

| 指紋(\`stop.lastFle\` の先頭 80 字) | 件数 | 結果 | DPR | 代表の経路(3 件) |
|---|---|---|---|---|
${rows || '| (停止なし) | 0 | - | - | - |'}

## 期待される終了

${expected}

## 4 件の指紋の再走査(#1405 の停止)

| 経路 | 結果 |
|---|---|
${four}

## 残り

- 経路の残り: ${remainingPaths.length} 件(先頭 10 件: ${remainingHead})
${remainingNotes}
`;
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const t0 = Date.now();
  const deadline = t0 + opts.budgetMs;
  fs.mkdirSync(opts.out, { recursive: true });
  const statePath = path.join(opts.out, 'crash-hunt-state.json');
  let state;
  if (opts.resume && fs.existsSync(statePath)) {
    state = JSON.parse(fs.readFileSync(statePath, 'utf8'));
  } else {
    state = { seed: opts.seed, startedAtMs: t0, startup: null, walks: [], probes: [], cdpRetries: 0, restartFailures: 0, startFailures: 0, sessions: 0, stops: 0, expectedExits: 0, resets: 0 };
  }
  const save = () => fs.writeFileSync(statePath, JSON.stringify(state));
  if (!state.startup) {
    state.startup = opts.skipStartup ? null : startupCheck(opts.bundle);
    log('startup', state.startup);
    save();
  }
  const ctx = state;
  const saveProbe = (p) => { state.probes.push(p); save(); log(`probe ${p.dpr} ${p.result}${p.expected ? ' (expected)' : ''} ${p.path}`); };
  let consecutiveFailures = 0;
  for (;;) {
    if (Date.now() >= deadline) break;
    if (state.walks.length && state.walks.at(-1).complete && (state.walks.length >= 2 || opts.only)) break;
    if (!state.walks.length || state.walks.at(-1).complete) {
      const dpr = state.walks.length === 0 ? 1 : randomDpr(opts.seed);
      // the second walk reuses the counts of the first (the menus are the same at every DPR): its submenus are opened from these counts
      const nodes = state.walks.length ? JSON.parse(JSON.stringify(state.walks[0].nodes)) : {};
      state.walks.push({ dpr, enumerated: false, extended: false, complete: false, nodes, queue: [], doneIds: [], expanded: [], pages: [], only: opts.only });
      save();
    }
    const walk = state.walks.at(-1);
    let res = null, startError = null;
    state.sessions++;
    const { out } = await bootBundle(opts.bundle, {
      afterIdle: async ({ send, evaluate }) => {
        try {
          res = await walkInSession({ send, evaluate, ctx, walk, deadline, saveProbe });
        } catch (e) {
          res = { outcome: 'setup', note: String(e?.message ?? e).slice(0, 300) };
        }
      },
    }).catch((e) => ({ out: { error: String(e?.message ?? e) } }));
    if (res === null) {
      startError = out.error ?? `not idle (state ${out.state})`;
      if (state.stops + state.resets > 0) state.restartFailures++;
      else state.startFailures++;
      save();
      log(`start failed: ${startError}`);
      consecutiveFailures++;
      if (consecutiveFailures >= 3) break;
      continue;
    }
    if (res.outcome === 'complete') { walk.complete = true; save(); continue; }
    if (res.outcome === 'budget') break;
    if (res.outcome === 'setup') {
      log(`setup failed: ${res.note}`);
      state.startFailures++;
      consecutiveFailures++;
      save();
      if (consecutiveFailures >= 3) break;
      continue;
    }
    consecutiveFailures = 0;
    if (res.outcome === 'stopped') state.stops++;
    if (res.outcome === 'expected-exit') state.expectedExits = (state.expectedExits ?? 0) + 1;
    if (res.outcome === 'reset') state.resets++;
    save();
    if (out.error) log(`session error after the walk: ${out.error}`);
  }
  const elapsedMs = Date.now() - t0;
  state.endedAtMs = Date.now();
  save();
  const date = new Date().toISOString().slice(0, 10);
  const sum = summarize(state, { budgetMs: opts.budgetMs });
  sum.elapsedMs = elapsedMs;
  fs.writeFileSync(path.join(opts.out, `crash-hunt-${date}.json`), JSON.stringify(sum, null, 1) + '\n');
  fs.writeFileSync(path.join(opts.out, `crash-hunt-${date}.md`), markdown(sum, date));
  log(`done: probes ${state.probes.length}, complete ${sum.complete}, elapsed ${Math.round(elapsedMs / 1000)} s`);
}

// the walk runs when this file is the program (the functions above are exported for the checks)
const self = process.argv[1] ? fs.realpathSync(process.argv[1]) : null;
if (self === fs.realpathSync(fileURLToPath(import.meta.url))) {
  await main().catch((e) => {
    process.stderr.write(`${String(e?.stack ?? e)}\n`);
    process.exitCode = 1;
  });
}
