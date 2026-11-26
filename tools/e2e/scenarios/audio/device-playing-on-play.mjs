// audio/device-playing-on-play: the device reads playing once Space starts the transport. Check: device-playing (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/device-playing-on-play',
  covers: ['audio/output/device-playing', 'audio/transport/play'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'device-playing', { cycles: 1 }),
});
