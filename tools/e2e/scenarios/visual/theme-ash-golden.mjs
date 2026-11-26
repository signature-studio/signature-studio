// visual/theme-ash-golden: theme Ash: the main window's picture is the golden of the theme (theme.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ash-golden',
  covers: ['visual/theme/ash'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => themeGolden(ctx, "ash"),
});
