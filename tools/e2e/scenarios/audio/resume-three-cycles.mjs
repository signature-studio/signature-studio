// audio/resume-three-cycles: three stop and start cycles each make sound. Check: resume (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/resume-three-cycles',
  covers: ['audio/output/resume-after-stop', 'audio/transport/play', 'audio/transport/stop'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'resume', { cycles: 3 }),
});
