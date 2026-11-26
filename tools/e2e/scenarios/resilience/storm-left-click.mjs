// resilience/storm-left-click: 300 left clicks at seeded random places of the picture (the seed is the hash of the ID). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-left-click';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/mouse-left-click'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { left: 1 }, n: 300 });
  },
});
