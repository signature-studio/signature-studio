// resilience/escape-storm: Escape pressed ten times in a row (a key the windows may take) leaves the engine idle and the main form in place. The table is read after each press.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { mainFormHere, press, readable, stateName } from '../../lib/resilience.mjs';

export default defineScenario({
  id: 'resilience/escape-storm',
  covers: ['resilience/input/key-escape'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    await guest.focusDesktop();
    let reads = 0;
    for (let i = 0; i < 10; i++) {
      await press(guest, 'Escape');
      if (await readable(guest)) reads++;
    }
    const state = await stateName(guest);
    const main = await mainFormHere(guest);
    return { ok: state === 'engine-idle' && main && reads === 10, engineState: state, notes: [`10 Escape presses, table readable after ${reads}, main form ${main}, state ${state}`], results: { presses: 10, readsOk: reads } };
  },
});
