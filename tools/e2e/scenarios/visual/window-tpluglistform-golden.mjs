// visual/window-tpluglistform-golden: the plugin list (F8): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tpluglistform-golden',
  covers: ['visual/window/tpluglistform'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "tpluglistform"),
});
