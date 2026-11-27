// visual/theme-ppriime-dark-edge: theme PPRIIME Dark: the Settings window's edges are sharp.
// Known (Issue #1533 run, 2026-11-27): the Settings window's top side reads a 3 device px frame (the criterion is 1 or 2), as in theme-dark-edge. new: the same reading as the
// edges marked new (#1496); the frame is not checked yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ppriime-dark-edge',
  covers: ['visual/edge/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeEdge(ctx, "ppriime-dark"),
});
