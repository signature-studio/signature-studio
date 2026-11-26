// page/beacon-shape: the beacon the page sends is a version 3 record with no problem by the server's rules (beaconProblems of app/shared/perf.ts is empty).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconNow } from '../../lib/page.mjs';
import { beaconProblems } from '../../../../app/shared/perf.ts';

export default defineScenario({
  id: 'page/beacon-shape',
  covers: ['page/beacon/shape'],
  async run({ guest, page }) {
    const b = await beaconNow(page);
    const parsed = JSON.parse(b.body);
    const problems = beaconProblems(parsed);
    const state = (await guest.state())?.state ?? null;
    return { ok: problems.length === 0 && parsed.v === 3, engineState: state, notes: [`beacon version ${parsed.v}, ${Object.keys(parsed).length} fields; problems: ${problems.length}${problems.length ? ` (${problems.slice(0, 3).join('; ')})` : ''}`], results: { v: parsed.v, problems } };
  },
});
