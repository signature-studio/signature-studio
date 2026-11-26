// audio/repeat-12288: the PCM repeat at lag 12288 is below 0.01. Check: pcm-repeat (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): no capture completed (no sound in the window), after the record scenarios (the cause is not confirmed).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/repeat-12288',
  covers: ['audio/pcm/repeat'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'pcm-repeat', { lag: 12288 }),
});
