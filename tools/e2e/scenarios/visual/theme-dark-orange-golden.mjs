// visual/theme-dark-orange-golden: theme Dark orange: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Dark orange tile (tile 7) lies under the desktop-coloured box (guest 1686-2154 x 803-1402) on the Theme page; a click there
// does not apply the theme, so the picture is the Default theme's (100 % of the cells moved). new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dark-orange-golden',
  covers: ['visual/theme/dark-orange'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "dark-orange"),
});
