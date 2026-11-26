// resilience/storm-f-keys: 120 presses of the F keys F5 to F12 (the order is seeded). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-f-keys';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/key-f-keys'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { fkey: 1 }, n: 120 });
  },
});
