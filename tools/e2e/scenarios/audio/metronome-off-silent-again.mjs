// audio/metronome-off-silent-again: a second metronome off and on. Check: metronome-off (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/metronome-off-silent-again',
  covers: ['audio/metronome/off'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'metronome-off'),
});
