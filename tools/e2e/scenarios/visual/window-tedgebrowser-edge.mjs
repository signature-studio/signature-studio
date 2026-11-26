// visual/window-tedgebrowser-edge: the Browser panel (TEdgeBrowser, the web view of the Browser): its four edges are sharp and 2 device px wide at most.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tedgebrowser-edge',
  covers: ['visual/edge/tedgebrowser'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowEdge(ctx, "tedgebrowser"),
});
