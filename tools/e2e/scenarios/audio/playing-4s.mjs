// audio/playing-4s: playing with sound: no underrun in 4 s. Check: play-underrun (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/playing-4s',
  covers: ['audio/underrun/playing'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'play-underrun', { ms: 4000 }),
});
