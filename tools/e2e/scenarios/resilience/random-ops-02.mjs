// resilience/random-ops-02: a column of 200 seeded random operations (the seed is the hash of the ID): clicks, double clicks, drags, wheel, F keys, arrows, Escape, Enter, Ctrl combinations, Space.
// Expected: no stop, no exception, no stuck toast, no failed request, the window table readable all through, the engine idle after the column.
import { defineScenario } from '../../lib/scenario.mjs';
import { startIfStopped } from '../../lib/resilience.mjs';
import { INPUT_FEATURES, opColumn } from '../../lib/resilience.mjs';

const ID = 'resilience/random-ops-02';

export default defineScenario({
  id: ID,
  covers: INPUT_FEATURES,
  displays: ['4k-dpr1', '4k-random'],
  async run({ guest, page, log }) {
    await startIfStopped(guest, page, log);
    return opColumn(guest, ID, { page, log, why: 'the random DPR row is added (a long column; the DPR 1 row is the default)' });
  },
});
