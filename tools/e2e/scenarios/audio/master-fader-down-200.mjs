// audio/master-fader-down-200: lowering the master fader further lowers the peak. Check: fader (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
// The mark 'new' is off: it was the record scenarios' armed state (they now run last in their shard: the zz- IDs). It passed in the run for Issue 1491 (see the README).
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/master-fader-down-200',
  covers: ['audio/volume/master-fader'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'fader', { dir: 'down', px: 200 }),
});
