// visual/language-de-edge: language de: the Settings window's edges are sharp.
// Known (Issue #1496 run, 2026-11-26): the window's top side reads a 3 device px frame (the criterion is 1 or 2); whether the product draws it or the reading counts a gradient is not checked yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { languageEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/language-de-edge',
  covers: ['visual/edge/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => languageEdge(ctx, "de"),
});
