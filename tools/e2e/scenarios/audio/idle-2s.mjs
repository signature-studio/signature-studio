// audio/idle-2s: idle: no underrun in 2 s. Check: idle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/idle-2s',
  covers: ['audio/underrun/idle'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'idle', { ms: 2000 }),
});
