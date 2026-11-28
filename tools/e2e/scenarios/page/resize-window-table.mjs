// page/resize-window-table: the engine's main window (the window table) follows the window: after a resize it is the screen's size, at the screen's corner.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, deskInfo, mainForm, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-window-table',
  covers: ['page/resize/window-table'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    try {
      const before = await mainForm(guest);
      await page.setViewportSize({ width: Math.round(base.w * 0.75), height: Math.round(base.h * 0.75) });
      const s = await waitSettled(page, base.dpr, 20000);
      const after = await mainForm(guest);
      const d = await deskInfo(page);
      const ok = s.ok && before !== null && after !== null && Math.abs(after.w - d.guest.w) <= 2 && Math.abs(after.h - d.guest.h) <= 2 && Math.abs(after.x) <= 2 && Math.abs(after.y) <= 2;
      return { ok, engineState: d.state, notes: [`main form ${before ? `${before.w}x${before.h}` : 'none'} -> ${after ? `${after.w}x${after.h} at ${after.x},${after.y}` : 'none'}; the guest's screen ${d.guest.w}x${d.guest.h}`], results: { before: before ? [before.w, before.h] : null, after: after ? [after.w, after.h, after.x, after.y] : null } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
