// audio/pause-resume-with-sound: the same pause and resume with the metronome sound on. Check: pause-resume (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/pause-resume-with-sound',
  covers: ['audio/transport/pause-resume'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pause-resume', { sound: true }),
});
