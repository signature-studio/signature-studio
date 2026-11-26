// resilience/storm-ctrl: 60 Ctrl combinations (Ctrl+Z, Ctrl+Y, Ctrl+A; the order is seeded). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-ctrl';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/key-ctrl'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { ctrl: 1 }, n: 60 });
  },
});
