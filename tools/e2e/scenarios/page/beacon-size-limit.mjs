// page/beacon-size-limit: the beacon is under the server's limit (PERF_MAX_BODY of app/shared/perf.ts): the server would refuse a bigger body.
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconNow } from '../../lib/page.mjs';
import { PERF_MAX_BODY } from '../../../../app/shared/perf.ts';

export default defineScenario({
  id: 'page/beacon-size-limit',
  covers: ['page/beacon/size-limit'],
  async run({ guest, page }) {
    const b = await beaconNow(page);
    const n = Buffer.byteLength(b.body, 'utf8');
    const state = (await guest.state())?.state ?? null;
    return { ok: n <= PERF_MAX_BODY, engineState: state, notes: [`beacon ${n} bytes; the limit ${PERF_MAX_BODY}`], results: { bytes: n, limit: PERF_MAX_BODY } };
  },
});
