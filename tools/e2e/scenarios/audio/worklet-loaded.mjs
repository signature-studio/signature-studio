// audio/worklet-loaded: the output worklet is loaded (nodeReady) with no error. Check: worklet-loaded (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/worklet-loaded',
  covers: ['audio/output/worklet-loaded'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'worklet-loaded'),
});
