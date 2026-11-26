// visual/window-tedgebrowser-black: the Browser panel (TEdgeBrowser, the web view of the Browser): no all-black 64 x 64 tile is drawn with it open.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tedgebrowser-black',
  covers: ['visual/black/after-window'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowBlack(ctx, "tedgebrowser"),
});
