// audio/tempo-faster-clicks: a higher tempo gives more metronome clicks in 8 s. Check: tempo-clicks (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/tempo-faster-clicks',
  covers: ['audio/transport/tempo-speed', 'audio/pcm/beat-clicks'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'tempo-clicks', { dir: 'up', notches: 20 }),
});
