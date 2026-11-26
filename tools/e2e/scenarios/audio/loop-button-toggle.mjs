// audio/loop-button-toggle: the loop button (the top bar) changes its picture when clicked, and back. Check: loop-toggle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/loop-button-toggle',
  covers: ['audio/transport/loop-toggle'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'loop-toggle'),
});
