// audio/master-led-silences: the master mute (LED) makes the output silent; off again it sounds. Check: master-mute (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/master-led-silences',
  covers: ['audio/mute/master-led'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'master-mute'),
});
