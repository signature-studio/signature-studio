// resilience/table-reads-200: the window table is read 200 times in a row (no action between the reads). Expected: every read answers (the page may ask again: the number of asks is counted),
// the main form is in the table each time, no stop.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { mainFormHere, readable, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/table-reads-200',
  covers: ['resilience/window/table-readable-repeat'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    const retries0 = guest.dumpRetries;
    let ok = 0, fail = 0, mainMissing = 0;
    for (let i = 0; i < 200; i++) {
      if (await readable(guest)) { ok++; if (!(await mainFormHere(guest))) mainMissing++; } else fail++;
    }
    const state = await stateName(guest);
    const retries = guest.dumpRetries - retries0;
    return {
      ok: fail === 0 && mainMissing === 0 && state === 'engine-idle', engineState: state,
      notes: [`200 reads: ok ${ok}, failed ${fail}, main form missing ${mainMissing}, dumpRetries ${retries}`],
      results: { reads: 200, readsOk: ok, readsFailed: fail, mainFormMissing: mainMissing, dumpRetries: retries },
    };
  },
});
