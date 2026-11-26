// visual/dpr-1-75-edge: device pixel ratio 1.75 (set by the page's emulation, then put back): the Settings window's edges are sharp.
// Known (Issue #1496 run, 2026-11-26): the window's top side reads a 3 device px frame (the criterion is 1 or 2); whether the product draws it or the reading counts a gradient is not checked yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { dprEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/dpr-1-75-edge',
  covers: ['visual/edge/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => dprEdge(ctx, 1.75),
});
