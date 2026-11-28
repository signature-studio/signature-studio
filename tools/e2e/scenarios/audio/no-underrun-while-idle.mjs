// audio/no-underrun-while-idle: with the engine idle, the output takes its quanta without an underrun over 3 s (P3: no cut in the sound). Check: idle (lib/audio.mjs).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/no-underrun-while-idle',
  covers: ['page/module/audio-start', 'audio/underrun/idle'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'idle', { ms: 3000 }),
});
