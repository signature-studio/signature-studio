// audio/latency-key-to-sound-again: a second measurement of the key-to-sound latency. Check: latency (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/latency-key-to-sound-again',
  covers: ['audio/output/latency'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'latency'),
});
