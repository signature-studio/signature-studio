// visual/theme-light-pear-black: theme Light pear: no all-black 64 x 64 tile is drawn after the theme is set.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-light-pear-black',
  covers: ['visual/black/after-theme'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeBlack(ctx, "light-pear"),
});
