// audio/master-fader-down-200: lowering the master fader further lowers the peak. Check: fader (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// expectFail 'new' (found by this run, Issue not yet made): the fader started already lowered (the fader restore of master-fader-down-120 did not restore it); the peak did not fall 0.8 times.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/master-fader-down-200',
  covers: ['audio/volume/master-fader'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => runCheck(ctx, 'fader', { dir: 'down', px: 200 }),
});
