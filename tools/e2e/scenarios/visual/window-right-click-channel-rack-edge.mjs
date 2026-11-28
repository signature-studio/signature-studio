// visual/window-right-click-channel-rack-edge: the drop-down of the Channel rack (right-click on the rack): its four edges are sharp and 2 device px wide at most.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-channel-rack-edge',
  covers: ['visual/edge/right-click-channel-rack'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowEdge(ctx, "right-click-channel-rack"),
});
