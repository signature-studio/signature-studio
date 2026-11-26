// resilience/storm-enter: 100 presses of Enter (the default button of a box, or an item of a drop-down, is chosen by it). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-enter';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/key-enter'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { enter: 1 }, n: 100 });
  },
});
