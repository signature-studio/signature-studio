// visual/window-tmidiform-2-golden: the Settings window again (F11): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tmidiform-2-golden',
  covers: ['visual/window/tmidiform-2'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "tmidiform-2"),
});
