// audio/count-in-button-toggle: the count-in button changes its picture and back. Check: countin-toggle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/count-in-button-toggle',
  covers: ['audio/transport/count-in-toggle'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'countin-toggle'),
});
