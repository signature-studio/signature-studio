// visual/dpr-3-topbar: device pixel ratio 3 (set by the page's emulation, then put back): the top bar's picture is the golden of the ratio (dpr.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { dprTopbar } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/dpr-3-topbar',
  covers: ['visual/dpr/3'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => dprTopbar(ctx, 3),
});
