// delivery/beacon-refuses-wrong-type: a beacon that is not application/json is refused 415 by the Worker (the beacon's form is checked at the door, not stored).
import { defineScenario } from '../../lib/scenario.mjs';
import { PERF_PATH, wireFetch } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-refuses-wrong-type',
  covers: ['delivery/beacon/refuses-wrong-type'],
  displays: ['4k-dpr1'],
  async run() {
    const r = await wireFetch(PERF_PATH, { method: 'POST', headers: { 'Content-Type': 'text/plain' }, body: '{}' });
    const ok = r.status === 415;
    return { ok, notes: [`a text/plain beacon: ${r.status}`], results: { status: r.status } };
  },
});
