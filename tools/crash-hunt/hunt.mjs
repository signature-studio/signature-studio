// crash-hunt (Issue #1405): walks the FL screen's operations in the lifted engine's page and records what each one did, one probe per path and DPR.
// Usage: node tools/crash-hunt/hunt.mjs --out <directory> [--budget 50m] [--bundle <raw/<name>>] [--seed 20251125] [--resume]   (run through tools/crash-hunt/run.sh; needs app/node_modules and app/dist, as boot-bundle.mjs does).
//   --budget: a number with s, m or h (50m). --bundle: the bundle directory (default lift/out-app/bundle/raw/fl-Oz). --seed: the seed of the random DPR (1.01 to 2.24).
//   --resume: continues from <out>/crash-hunt-state.json. --skip-startup: a test aid (the startup check is not run; startup is then null).
// Walk: two walks, one per DPR (1, then the random one). Each walk: (a) the menus: the 8 top labels of the bar (their places are the bar's fractions, the bar is found in the window table),
//   every item of each drop-down, and the submenus opened by an item (up to 3 levels); the right-click menus of the four windows (Channel rack, Mixer, Playlist, Browser); (b) the keys F5 to F12, Alt+F8, Ctrl+O/E/P/F/I, Space.
//   The File menu's last item (Exit) is the last probe of a walk. Items are chosen by the keys: Down presses (the count of the Down presses to reach an item) and Enter.
//   Item texts are NOT read (no OCR; the paths say "#n", the position in its menu). The number of items of a menu comes from the Down presses: the count is the distance between the first two equal pictures of the drop-down's rectangle (the highlight goes round). The second walk reuses the counts of the first (the same menus at every DPR).
// A session is one start of the bundle (boot-bundle's bootBundle): the walk runs in its afterIdle; a stop (exited, error, or no idle within 20 s = hung) ends the session, and the next session starts the bundle again and goes on from the next path (restartFailures counts starts that failed after a stop).
// The state is written to <out>/crash-hunt-state.json after every probe. The findings are <out>/crash-hunt-<UTC date>.json and .md (the fingerprints: the engine's last "fle: " line, first 80 characters, numbers and addresses replaced with #; "(fle 無し)" without one; "(停滞)" for a hang).
// A probe's observation: after the action, 2 s, then the engine state up to 20 s after the action (exited: stopped; error or the page's error: error; running: hung), the windows that are new (they are closed with Escape, then Enter), and the engine log's new lines.
// Not done here (remaining, with the reason in the findings): the F10 Settings tabs (d), the plugin instantiation (f), the Browser tabs and the Piano roll tools beyond the TOOLS menu (e), the Tempo / Song Position / Pattern Selector parts (c).
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
// the keys: F5 to F12, Alt+F8, Ctrl+O, Ctrl+E, Ctrl+P, Ctrl+F, Ctrl+I, Space (modifiers: 1 Alt, 2 Ctrl)
const KEYS = [
  ...[116, 117, 118, 119, 120, 121, 122, 123].map((vk) => ({ name: `F${vk - 111}`, code: `F${vk - 111}`, key: `F${vk - 111}`, vk, mods: 0 })),
  { name: 'Alt+F8', code: 'F8', key: 'F8', vk: 119, mods: 1 },
  ...[['O', 79], ['E', 69], ['P', 80], ['F', 70], ['I', 73]].map(([c, vk]) => ({ name: `Ctrl+${c}`, code: `Key${c}`, key: c.toLowerCase(), vk, mods: 2 })),
  { name: 'Space', code: 'Space', key: ' ', vk: 32, mods: 0 },
];
const KEY_ESC = { code: 'Escape', key: 'Escape', vk: 27, mods: 0 };
const KEY_ENTER = { code: 'Enter', key: 'Enter', vk: 13, mods: 0 };
const KEY_DOWN = { code: 'ArrowDown', key: 'ArrowDown', vk: 40, mods: 0 };
const DOWN_WAIT_MS = 200, NAV_WAIT_MS = 120, OPEN_WAIT_MS = 600, OPEN_SETTLE_MS = 500, OBSERVE_MS = 2000, SETTLE_MS = 20000, POLL_MS = 400;
const MAX_ITEMS = 90, MAX_DEPTH = 3;
const FILE_LABEL = 'File';
const STARTUP_LIMIT = { elapsedMs: 12500, preBootBytes: 314368711 };
const TIMEOUT_EVAL = /^timeout \d+ ms: Runtime\.evaluate/;
const TIMEOUT_ANY = /^timeout \d+ ms:/;

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

