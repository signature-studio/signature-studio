// audio/pattern-mode-plays: in Pattern mode the transport plays. Check: pattern-play (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/pattern-mode-plays',
  covers: ['audio/transport/pattern-play'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pattern-play'),
});
