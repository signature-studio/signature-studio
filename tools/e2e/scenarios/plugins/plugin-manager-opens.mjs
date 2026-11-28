// plugins/plugin-manager-opens: an Options item starts PluginManager.exe and its window appears; the engine stays engine-idle for 10 s after. The Plugin Manager is another program, started as a thread of the engine (#1499, closed: the window opens, so no mark).
import { defineScenario } from '../../lib/scenario.mjs';
import { openMenu, windowTable } from '../../lib/common.mjs';
import { sleep } from '../../lib/flow.mjs';

const MAX_ITEMS = 30;
const FILE = /PluginManager\.exe/;
const WINDOW = /plugin ?manager/i;

export default defineScenario({
  id: 'plugins/plugin-manager-opens',
  covers: ['plugins/tool/plugin-manager', 'plugins/window/plugin-manager', 'plugins/stop/plugin-manager'],
  expectFail: null,
  async run({ guest }) {
    await openMenu(guest, 'Options');
    await guest.escape();
    let hit = null;
    for (let k = 1; k <= MAX_ITEMS && !hit; k++) {
      const from = await guest.ev('window.__app.engineLog().length');
      await openMenu(guest, 'Options');
      for (let j = 0; j < k; j++) { await guest.key('ArrowDown', 'ArrowDown', 40); await sleep(120); }
      await guest.key('Enter', 'Enter', 13);
      await sleep(1500);
      const lines = await guest.ev(`window.__app.engineLog().slice(${from})`);
      if (lines.some((l) => FILE.test(l))) hit = k;
      await guest.escape();
      await sleep(300);
    }
    if (!hit) return { ok: false, engineState: (await guest.state())?.state ?? null, notes: [`none of the first ${MAX_ITEMS} Options items starts PluginManager.exe`] };
    let win = null;
    for (let t = 0; t < 20 && !win; t++) {
      await sleep(500);
      win = (await windowTable(guest)).top.find((w) => WINDOW.test(w.cls) || WINDOW.test(w.title)) ?? null;
    }
    const t0 = Date.now();
    let idle = true;
    while (Date.now() - t0 < 10000) {
      const s = await guest.state();
      if (!s || s.state !== 'engine-idle') { idle = false; break; }
      await sleep(500);
    }
    return { ok: !!win && idle, engineState: (await guest.state())?.state ?? null, notes: [`Options item ${hit} starts PluginManager.exe; the window: ${win ? `${win.cls} "${win.title}"` : 'none'}; engine idle for 10 s: ${idle}`] };
  },
});
