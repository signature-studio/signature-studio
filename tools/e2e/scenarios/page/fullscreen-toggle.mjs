// page/fullscreen-toggle: full screen is entered and left twice in a row: each time the state of the document is the one asked for, and the picture keeps drawing.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/fullscreen-toggle',
  covers: ['page/fullscreen/toggle'],
  async run({ guest, page }) {
    await guest.focusDesktop();
    const seq = [];
    let good = 0;
    try {
      for (let i = 0; i < 2; i++) {
        await page.evaluate(() => document.documentElement.requestFullscreen().catch(() => null));
        await sleep(1000);
        const on = await page.evaluate(() => document.fullscreenElement ? document.fullscreenElement.tagName : null);
        const f1 = await menuChangesPicture(guest, () => pictureHash(guest));
        await page.evaluate(() => document.exitFullscreen().catch(() => null));
        await sleep(1000);
        const off = await page.evaluate(() => document.fullscreenElement ? document.fullscreenElement.tagName : null);
        const f2 = await menuChangesPicture(guest, () => pictureHash(guest));
        seq.push(`${i + 1}: on ${on}, off ${off}, drawn ${f1.pops > 0 && f1.shown !== f1.after} ${f2.pops > 0 && f2.shown !== f2.after}`);
        if (on === 'HTML' && off === null && f1.pops > 0 && f2.pops > 0) good++;
      }
      return { ok: good === 2, engineState: (await guest.state())?.state ?? null, notes: seq, results: { good } };
    } finally {
      await page.evaluate(() => { if (document.fullscreenElement) return document.exitFullscreen().catch(() => null); return null; });
      await sleep(800);
    }
  },
});
