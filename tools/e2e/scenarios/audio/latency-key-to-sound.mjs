// audio/latency-key-to-sound: the time from the Space key to the first non-silent output is within the bound (lib/audio.mjs). Check: latency (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/latency-key-to-sound',
  covers: ['audio/output/latency'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'latency'),
});
