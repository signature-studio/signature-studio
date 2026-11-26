// audio/metronome-off-silent: with the metronome off the transport plays in silence; on again it makes sound. Check: metronome-off (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/metronome-off-silent',
  covers: ['audio/metronome/off'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'metronome-off'),
});
