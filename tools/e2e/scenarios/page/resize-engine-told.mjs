// page/resize-engine-told: after the window changes size the engine is told the new screen (the page's record of the change: the room, sent to the engine).
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-engine-told',
  covers: ['page/resize/engine-told'],
  async run({ page }) {
    const base = await baseOf(page);
    try {
      await page.setViewportSize({ width: Math.round(base.w * 0.6), height: Math.round(base.h * 0.6) });
      const s = await waitSettled(page, base.dpr, 20000);
      const rec = await page.evaluate(() => { const r = window.__app.screen.resizes; const last = r[r.length - 1] ?? null; return last ? { sent: last.sent, room: [last.room.width, last.room.height], guest: [last.guest.width, last.guest.height] } : null; });
      const ok = s.ok && rec !== null && rec.sent === true && rec.guest[0] > 0;
      return { ok, engineState: s.d.state, notes: [`the last change: room ${rec ? rec.room.join('x') : 'none'} CSS px, the engine is told ${rec ? rec.guest.join('x') : '-'} (sent ${rec ? rec.sent : '-'})`], results: rec };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
