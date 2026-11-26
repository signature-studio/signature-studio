// audio/resume-after-stop: after a stop, the next start makes sound again (two cycles). Check: resume (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/resume-after-stop',
  covers: ['audio/output/resume-after-stop', 'audio/transport/stop'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'resume', { cycles: 2 }),
});
