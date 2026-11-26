// audio/stop-quiet-3s: after Space stops, nothing plays for 3 s. Check: stop-quiet (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/stop-quiet-3s',
  covers: ['audio/transport/stop'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'stop-quiet', { ms: 3000 }),
});
