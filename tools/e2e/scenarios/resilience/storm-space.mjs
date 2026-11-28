// resilience/storm-space: 60 presses of Space (play and stop of the transport, each press). Expected: no stop, no exception, no stuck toast, the window table readable.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/storm-space';

export default defineScenario({
  id: ID,
  covers: ['resilience/input/key-space'],
  displays: ['4k-dpr1'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, weights: { space: 1 }, n: 60 });
  },
});
