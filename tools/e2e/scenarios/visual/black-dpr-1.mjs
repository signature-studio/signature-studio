// visual/black-dpr-1: device pixel ratio 1 (set by the page's emulation, then put back): no all-black 64 x 64 tile outside the Browser panel.
import { defineScenario } from '../../lib/scenario.mjs';
import { blackDpr } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/black-dpr-1',
  covers: ['visual/black/after-dpr'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => blackDpr(ctx, 1),
});
