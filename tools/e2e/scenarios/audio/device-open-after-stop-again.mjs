// audio/device-open-after-stop-again: a second time, after a start and a stop, the output keeps taking quanta and the transport is stopped. Check: keeps-after-stop (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/device-open-after-stop-again',
  covers: ['audio/output/keeps-after-stop', 'audio/transport/stop'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'keeps-after-stop'),
});
