// visual/theme-sytrus-golden: theme Sytrus: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Sytrus pick did not apply (the picture is the previous theme's, DEDZII Light's look; 100 % of the cells moved). The tile is
// in the lower rows, where the desktop-coloured box (guest 1686-2154 x 803-1402) covers the Theme page. new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-sytrus-golden',
  covers: ['visual/theme/sytrus'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "sytrus"),
});
