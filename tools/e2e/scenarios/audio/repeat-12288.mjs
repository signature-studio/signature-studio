// audio/repeat-12288: the PCM repeat at lag 12288 is below 0.01. Check: pcm-repeat (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// The mark 'new' is off: it was the record scenarios' armed state (they now run last in their shard: the zz- IDs). It passed in the run for Issue 1491 (see the README).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/repeat-12288',
  covers: ['audio/pcm/repeat'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pcm-repeat', { lag: 12288 }),
});
