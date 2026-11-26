// audio/pause-resume: a stop pauses the playing quanta; the next start resumes them. Check: pause-resume (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/pause-resume',
  covers: ['audio/transport/pause-resume'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'pause-resume'),
});
