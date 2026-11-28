// page/history-hash-back-forward: the page's history moves back and forward over two fragments: the page keeps running (no error, idle) and draws after each step.
import { defineScenario } from '../../lib/scenario.mjs';
import { menuChangesPicture, pictureHash } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/history-hash-back-forward',
  covers: ['page/history/hash'],
  async run({ guest, page }) {
    try {
      await page.evaluate(() => { location.hash = '#e2e-a'; });
      await sleep(400);
      await page.evaluate(() => { location.hash = '#e2e-b'; });
      await sleep(400);
      await page.goBack();
      await sleep(800);
      const back = await page.evaluate(() => location.hash);
      await page.goForward();
      await sleep(800);
      const fwd = await page.evaluate(() => location.hash);
      const f = await menuChangesPicture(guest, () => pictureHash(guest));
      const drawn = f.pops > 0 && f.shown !== f.after;
      const state = (await guest.state())?.state ?? null;
      return { ok: back === '#e2e-a' && fwd === '#e2e-b' && drawn && state === 'engine-idle', engineState: state, notes: [`back to ${back}, forward to ${fwd}; the File menu drew ${drawn}; engine ${state}`], results: { back, fwd, drawn } };
    } finally {
      await page.evaluate(() => history.replaceState(null, '', location.pathname + location.search));
      await sleep(300);
    }
  },
});
