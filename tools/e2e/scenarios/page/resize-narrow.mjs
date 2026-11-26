// page/resize-narrow: the window becomes narrow and tall (30 % of its width): the canvas takes the new room's aspect ratio, and it stays crisp.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-narrow',
  covers: ['page/resize/narrow'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    try {
      await page.setViewportSize({ width: Math.round(base.w * 0.3), height: base.h });
      const s = await waitSettled(page, base.dpr, 20000);
      const r = await crispNow(guest, page);
      const aspect = r.d.canvas ? r.d.canvas.w / r.d.canvas.h : 0;
      const wantAspect = Math.round(base.w * 0.3 * base.dpr) / Math.round(base.h * base.dpr);
      const ok = s.ok && r.ok && Math.abs(aspect - wantAspect) < 0.01;
      return { ok, engineState: r.d.state, notes: [`window ${Math.round(base.w * 0.3)}x${base.h} CSS px: settled ${s.ok}; canvas ${r.c.canvas.join('x')} (aspect ${aspect.toFixed(4)}, the room's ${wantAspect.toFixed(4)}); crisp ${r.ok}`], results: { aspect, wantAspect, crisp: r.ok } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
