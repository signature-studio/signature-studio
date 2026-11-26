// delivery/beacon-post-accepted: the page's beacon is sent as application/json to its own origin and the Worker answers 204 (the page does not depend on it);
// X-Perf-Stored says yes or no (the stored copy: the beacon bucket).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconOf } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-post-accepted',
  covers: ['delivery/beacon/post-accepted'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const b = await beaconOf(page);
    if (b.text === null || b.status === null) return { ok: false, notes: ['no beacon answer in 45 s'] };
    const type = String(b.reqHeaders?.['content-type'] ?? '').split(';')[0].trim();
    const ok = b.status === 204 && type === 'application/json' && (b.stored === 'yes' || b.stored === 'no');
    return { ok, notes: [`POST ${b.status}; Content-Type ${type}; X-Perf-Stored ${b.stored}`], results: { status: b.status, contentType: type, stored: b.stored } };
  },
});
