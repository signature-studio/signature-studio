// page/focus-engine-keeps-running: the window is without the focus for three seconds: the engine keeps running (it does not stop or fail) and is idle again with the focus.
import { defineScenario } from '../../lib/scenario.mjs';
import { setFocused } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/focus-engine-keeps-running',
  covers: ['page/focus/engine-keeps-running'],
  async run({ guest, page }) {
    try {
      await setFocused(page, false);
      let worst = null;
      for (let i = 0; i < 6; i++) { await sleep(500); const s = (await guest.state())?.state ?? null; if (worst === null) worst = s; if (s === 'error' || s === 'exited') worst = s; }
      await setFocused(page, true);
      await sleep(1000);
      const s = (await guest.state())?.state ?? null;
      return { ok: worst !== 'error' && worst !== 'exited' && s === 'engine-idle', engineState: s, notes: [`without focus for 3 s: the engine was ${worst} at worst; with focus: ${s}`], results: { worst, after: s } };
    } finally {
      await setFocused(page, true);
      await sleep(400);
    }
  },
});
