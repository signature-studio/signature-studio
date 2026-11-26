// audio/mouse-1000hz-idle-2s: idle while the mouse moves at 1000 Hz: no underrun and the worklet keeps taking quanta. Check: mouse-idle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/mouse-1000hz-idle-2s',
  covers: ['audio/underrun/mouse-1000hz'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'mouse-idle', { ms: 2000, hz: 1000 }),
});
