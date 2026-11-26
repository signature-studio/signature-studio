// delivery/beacon-first-idle-form: the measurement beacon the page sends (POST /__perf) has the form app/shared/perf.ts checks: beaconProblems is empty.
// The beacon is the page's own, captured as it is sent (the next one of the page's 30 s period at most).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconForm, beaconOf } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-first-idle-form',
  covers: ['delivery/beacon/first-idle-form'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const b = await beaconOf(page);
    if (b.text === null) return { ok: false, notes: ['no beacon was sent in 45 s'] };
    const f = beaconForm(b.text);
    const ok = f.parsed && f.problems.length === 0;
    return { ok, notes: [`beacon of ${b.text.length} bytes: ${f.problems.length} problems${f.problems.length ? `: ${f.problems.slice(0, 3).join('; ')}` : ''}`], results: { bytes: b.text.length, problems: f.problems.slice(0, 20) } };
  },
});
