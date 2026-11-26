// page/zoom-restart-notice: after the ratio changes from the one FL Studio started with, the page's log says FL keeps its scale until it is restarted (Issue #1348).
import { defineScenario } from '../../lib/scenario.mjs';
import { baseOf, restoreBase, setRatio } from '../../lib/page.mjs';
import { sleep } from '../../lib/flow.mjs';

const NOTICE = 'the browser zoom changed after FL Studio started';

export default defineScenario({
  id: 'page/zoom-restart-notice',
  covers: ['page/zoom/restart-notice'],
  async run({ page, cdp }) {
    const base = await baseOf(page);
    const target = base.dpr === 2 ? 1.5 : 2;
    try {
      await setRatio(cdp, page, target);
      await sleep(2500);
      const n = await page.evaluate((t) => (document.getElementById('log')?.textContent ?? '').split(t).length - 1, NOTICE);
      return { ok: n >= 1, engineState: null, notes: [`ratio ${base.dpr} -> ${target}: the log says the notice ${n} time(s) (it is written once per page)`], results: { notices: n } };
    } finally {
      await restoreBase(cdp, base);
      await sleep(1500);
    }
  },
});
