// audio/repeat-4096: the PCM repeat at lag 4096 (a control lag) is below 0.01. Check: pcm-repeat (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/repeat-4096',
  covers: ['audio/pcm/repeat'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pcm-repeat', { lag: 4096 }),
});
