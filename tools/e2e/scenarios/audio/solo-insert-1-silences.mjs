// audio/solo-insert-1-silences: the solo of the Mixer's insert 1 (Ctrl+click on its LED) silences the output of the kick (the Master), and the second Ctrl+click brings it back.
// Check: solo-insert (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/solo-insert-1-silences',
  covers: ['audio/solo/insert-1'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'solo-insert'),
});
