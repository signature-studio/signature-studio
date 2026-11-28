// visual/theme-veela-golden: theme Veela: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Veela tile (tile 5) lies under a desktop-coloured box (#05070a, guest 1686-2154 x 803-1402) on the Theme page; a click there does not
// apply the theme, so the picture is DEDZII Light's (100 % of the cells moved). new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-veela-golden',
  covers: ['visual/theme/veela'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "veela"),
});
