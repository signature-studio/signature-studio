// audio/repeat-32768: the PCM repeat at lag 32768 is below 0.01. Check: pcm-repeat (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/repeat-32768',
  covers: ['audio/pcm/repeat'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pcm-repeat', { lag: 32768 }),
});
