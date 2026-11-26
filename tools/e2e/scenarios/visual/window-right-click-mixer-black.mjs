// visual/window-right-click-mixer-black: the drop-down of the Mixer (right-click on the mixer): no all-black 64 x 64 tile is drawn with it open.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-mixer-black',
  covers: ['visual/black/after-window'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowBlack(ctx, "right-click-mixer"),
});
