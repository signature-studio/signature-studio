// visual/window-tmidiform-2-black: the Settings window again (F11): no all-black 64 x 64 tile is drawn with it open.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tmidiform-2-black',
  covers: ['visual/black/after-window'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowBlack(ctx, "tmidiform-2"),
});
