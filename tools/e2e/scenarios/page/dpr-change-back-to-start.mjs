// page/dpr-change-back-to-start: the page moves to 1.5 and back: the canvas and the picture are the size they were at the start, and they are crisp.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, restoreBase, setRatio, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/dpr-change-back-to-start',
  covers: ['page/dpr-change/back-to-start'],
  async run({ guest, page, cdp }) {
    const base = await baseOf(page);
    const before = await crispNow(guest, page);
    try {
      await setRatio(cdp, page, 1.5);
      const away = await waitSettled(page, 1.5, 20000);
      await restoreBase(cdp, base);
      const back = await waitSettled(page, base.dpr, 20000);
      const r = await crispNow(guest, page);
      const same = r.c.canvas[0] === before.c.canvas[0] && r.c.canvas[1] === before.c.canvas[1];
      return { ok: away.ok && back.ok && r.ok && same, engineState: r.d.state, notes: [`ratio ${base.dpr} -> 1.5 -> ${base.dpr}: settled ${away.ok} and ${back.ok}; canvas ${before.c.canvas.join('x')} before, ${r.c.canvas.join('x')} after; crisp ${r.ok}`], results: { before: before.c.canvas, after: r.c.canvas, crisp: r.ok } };
    } finally {
      await restoreBase(cdp, base);
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
