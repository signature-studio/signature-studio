// visual/window-tmidiform-golden: the Settings window (F10): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tmidiform-golden',
  covers: ['visual/window/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "tmidiform"),
});
