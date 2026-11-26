// audio/play-empty-project: in the empty project (New) the transport still plays. Check: play-rise (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/play-empty-project',
  covers: ['audio/transport/play'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'play-rise', { ms: 2000 }),
});
