// (d) plugin-manager-opens (Issue #1484): the Options menu's item that opens the Plugin Manager opens its window, and the engine stays engine-idle for 10 s after.
// The Plugin Manager is PluginManager.exe (System\Tools\Plugin Manager\ in the installer), a program of its own: the engine has no association for it
// (S-ia2-shellexecute-noassoc: ShellExecuteW answers 31 and starts nothing). The item is found by the engine's own line that names the file: the Options
// menu's items are chosen one by one (Down presses, then Enter), and each item's new engine lines are read. The window must then appear in the window table.
// expectFail '#1499': the window does not open until the file is there (#1499). Until then only the page and the engine's counters are judged.
import { closeWelcome, dismissProblems, noThanks, sleep, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

const MENU_BAR = 'TNewMenu';
// the bar's places of the Options label (tools/crash-hunt/hunt.mjs LABELS and LABEL_FY)
const OPTIONS_FX = 0.666, LABEL_FY = 0.45;
const MAX_ITEMS = 30;
const IDLE_MS = 10000;
const PLUGIN_MANAGER_FILE = /PluginManager\.exe/;
const PLUGIN_MANAGER_WINDOW = /plugin ?manager/i;
const DETAIL = '(async () => (await window.__app.deskDumpBase64(true)).windows)()';

/** the detailed window table (children included, as tools/crash-hunt/hunt.mjs reads it); a child's x, y are relative to its parent */
export function parseDetail(text) {
  return String(text).split('\n').map((l) => {
    const m = /^(child )?lid (\d+)(?: parent lid (\d+))? at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/.exec(l);
    if (!m) return null;
    const cls = /class "([^"]*)"/.exec(m[8]);
    const cap = /(?:text|title) "([^"]*)"\s*$/.exec(m[8]);
    return { child: !!m[1], lid: Number(m[2]), parent: m[3] ? Number(m[3]) : null, x: Number(m[4]), y: Number(m[5]), w: Number(m[6]), h: Number(m[7]), cls: cls ? cls[1] : '', text: cap ? cap[1] : '', vis: /vis 1/.test(m[8]) };
  }).filter(Boolean);
}

function absolutePosition(list, win) {
  let x = win.x, y = win.y, cur = win;
  while (cur.child && cur.parent !== null) {
    const up = list.find((w) => w.lid === cur.parent);
    if (!up) break;
    x += up.x; y += up.y;
    cur = up;
  }
  return { x, y };
}

/** the windows as the table has them now: a "ask again" answer (the picture is being shown) is asked again for up to 60 s */
async function detail(guest) {
  const t0 = Date.now();
  for (;;) {
    try {
      return parseDetail(await guest.ev(DETAIL));
    } catch (e) {
      if (!/ask again/.test(String(e?.message ?? e)) || Date.now() - t0 >= 60000) throw e;
      await sleep(100);
    }
  }
}

async function optionsLabel(guest) {
  const all = await detail(guest);
  const bar = all.find((w) => w.cls === MENU_BAR && w.child && absolutePosition(all, w).y < 60);
  if (!bar) return null;
  const at = absolutePosition(all, bar);
  return { x: at.x + OPTIONS_FX * bar.w, y: at.y + LABEL_FY * bar.h };
}

export default {
  name: 'plugin-manager-opens',
  expectFail: '#1499',
  async run({ guest, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    if (await guest.findWindow((w) => w.cls === WELCOME)) {
      await noThanks(guest);
      await closeWelcome(guest);
    }
    const end = await waitProjectOpen(guest, first.title, 120000);
    if (!end.opened) return { ok: false, engineState: end.state, notes: [...notes, 'the default project did not open'] };
    await dismissProblems(guest);
    const label = await optionsLabel(guest);
    if (!label) return { ok: false, engineState: end.state, notes: [...notes, 'the menu bar (TNewMenu) is not in the window table'] };
    let hit = null;
    for (let k = 1; k <= MAX_ITEMS && !hit; k++) {
      const from = await guest.ev('window.__app.engineLog().length');
      const before = new Set((await detail(guest)).map((w) => w.lid));
      await guest.click(label.x, label.y);
      await sleep(600);
      for (let j = 0; j < k; j++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(120); }
      await guest.key('Enter', 'Enter', 13);
      await sleep(1500);
      const lines = await guest.ev(`window.__app.engineLog().slice(${from})`);
      const fresh = (await detail(guest)).filter((w) => !before.has(w.lid) && !w.child);
      const named = lines.filter((l) => PLUGIN_MANAGER_FILE.test(l));
      if (named.length > 0) hit = { k, named, fresh };
      else if (lines.length > 0 || fresh.length > 0) notes.push(`Options item ${k}: ${lines.length} new engine line(s), ${fresh.length} new window(s) ${fresh.map((w) => `${w.cls} "${w.text}"`).join(', ')}`);
      await guest.escape();
      await sleep(300);
    }
    if (!hit) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [...notes, `none of the first ${MAX_ITEMS} Options items named PluginManager.exe`] };
    log(`Options item ${hit.k} starts the Plugin Manager: ${hit.named[0]}`);
    notes.push(`Options item ${hit.k} (Down x ${hit.k}, Enter): ${hit.named[0]}`);
    let windowSeen = hit.fresh.find((w) => PLUGIN_MANAGER_WINDOW.test(w.cls) || PLUGIN_MANAGER_WINDOW.test(w.text)) ?? null;
    for (let t = 0; t < 20 && !windowSeen; t++) {
      await sleep(500);
      windowSeen = (await detail(guest)).find((w) => !w.child && (PLUGIN_MANAGER_WINDOW.test(w.cls) || PLUGIN_MANAGER_WINDOW.test(w.text))) ?? null;
    }
    notes.push(windowSeen ? `the Plugin Manager window: ${windowSeen.cls} "${windowSeen.text}"` : 'no Plugin Manager window in the window table 10 s after the item');
    const t0 = Date.now();
    let idle = true, last = null;
    while (Date.now() - t0 < IDLE_MS) {
      last = await guest.state().catch(() => null);
      if (!last || last.state !== 'engine-idle') { idle = false; break; }
      await sleep(500);
    }
    notes.push(`engine ${last?.state ?? 'unreadable'} for ${Math.min(IDLE_MS, Date.now() - t0)} ms after the item`);
    return { ok: !!windowSeen && idle, engineState: last?.state ?? null, notes };
  },
};
