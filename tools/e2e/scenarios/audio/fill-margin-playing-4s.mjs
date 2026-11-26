// audio/fill-margin-playing-4s: the output keeps at least one quantum (128 frames) ahead of its read position while playing. Check: fill-margin (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/fill-margin-playing-4s',
  covers: ['audio/output/fill-margin'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'fill-margin', { ms: 4000 }),
});
