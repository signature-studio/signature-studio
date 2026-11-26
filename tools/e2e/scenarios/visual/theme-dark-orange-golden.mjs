// visual/theme-dark-orange-golden: theme Dark orange: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dark-orange-golden',
  covers: ['visual/theme/dark-orange'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "dark-orange"),
});
