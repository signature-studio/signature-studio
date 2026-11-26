// visual/window-right-click-channel-rack-black: the drop-down of the Channel rack (right-click on the rack): no all-black 64 x 64 tile is drawn with it open.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-channel-rack-black',
  covers: ['visual/black/after-window'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowBlack(ctx, "right-click-channel-rack"),
});
