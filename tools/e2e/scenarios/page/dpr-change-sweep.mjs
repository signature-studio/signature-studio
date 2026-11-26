// page/dpr-change-sweep: the page is moved through 1.25, 1.5, 1.75, 2, 2.5, 3 and back to its ratio, one after another: at each step the canvas is settled and crisp.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, restoreBase, setRatio, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/dpr-change-sweep',
  covers: ['page/dpr-change/sweep'],
  async run({ guest, page, cdp }) {
    const base = await baseOf(page);
    const steps = [1.25, 1.5, 1.75, 2, 2.5, 3];
    const notes = [];
    let good = 0;
    try {
      for (const d of steps) {
        await setRatio(cdp, page, d);
        const s = await waitSettled(page, d, 20000);
        const r = await crispNow(guest, page);
        if (s.ok && r.ok) good++;
        notes.push(`${d}: settled ${s.ok}, canvas ${r.c.canvas.join('x')}, crisp ${r.ok}`);
      }
      await restoreBase(cdp, base);
      const back = await waitSettled(page, base.dpr, 20000);
      notes.unshift(`${good} of ${steps.length} ratios settled and crisp; back to ${base.dpr}: settled ${back.ok}`);
      return { ok: good === steps.length && back.ok, engineState: (await guest.state())?.state ?? null, notes, results: { good, of: steps.length } };
    } finally {
      await restoreBase(cdp, base);
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
