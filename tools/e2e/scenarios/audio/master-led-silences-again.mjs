// audio/master-led-silences-again: a second master mute and unmute. Check: master-mute (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/master-led-silences-again',
  covers: ['audio/mute/master-led'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'master-mute'),
});
