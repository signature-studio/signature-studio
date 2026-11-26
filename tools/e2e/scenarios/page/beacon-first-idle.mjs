// page/beacon-first-idle: the page sent its first beacon when the engine reached its first idle, and the server accepted it (the page's own record of its beacons).
import { defineScenario } from '../../lib/scenario.mjs';

export default defineScenario({
  id: 'page/beacon-first-idle',
  covers: ['page/beacon/first-idle'],
  async run({ guest, page }) {
    const h = await page.evaluate(() => window.__app.perf.history.map((x) => ({ reason: x.reason, ok: x.result.ok, status: x.result.status })));
    const first = h.find((x) => x.reason === 'first-idle') ?? null;
    const state = (await guest.state())?.state ?? null;
    return { ok: first !== null && first.ok === true, engineState: state, notes: [`beacons sent by this page: ${h.length} (${h.map((x) => `${x.reason} ${x.status}`).join(', ')})`], results: { beacons: h.length, first } };
  },
});
