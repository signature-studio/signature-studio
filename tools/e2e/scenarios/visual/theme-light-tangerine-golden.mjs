// visual/theme-light-tangerine-golden: theme Light tangerine: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Light tangerine pick did not apply (the picture is the previous theme's, DEDZII Light's look; 99.6 % of the cells moved).
// The tile is in the lower rows, where the desktop-coloured box (guest 1686-2154 x 803-1402) covers the Theme page. new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-light-tangerine-golden',
  covers: ['visual/theme/light-tangerine'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "light-tangerine"),
});