/** the fingerprint of a stop: the last "fle: " line without digits and addresses, 80 characters at most */
export function fingerprint(result, stop) {
  if (result === 'hung') return '(停滞)';
  const fle = stop && stop.lastFle;
  if (!fle) return '(fle 無し)';
  return fle.replace(/0x[0-9a-fA-F]+/g, '#').replace(/\d+/g, '#').replace(/\s+/g, ' ').slice(0, 80);
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
const groupOf = (root) => (root.startsWith('right-click') ? 'c' : 'a');

function parseArgs(argv) {
  const get = (f) => { const i = argv.indexOf(f); return i >= 0 ? argv[i + 1] : undefined; };
  const budgetText = get('--budget') ?? '50m';
  const m = /^(\d+(?:\.\d+)?)(s|m|h)$/.exec(budgetText);
  if (!m) throw new Error(`--budget ${budgetText}: use a number with s, m or h`);
  const out = get('--out');
  if (!out) throw new Error('--out is required');
  return {
    budgetMs: Number(m[1]) * { s: 1000, m: 60000, h: 3600000 }[m[2]],
    out: path.resolve(out),
    bundle: path.resolve(get('--bundle') ?? path.join(repo, 'lift/out-app/bundle/raw/fl-Oz')),
    seed: Number(get('--seed') ?? 20251125),
    resume: argv.includes('--resume'),
    skipStartup: argv.includes('--skip-startup'),
  };
}

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
  async clickGuest(gx, gy, button = 'left') {
    const r = await this.ev(RECT);
    const c = { x: r.x + (gx + 0.5) * (r.w / r.gw), y: r.y + (gy + 0.5) * (r.h / r.gh) };
    const mask = button === 'right' ? 2 : 1;
    await this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y, button: 'none', buttons: 0, pointerType: 'mouse' });
    await sleep(60);
    await this.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button, buttons: mask, clickCount: 1, pointerType: 'mouse' });
    await sleep(50);
    await this.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button, buttons: 0, clickCount: 1, pointerType: 'mouse' });
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
  }
  /** the baseline: the windows that are open with no action (the main form and the panels) */
  async setBaseline() { this.base = new Set((await this.windows()).map((w) => w.lid)); this.clean = true; }
  /** the drop-down that the last action opened, counted: its items are queued (a submenu's items go first, depth up to MAX_DEPTH) */
  async enumerate() {
    const w = this.walk;
    w.queue = [];
    const deferred = [];
    for (const root of this.roots.keys()) {
      if (!w.nodes[root]) {
        await this.closeAll();
        await this.openRoot(root);
        const { pops } = this.newOf(await this.windows());
        w.nodes[root] = pops.length === 0 ? { n: 0, pre: false, note: 'no drop-down after the press' } : await this.countPopup(pops[pops.length - 1]);
        await this.closeAll();
      }
      const c = w.nodes[root];
      const n = c.n ?? 0;
      for (let j = 1; j <= n; j++) {
        if (root === FILE_LABEL && j === n) { deferred.push({ kind: 'menu', path: [root, j], deferred: true }); continue; }
        w.queue.push({ kind: 'menu', path: [root, j] });
      }
      log(`dpr ${w.dpr}: ${root}: ${c.n} items${c.pre ? ' (pre)' : ''}`);
    }
    for (const k of KEYS) w.queue.push({ kind: 'key', key: k });
    w.queue.push(...deferred);
    w.enumerated = true;
  }
  /** the probe of one queue item */
  async runTask(task) {
    const w = this.walk;
    const probe = { id: '', path: '', dpr: w.dpr, group: 'b', result: null, submenu: null, notes: [], newWindows: [], state: null, stop: null, fingerprint: null, logTail: [], logLines: 0, elapsedMs: 0 };
    const t0 = Date.now();
    try {
      if (!this.clean) {
        const left = await this.closeAll();
        if (left.length) {
          probe.notes.push(`left open before the path: ${left.map((x) => x.cls).join(', ')}`);
          probe.leftover = left.some((x) => !x.child);
          probe.result = 'ok';
          probe.id = task.kind === 'key' ? `key ${task.key.name}` : nodeKey(task.path);
          probe.path = task.kind === 'key' ? `key ${task.key.name}` : pathText(task.path);
          probe.group = task.kind === 'key' ? 'b' : groupOf(task.path[0]);
          probe.elapsedMs = Date.now() - t0;
          return probe;
        }
      }
      const item = { logFrom: (await this.state()).logLength };
      if (task.kind === 'key') {
        probe.id = `key ${task.key.name}`;
        probe.path = `key ${task.key.name}`;
        await this.press(task.key);
      } else {
        const p = task.path;
        probe.id = nodeKey(p);
        probe.path = pathText(p);
        probe.group = groupOf(p[0]);
        if (task.deferred) probe.notes.push('deferred: the last item of File (Exit in the default order) is probed last');
        await this.navigate(p, true);
      }
      await this.observe(item);
      Object.assign(probe, { result: item.result, state: item.state, stop: item.stop, fingerprint: item.fingerprint, newWindows: item.newWindows, logTail: item.logTail, logLines: item.logLines });
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
  while (walk.queue.length) {
    if (Date.now() >= deadline) return { outcome: 'budget' };
    const st = await s.state();
    if (st.state !== 'engine-idle') {
      const p = { id: 'before the next path', path: 'before the next path', dpr: walk.dpr, group: '-', result: st.state === 'exited' ? 'stopped' : 'hung', state: st.state, stop: null, fingerprint: st.state === 'exited' ? '(fle 無し)' : '(停滞)', notes: [`state ${st.state} between two probes`], newWindows: [], logTail: [], logLines: 0, elapsedMs: 0 };
      saveProbe(p);
      return { outcome: 'stopped' };
    }
    const task = walk.queue.shift();
    const id = task.kind === 'key' ? `key ${task.key.name}` : nodeKey(task.path);
    if (walk.doneIds.includes(id)) continue;
    walk.doneIds.push(id);
    const probe = await s.runTask(task);
    saveProbe(probe);
    if (probe.result !== 'ok') return { outcome: 'stopped' };
    if (probe.leftover) return { outcome: 'reset' };
  }
  return { outcome: 'complete' };
}

function summarize(state, opts) {
  const probes = state.probes;
  const byDpr = {}, byGroup = {}, byResult = {};
  for (const p of probes) {
    const d = String(p.dpr);
    byDpr[d] = (byDpr[d] ?? 0) + 1;
    byGroup[p.group] = (byGroup[p.group] ?? 0) + 1;
    byResult[p.result] = (byResult[p.result] ?? 0) + 1;
  }
  const fps = new Map();
  for (const p of probes) {
    if (p.result === 'ok') continue;
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
  for (const w of state.walks) for (const t of w.queue) remaining.push(`dpr ${w.dpr}: ${t.kind === 'key' ? `key ${t.key.name}` : pathText(t.path)}`);
  if (state.walks.length < 2) remaining.push(`dpr (random): not started (the walk of DPR 1 is not complete)`);
  const notStarted = [{ group: 'd', text: 'F10 Settings: the tabs and their choices, the Language and Theme switches (not implemented; F10 itself is a key probe)' }, { group: 'e', text: 'Browser tabs (Sounds, Plugin database, Packs ...)' }, { group: 'f', text: 'plugins: the Effects (112) and the Generators (70) of inputs/plugin-classes.tsv (not implemented: #1371 tool not in the repository)' }, { group: 'c', text: 'top bar parts (Tempo, Song Position, Pattern Selector) and the Mixer inserts' }];
  const complete = state.walks.length === 2 && state.walks.every((w) => w.complete);
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
    resets: state.resets,
    skipped: [],
    skippedNote: 'the items are not named (their texts are not read: no OCR, and the guest memory is not read for the item texts), so an external item (web help, update, registration, account) is not skipped by name; each one is probed and its result is recorded',
    remaining: [...remaining, ...notStarted.map((x) => `(${x.group}) not run: ${x.text}`)],
    probeCount: probes.length,
    probesByDpr: byDpr,
    probesByGroup: byGroup,
    probesByResult: byResult,
    fingerprints: fpRows,
    walks: state.walks.map((w) => ({ dpr: w.dpr, complete: w.complete, queueLeft: w.queue.length, counted: Object.fromEntries(Object.entries(w.nodes).map(([k, v]) => [k, { n: v.n, pre: v.pre, note: v.note ?? null }])) })),
    probes: probes.map((p) => ({ id: p.id, path: p.path, dpr: p.dpr, group: p.group, result: p.result, state: p.state, submenu: p.submenu, stop: p.stop, fingerprint: p.fingerprint, newWindows: p.newWindows, notes: p.notes, logLines: p.logLines, logTail: p.logTail, elapsedMs: p.elapsedMs, leftover: !!p.leftover })),
    nproc: os.availableParallelism(),
    loadavg: os.loadavg(),
  };
}

function markdown(sum, date) {
  const rows = sum.fingerprints.map((f) => `| \`${f.fingerprint.replace(/\|/g, '\\|')}\` | ${f.count} | ${f.results.join(', ')} | ${f.dprs.join(', ')} | ${f.representativePaths.map((p) => `\`${p.replace(/\|/g, '\\|')}\``).join('<br>')} |`).join('\n');
  const pm = (o) => Object.entries(o).map(([k, v]) => `${k}: ${v}`).join(', ');
  return `# crash-hunt ${date} (Issue #1405)

実エンジン: \`tools/crash-hunt/run.sh\` で、fe-build の bundle(raw/fl-Oz)の engine を、headless Chrome で起動し、FL の画面の操作(メニュー・キー・右クリック)を 1 件ずつ踏んだ。runner の値。この機械とは別枠。

## 概要

- probe: ${sum.probeCount} 件(DPR 別: ${pm(sum.probesByDpr)}。グループ別: ${pm(sum.probesByGroup)}。グループは a メニュー・b キー・c 右クリック)
- 結果: ${pm(sum.probesByResult)}
- 予算: ${sum.budgetMs / 60000} 分。経過: ${(sum.elapsedMs / 60000).toFixed(1)} 分。完了: ${sum.complete ? 'はい' : 'いいえ(remaining を見る)'}
- DPR: ${sum.dprs.join(', ')}。停止の後の起動: ${sum.sessions} 回(停止 ${sum.stops}、窓の残り ${sum.resets})。起動し直しの失敗: ${sum.restartFailures}。CDP の Runtime.evaluate の再試行: ${sum.cdpRetries}。
- menuItemsTotal(DPR 1 の走査): ${sum.menuItemsTotal}。DPR 別: ${sum.menuItemsByDpr.map((m) => `${m.dpr} → ${m.total}`).join(', ')}

## 指紋ごとの表

| 指紋(\`stop.lastFle\` の先頭 80 字) | 件数 | 結果 | DPR | 代表の経路(3 件) |
|---|---|---|---|---|
${rows || '| (停止なし) | 0 | - | - | - |'}

## 残り

- 経路の残り: ${sum.remaining.filter((r) => !r.startsWith('(')).length} 件(先頭 10 件: ${sum.remaining.filter((r) => !r.startsWith('(')).slice(0, 10).map((r) => `\`${r}\``).join(', ') || 'なし'})
${sum.remaining.filter((r) => r.startsWith('(')).map((r) => `- ${r}`).join('\n')}
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
    state = { seed: opts.seed, startedAtMs: t0, startup: null, walks: [], probes: [], cdpRetries: 0, restartFailures: 0, startFailures: 0, sessions: 0, stops: 0, resets: 0 };
  }
  const save = () => fs.writeFileSync(statePath, JSON.stringify(state));
  if (!state.startup) {
    state.startup = opts.skipStartup ? null : startupCheck(opts.bundle);
    log('startup', state.startup);
    save();
  }
  const ctx = state;
  const saveProbe = (p) => { state.probes.push(p); save(); log(`probe ${p.dpr} ${p.result} ${p.path}`); };
  let consecutiveFailures = 0;
  for (;;) {
    if (Date.now() >= deadline) break;
    if (state.walks.length && state.walks.at(-1).complete && state.walks.length >= 2) break;
    if (!state.walks.length || state.walks.at(-1).complete) {
      const dpr = state.walks.length === 0 ? 1 : randomDpr(opts.seed);
      // the second walk reuses the counts of the first (the menus are the same at every DPR): its submenus are opened from these counts
      const nodes = state.walks.length ? JSON.parse(JSON.stringify(state.walks[0].nodes)) : {};
      state.walks.push({ dpr, enumerated: false, complete: false, nodes, queue: [], doneIds: [], expanded: [] });
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
