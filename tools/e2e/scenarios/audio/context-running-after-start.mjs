// audio/context-running-after-start: the AudioContext runs and the status reads running after the start. Check: context-running (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/context-running-after-start',
  covers: ['audio/output/context-running'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'context-running'),
});
