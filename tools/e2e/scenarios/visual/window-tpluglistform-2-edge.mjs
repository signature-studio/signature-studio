// visual/window-tpluglistform-2-edge: the plugin list again (Alt+F8): its four edges are sharp and 2 device px wide at most.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tpluglistform-2-edge',
  covers: ['visual/edge/tpluglistform-2'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => windowEdge(ctx, "tpluglistform-2"),
});
