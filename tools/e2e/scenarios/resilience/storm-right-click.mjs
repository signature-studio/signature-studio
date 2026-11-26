// resilience/storm-right-click: 60 right-button presses (held 300 ms) at seeded random places. Expected: no stop, no exception, no stuck toast, the window table readable, the drop-downs closed by the next press.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-right-click';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/mouse-right-click'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { right: 1 }, n: 60 });
  },
});
