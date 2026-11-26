// audio/device-open-after-stop: after Space stops the transport, the output keeps taking quanta (silence) and the transport is stopped. Check: keeps-after-stop (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/device-open-after-stop',
  covers: ['audio/output/keeps-after-stop', 'audio/transport/stop'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'keeps-after-stop'),
});
