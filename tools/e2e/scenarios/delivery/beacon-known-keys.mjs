// delivery/beacon-known-keys: the beacon's top-level fields are the known ones (BEACON_KEYS: the beacon's own type): nothing else is sent.
import { defineScenario } from '../../lib/scenario.mjs';
import { BEACON_KEYS, beaconForm, beaconOf } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-known-keys',
  covers: ['delivery/beacon/known-keys'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const b = await beaconOf(page);
    if (b.text === null) return { ok: false, notes: ['no beacon was sent in 45 s'] };
    const f = beaconForm(b.text);
    const ok = f.parsed && f.unknownKeys.length === 0;
    return { ok, notes: [`${Object.keys(JSON.parse(b.text)).length} top-level fields; not among the ${BEACON_KEYS.length} known: ${f.unknownKeys.length}`], results: { unknownKeys: f.unknownKeys } };
  },
});
