// visual/theme-ultra-green-golden: theme Ultra green: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ultra-green-golden',
  covers: ['visual/theme/ultra-green'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "ultra-green"),
});
