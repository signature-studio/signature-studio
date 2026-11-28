// audio/play-1s: Space starts the transport: the playing quanta rise in 1 s. Check: play-rise (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/play-1s',
  covers: ['audio/transport/play'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'play-rise', { ms: 1000 }),
});
