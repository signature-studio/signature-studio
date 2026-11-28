// audio/pan-master-wheel: the Master's knob (the Mixer's big knob at the Master track) moves the left-right balance of the output: turned, the balance moves;
// turned back, it returns. Check: pan-master (lib/audio.mjs). The display is 4k-dpr1: the output's levels do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/pan-master-wheel',
  covers: ['audio/pan/master'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pan-master', { notches: 4 }),
});
