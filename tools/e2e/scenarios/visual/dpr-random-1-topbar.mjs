// visual/dpr-random-1-topbar: device pixel ratio 2.07 (set by the page's emulation, then put back): the top bar's picture is the golden of the ratio (dpr.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { dprTopbar } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/dpr-random-1-topbar',
  covers: ['visual/dpr/random-1'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => dprTopbar(ctx, 2.07),
});
