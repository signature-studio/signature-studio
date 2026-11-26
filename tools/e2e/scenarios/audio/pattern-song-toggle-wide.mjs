// audio/pattern-song-toggle-wide: the Pattern/Song toggle, the region read wider. Check: pattern-song-toggle (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/pattern-song-toggle-wide',
  covers: ['audio/transport/pattern-song'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pattern-song-toggle', { r: 20 }),
});
