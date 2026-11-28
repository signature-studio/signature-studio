// page/html-status-line: the page's status line (#status) says the engine's state: at idle it is engine-idle.
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/html-status-line',
  covers: ['page/html/status-line'],
  async run({ guest, page }) {
    const s = await guest.state();
    const line = await page.evaluate(() => document.getElementById('status')?.dataset.state ?? null);
    const ok = s !== null && s.state === 'engine-idle' && line === s.state;
    return { ok, engineState: s?.state ?? null, notes: [`engine ${s?.state}; the status line says ${line}`], results: { engine: s?.state ?? null, line } };
  },
});
