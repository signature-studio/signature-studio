// visual/theme-veela-edge: theme Veela: the Settings window's edges are sharp.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-veela-edge',
  covers: ['visual/edge/tmidiform'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeEdge(ctx, "veela"),
});
