// audio/tempo-slower-clicks: a lower tempo gives fewer metronome clicks in 8 s. Check: tempo-clicks (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/tempo-slower-clicks',
  covers: ['audio/transport/tempo-speed', 'audio/pcm/beat-clicks'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'tempo-clicks', { dir: 'down', notches: 20 }),
});
