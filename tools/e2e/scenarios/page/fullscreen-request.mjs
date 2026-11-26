// page/fullscreen-request: the page asks for full screen: the document is then in full screen, the picture keeps drawing, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/fullscreen-request',
  covers: ['page/fullscreen/request'],
  async run({ guest, page }) {
    await guest.focusDesktop();
    try {
      const req = await page.evaluate(async () => { try { await document.documentElement.requestFullscreen(); return { ok: true }; } catch (e) { return { ok: false, err: String(e).slice(0, 120) }; } });
      await sleep(1200);
      const el = await page.evaluate(() => document.fullscreenElement ? document.fullscreenElement.tagName : null);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const state = (await guest.state())?.state ?? null;
      return { ok: req.ok && el === 'HTML' && drawn && state === 'engine-idle', engineState: state, notes: [`requested ${req.ok} (${req.err ?? 'no error'}); fullscreenElement ${el}; the File menu drew ${drawn}; engine ${state}`], results: { requested: req.ok, element: el, drawn } };
    } finally {
      await page.evaluate(() => { if (document.fullscreenElement) return document.exitFullscreen().catch(() => null); return null; });
      await sleep(1000);
    }
  },
});
