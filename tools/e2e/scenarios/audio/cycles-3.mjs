// audio/cycles-3: three start and stop cycles, each playing 2 s with no underrun. Check: cycles (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): no sound in the three cycles, after the record scenarios (the cause is not confirmed: the state the record scenarios leave).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/cycles-3',
  covers: ['audio/underrun/cycles'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'cycles', { cycles: 3, playMs: 2000 }),
});
