// visual/theme-ppriime-dark-golden: theme PPRIIME Dark: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ppriime-dark-golden',
  covers: ['visual/theme/ppriime-dark'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "ppriime-dark"),
});
