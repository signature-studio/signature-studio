// audio/cycles-8: eight short cycles with no underrun. Check: cycles (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/cycles-8',
  covers: ['audio/underrun/cycles'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'cycles', { cycles: 8, playMs: 1500 }),
});
