// audio/channel-led-picture-wide: the channel LED toggle, the region read wider. Check: channel-led (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/channel-led-picture-wide',
  covers: ['audio/mute/channel-led'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'channel-led', { r: 16 }),
});
