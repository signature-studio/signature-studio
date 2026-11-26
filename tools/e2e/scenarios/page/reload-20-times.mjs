// page/reload-20-times: the page is reloaded twenty times: each time the start runs again to the idle engine and the project opens, and the window table can be read (at DPR 1: a start takes about half a minute).
import { defineScenario } from '../../lib/scenario.mjs';
import { persist, windowTable } from '../../lib/common.mjs';

export default defineScenario({
  id: 'page/reload-20-times',
  covers: ['page/reload/20-times'],
  displays: ['dpr:1'],
  async run({ guest, page }) {
    const notes = [];
    let good = 0;
    const t0 = Date.now();
    for (let i = 1; i <= 20; i++) {
      const r = await persist(page, guest);
      const table = await windowTable(guest).then((t) => t.top.length).catch(() => 0);
      const ok = r.opened === true && r.state === 'engine-idle' && table > 0;
      if (ok) good++;
      else notes.push(`reload ${i}: opened ${r.opened}, state ${r.state}, windows ${table}`);
    }
    notes.unshift(`${good} of 20 reloads started again (engine idle, project open, window table read); ${Math.round((Date.now() - t0) / 1000)} s in all`);
    return { ok: good === 20, engineState: (await guest.state())?.state ?? null, notes, results: { good, of: 20 } };
  },
});
