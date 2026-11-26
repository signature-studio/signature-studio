// audio/metronome-on-sound-2s: with the metronome on, playing gives non-silent output for 2 s, with no underrun. Check: metronome-sound (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/metronome-on-sound-2s',
  covers: ['audio/metronome/on'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'metronome-sound', { ms: 2000 }),
});
