// audio/master-fader-up-60: raising the master fader raises the RMS of the output. Check: fader (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/master-fader-up-60',
  covers: ['audio/volume/master-fader'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'fader', { dir: 'up', px: 60 }),
});
