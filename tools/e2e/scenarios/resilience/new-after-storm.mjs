// resilience/new-after-storm: 60 seeded random operations (the seed is the hash of the ID), then File > New. Expected: the table is readable through the operations, no stop,
// after New the engine is idle and the main form is in the table. A stop is reported (results.engineStop) and the engine is started again.
import { defineScenario } from '../../lib/scenario.mjs';
import { resetProject } from '../../lib/common.mjs';
import { mainFormHere, planOps, runOps, seedOf, startIfStopped, stateName, stopReport } from '../../lib/resilience.mjs';

const ID = 'resilience/new-after-storm';

export default defineScenario({
  id: ID,
  covers: ['resilience/project/new-after-input-storm'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const r = await runOps(guest, planOps(seedOf(ID), 60));
    let engineStop = null, recovery = null;
    if (r.stops > 0) {
      engineStop = await stopReport(guest);
      recovery = await startIfStopped(guest, page, log);
    }
    await resetProject(guest);
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    return {
      ok: r.stops === 0 && r.reads.fail === 0 && state === 'engine-idle' && main, engineState: state,
      notes: [`60 operations (seed of the ID), stops ${r.stops}${r.stoppedAt ? ` (after operation ${r.stoppedAt.index + 1}: ${r.stoppedAt.what})` : ''}, table reads failed ${r.reads.fail}; then New: main form ${main}, state ${state}`],
      results: { ops: 60, stops: r.stops, stoppedAt: r.stoppedAt, engineStop, recovery, readsOk: r.reads.ok, readsFailed: r.reads.fail, mainForm: main },
    };
  },
});
