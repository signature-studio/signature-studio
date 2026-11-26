// visual/window-tpluglistform-edge: the plugin list (F8): its four edges are sharp and 2 device px wide at most.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tpluglistform-edge',
  covers: ['visual/edge/tpluglistform'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => windowEdge(ctx, "tpluglistform"),
});
