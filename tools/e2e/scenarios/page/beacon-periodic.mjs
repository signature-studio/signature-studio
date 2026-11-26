// page/beacon-periodic: while the engine runs the page sends a beacon every 30 s: in 35 s the page sends at least one more.
import { defineScenario } from '../../lib/scenario.mjs';
import { sleep } from '../../lib/flow.mjs';

export default defineScenario({
  id: 'page/beacon-periodic',
  covers: ['page/beacon/periodic'],
  async run({ guest, page }) {
    const before = await page.evaluate(() => window.__app.perf.sent);
    await sleep(35000);
    const after = await page.evaluate(() => window.__app.perf.sent);
    const state = (await guest.state())?.state ?? null;
    return { ok: after > before, engineState: state, notes: [`beacons sent in 35 s: ${after - before} (the count ${before} -> ${after})`], results: { before, after } };
  },
});
