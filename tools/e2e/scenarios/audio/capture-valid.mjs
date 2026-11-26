// audio/capture-valid: an 8 s capture of the output has no missing frames (valid). Check: pcm-capture (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/capture-valid',
  covers: ['audio/pcm/capture-valid'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pcm-capture'),
});
