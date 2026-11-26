// page/resize-picture-kept: after the window is resized the engine draws the new picture (the File menu changes it), at the new size.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, deskInfo, menuChangesPicture, pictureHash, waitSettled } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/resize-picture-kept',
  covers: ['page/resize/picture-kept'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    try {
      await page.setViewportSize({ width: Math.round(base.w * 0.7), height: Math.round(base.h * 0.7) });
      const s = await waitSettled(page, base.dpr, 20000);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const d = await deskInfo(page);
      return { ok: s.ok && drawn, engineState: d.state, notes: [`window ${Math.round(base.w * 0.7)}x${Math.round(base.h * 0.7)} CSS px: settled ${s.ok}; the File menu drew ${drawn} at the picture ${d.guest.w}x${d.guest.h}`], results: { drawn, guest: [d.guest.w, d.guest.h] } };
    } finally {
      await page.setViewportSize({ width: base.w, height: base.h });
      await waitSettled(page, base.dpr, 20000).catch(() => null);
    }
  },
});
