// audio/idle-1s: idle: no underrun in 1 s, and the worklet keeps taking quanta. Check: idle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/idle-1s',
  covers: ['audio/underrun/idle'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'idle', { ms: 1000 }),
});
