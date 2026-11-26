// page/beacon-accepted: the server accepts a beacon sent now (204) and says it stored it (X-Perf-Stored: yes).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconNow } from '../../lib/page.mjs';

export default defineScenario({
  id: 'page/beacon-accepted',
  covers: ['page/beacon/accepted'],
  async run({ guest, page }) {
    const b = await beaconNow(page);
    const state = (await guest.state())?.state ?? null;
    return { ok: b.result.ok === true && b.result.status === 204 && b.result.stored === true, engineState: state, notes: [`server answer: ${b.result.status}, stored ${b.result.stored} (${b.result.note})`], results: b.result };
  },
});
