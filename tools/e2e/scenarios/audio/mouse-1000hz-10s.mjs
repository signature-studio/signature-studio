// audio/mouse-1000hz-10s: playing with sound while the mouse moves at 1000 Hz: no underrun (10 s). Check: mouse-underrun (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/mouse-1000hz-10s',
  covers: ['audio/underrun/mouse-1000hz'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'mouse-underrun', { ms: 10000, hz: 1000 }),
});
