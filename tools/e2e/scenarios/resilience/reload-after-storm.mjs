// resilience/reload-after-storm: 60 seeded random operations (the seed is the hash of the ID), then the page is reloaded. Expected: the start completes (engine-idle, the project open,
// the main form in the table) after the reload; no stop. The random DPR row is added (the page's start is the long part; the DPR 1 row is the default).
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { MAIN_FORM, persist, windowTable } from '../../lib/common.mjs';
import { installToastWatch } from '../../lib/detect.mjs';
import { planOps, runOps, seedOf, stateName } from '../../lib/resilience.mjs';

const ID = 'resilience/reload-after-storm';

export default defineScenario({
  id: ID,
  covers: ['resilience/reload/after-input-storm'],
  displays: ['4k-dpr1', '4k-random'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const r0 = await runOps(guest, planOps(seedOf(ID), 60));
    const r = await persist(page, guest);
    await installToastWatch(page);
    const main = (await windowTable(guest).catch(() => ({ top: [] }))).top.some((w) => w.cls === MAIN_FORM);
    const state = await stateName(guest);
    return {
      ok: r0.stops === 0 && r.state === 'engine-idle' && r.opened === true && main && state === 'engine-idle', engineState: state,
      notes: [`60 operations (stops ${r0.stops}), then the reload: state ${r.state}, project open ${r.opened}, main form ${main}; the random DPR row is added (the start is the long part)`],
      results: { ops: 60, stopsBefore: r0.stops, state: r.state, opened: r.opened, mainForm: main },
    };
  },
});
