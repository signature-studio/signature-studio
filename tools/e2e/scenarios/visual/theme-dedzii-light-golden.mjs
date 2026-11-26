// visual/theme-dedzii-light-golden: theme DEDZII Light: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dedzii-light-golden',
  covers: ['visual/theme/dedzii-light'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "dedzii-light"),
});
