// resilience/storm-arrows: 200 presses of the arrow keys (left, up, right, down; the order is seeded). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-arrows';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/key-arrow'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { arrow: 1 }, n: 200 });
  },
});
