// visual/theme-light-cherry-golden: theme Light cherry: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-light-cherry-golden',
  covers: ['visual/theme/light-cherry'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "light-cherry"),
});
