// visual/theme-dedzii-golden: theme DEDZII: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dedzii-golden',
  covers: ['visual/theme/dedzii'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "dedzii"),
});
