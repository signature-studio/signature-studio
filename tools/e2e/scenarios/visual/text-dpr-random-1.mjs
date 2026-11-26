// visual/text-dpr-random-1: device pixel ratio 2.07 (set by the page's emulation, then put back): the menu bar's text is drawn.
import { defineScenario } from '../../lib/scenario.mjs';
import { textDpr } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/text-dpr-random-1',
  covers: ['visual/text/topbar'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => textDpr(ctx, 2.07),
});
