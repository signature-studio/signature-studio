// page/resize-shrink: the window shrinks to 60 %: the canvas follows the window, stays crisp, and the engine's main window follows the screen.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, mainForm, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-shrink',
  covers: ['page/resize/shrink'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    try {
      await page.setViewportSize({ width: Math.round(base.w * 0.6), height: Math.round(base.h * 0.6) });
      const s = await waitSettled(page, base.dpr, 20000);
      const r = await crispNow(guest, page);
      const m = await mainForm(guest);
      const followed = m !== null && Math.abs(m.w - r.d.guest.w) <= 2 && Math.abs(m.h - r.d.guest.h) <= 2;
      return { ok: s.ok && r.ok && followed, engineState: r.d.state, notes: [`window ${Math.round(base.w * 0.6)}x${Math.round(base.h * 0.6)} CSS px: settled ${s.ok} in ${s.waitedMs} ms; canvas ${r.c.canvas.join('x')}; crisp ${r.ok}; main form ${m ? `${m.w}x${m.h}` : 'none'} vs guest ${r.d.guest.w}x${r.d.guest.h}`], results: { crisp: r.ok, mainForm: m ? [m.w, m.h] : null, guest: [r.d.guest.w, r.d.guest.h] } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
