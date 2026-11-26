// audio/gap-free-playing-2s: no engine gap or skipped frames while playing with sound for 2 s. Check: gap-free (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/gap-free-playing-2s',
  covers: ['audio/output/no-gap'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'gap-free', { ms: 2000 }),
});
