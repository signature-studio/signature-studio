// resilience/storm-double-click: 80 double clicks at seeded random places of the picture. Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-double-click';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/mouse-double-click'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { double: 1 }, n: 80 });
  },
});
