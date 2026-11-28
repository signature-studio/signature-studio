// visual/window-tpluglistform-2-golden: the plugin list again (Alt+F8): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tpluglistform-2-golden',
  covers: ['visual/window/tpluglistform-2'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "tpluglistform-2"),
});
