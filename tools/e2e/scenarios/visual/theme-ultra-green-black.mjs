// visual/theme-ultra-green-black: theme Ultra green: no all-black 64 x 64 tile is drawn after the theme is set.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ultra-green-black',
  covers: ['visual/black/after-theme'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeBlack(ctx, "ultra-green"),
});
