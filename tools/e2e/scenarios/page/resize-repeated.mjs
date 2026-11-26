// page/resize-repeated: the window is resized ten times (60 % and 100 %, alternately): each time the canvas follows and is crisp, and the last size is the first one's.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-repeated',
  covers: ['page/resize/repeated'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    const notes = [];
    let good = 0;
    const first = await crispNow(guest, page);
    try {
      for (let i = 0; i < 10; i++) {
        const f = i % 2 === 0 ? 0.6 : 1;
        await page.setViewportSize({ width: Math.round(base.w * f), height: Math.round(base.h * f) });
        const s = await waitSettled(page, base.dpr, 20000);
        const r = await crispNow(guest, page);
        if (s.ok && r.ok) good++;
        else notes.push(`step ${i + 1} (${f}): settled ${s.ok}, crisp ${r.ok}`);
      }
      const last = await crispNow(guest, page);
      notes.unshift(`${good} of 10 resizes settled and crisp; the last canvas ${last.c.canvas.join('x')}`);
      const same = last.c.canvas[0] === first.c.canvas[0] && last.c.canvas[1] === first.c.canvas[1];
      notes.push(`the first canvas ${first.c.canvas.join('x')}, the last ${last.c.canvas.join('x')}`);
      return { ok: good === 10 && same && last.ok, engineState: last.d.state, notes, results: { good, same } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
