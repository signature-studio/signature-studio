// visual/window-right-click-mixer-edge: the drop-down of the Mixer (right-click on the mixer): its four edges are sharp and 2 device px wide at most.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-mixer-edge',
  covers: ['visual/edge/right-click-mixer'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowEdge(ctx, "right-click-mixer"),
});
