// page/visibility-engine-keeps-running: the page is hidden for four seconds: the engine does not stop or fail, and it is idle when the page is visible again.
import { defineScenario } from '../../lib/scenario.mjs';
import { setHidden } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/visibility-engine-keeps-running',
  covers: ['page/visibility/engine-keeps-running'],
  async run({ guest, page }) {
    try {
      await setHidden(page, true);
      let worst = null;
      for (let i = 0; i < 8; i++) { await sleep(500); const s = (await guest.state())?.state ?? null; worst = worst ?? s; if (s === 'error' || s === 'exited') worst = s; }
      await setHidden(page, false);
      await sleep(1500);
      const s = (await guest.state())?.state ?? null;
      const ok = worst !== 'error' && worst !== 'exited' && s === 'engine-idle';
      return { ok, engineState: s, notes: [`hidden for 4 s: the engine was ${worst ?? 'not read'} at worst; after visible: ${s}`], results: { worst, after: s } };
    } finally {
      await setHidden(page, false);
      await sleep(500);
    }
  },
});
