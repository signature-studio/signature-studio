// audio/quanta-while-open-5s: the worklet takes quanta while the output is open (5 s). Check: quanta-rise (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/quanta-while-open-5s',
  covers: ['audio/output/quanta-while-open'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'quanta-rise', { ms: 5000 }),
});
