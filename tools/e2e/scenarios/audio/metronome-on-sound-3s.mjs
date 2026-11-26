// audio/metronome-on-sound-3s: the same over 3 s. Check: metronome-sound (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/metronome-on-sound-3s',
  covers: ['audio/metronome/on'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'metronome-sound', { ms: 3000 }),
});
