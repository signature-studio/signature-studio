// visual/window-tedgebrowser-golden: the Browser panel (TEdgeBrowser, the web view of the Browser): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tedgebrowser-golden',
  covers: ['visual/window/tedgebrowser'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "tedgebrowser"),
});
