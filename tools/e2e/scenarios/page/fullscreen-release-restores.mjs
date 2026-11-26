// page/fullscreen-release-restores: full screen is left (exitFullscreen): the document is out of full screen, the canvas is the window's size again, and it is crisp and drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, crispNow, deskInfo, menuChangesPicture, pictureHash, waitSettled } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/fullscreen-release-restores',
  covers: ['page/fullscreen/release'],
  async run({ guest, page }) {
    const base = await baseOf(page);
    await guest.focusDesktop();
    try {
      await page.evaluate(() => document.documentElement.requestFullscreen().catch(() => null));
      await sleep(1200);
      await page.evaluate(() => document.exitFullscreen().catch(() => null));
      await sleep(1200);
      const el = await page.evaluate(() => document.fullscreenElement ? document.fullscreenElement.tagName : null);
      const s = await waitSettled(page, base.dpr, 20000);
      const r = await crispNow(guest, page);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const d = await deskInfo(page);
      const fills = d.canvas !== null && Math.abs(d.canvas.cssW - d.inner[0]) <= 1 && Math.abs(d.canvas.cssH - d.inner[1]) <= 1;
      return { ok: el === null && s.ok && r.ok && drawn && fills, engineState: d.state, notes: [`after exit: fullscreenElement ${el}; settled ${s.ok}; crisp ${r.ok}; fills the window ${fills}; the File menu drew ${drawn}`], results: { element: el, crisp: r.ok, drawn, fills } };
    } finally {
      await page.evaluate(() => { if (document.fullscreenElement) return document.exitFullscreen().catch(() => null); return null; });
      await sleep(800);
    }
  },
});
