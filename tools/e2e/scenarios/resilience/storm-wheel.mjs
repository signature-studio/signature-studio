// resilience/storm-wheel: 200 wheel turns (1 to 3 notches each, either way) over seeded random places. Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-wheel';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/mouse-wheel'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { wheel: 1 }, n: 200 });
  },
});
