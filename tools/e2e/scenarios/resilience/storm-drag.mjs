// resilience/storm-drag: 60 left-button drags (8 steps each) between seeded random places. Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-drag';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/mouse-drag'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { drag: 1 }, n: 60 });
  },
});
