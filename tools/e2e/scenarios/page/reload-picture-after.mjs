// page/reload-picture-after: after a reload the first picture of the engine has no black window: at most 10 % of its 64 x 64 tiles inside the windows are black (the windows are drawn, not left black).
import { defineScenario } from '../../lib/scenario.mjs';
import { persist, windowTable } from '../../lib/common.mjs';
import { blackInWindows } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/reload-picture-after',
  covers: ['page/reload/picture-after'],
  async run({ guest, page }) {
    const r = await persist(page, guest);
    const b = await blackInWindows(guest);
    const t = await windowTable(guest);
    const ok = r.opened === true && r.state === 'engine-idle' && b.black <= b.total * 0.1 && t.top.length > 0;
    return { ok, engineState: r.state, notes: [`after the reload: opened ${r.opened}; black 64 x 64 tiles inside the windows ${b.black} of ${b.total} (${(100 * b.black / b.total).toFixed(1)} %); windows ${t.top.length}`], results: { opened: r.opened, black: b.black, total: b.total, windows: t.top.length } };
  },
});
