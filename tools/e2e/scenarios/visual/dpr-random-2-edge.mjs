// visual/dpr-random-2-edge: device pixel ratio 2.06 (set by the page's emulation, then put back): the Settings window's edges are sharp.
// Known (Issue #1496 runs): intermittent. At this ratio the Settings window's opening stopped the engine (exit code 5: a libc mutex held by the guest thread) or the renderer (Target crashed) in 2 of 3 runs; run alone by the report's run, so the other scenarios of a row are not cut short.
import { defineScenario } from '../../lib/scenario.mjs';
import { dprEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/dpr-random-2-edge',
  covers: ['visual/edge/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => dprEdge(ctx, 2.06),
});
