// audio/channel-led-picture: the channel LED changes its picture when clicked, and back. Check: channel-led (lib/audio.mjs). The display is 4k-dpr1: the output's numbers do not depend on the DPR.
import { defineScenario } from '../../lib/scenario.mjs';
import { runCheck } from '../../lib/audio.mjs';

export default defineScenario({
  id: 'audio/channel-led-picture',
  covers: ['audio/mute/channel-led'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => runCheck(ctx, 'channel-led'),
});
