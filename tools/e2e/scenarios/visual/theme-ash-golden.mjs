// visual/theme-ash-golden: theme Ash: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Ash tile (tile 6) lies under a desktop-coloured box (#05070a, guest 1686-2154 x 803-1402) on the Theme page; a click there does not
// apply the theme, so the picture is the Default theme's (97.7 % of the cells moved). new: the product draws the box over the tiles (cause not checked).
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ash-golden',
  covers: ['visual/theme/ash'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "ash"),
});
