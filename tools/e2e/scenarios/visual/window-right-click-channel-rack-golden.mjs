// visual/window-right-click-channel-rack-golden: the drop-down of the Channel rack (right-click on the rack): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-channel-rack-golden',
  covers: ['visual/window/right-click-channel-rack'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "right-click-channel-rack"),
});
