// visual/theme-dedzii-black: theme DEDZII: no all-black 64 x 64 tile is drawn after the theme is set.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dedzii-black',
  covers: ['visual/black/after-theme'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeBlack(ctx, "dedzii"),
});
