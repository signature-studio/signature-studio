// visual/window-right-click-mixer-golden: the drop-down of the Mixer (right-click on the mixer): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-mixer-golden',
  covers: ['visual/window/right-click-mixer'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "right-click-mixer"),
});
