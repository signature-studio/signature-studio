// visual/theme-ppriime-light-golden: theme PPRIIME Light: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the PPRIIME Light tile (tile 4) lies under a desktop-coloured box (#05070a, guest 1686-2154 x 803-1402) on the Theme page; a click
// there does not apply the theme, so the picture is DEDZII Light's (99.6 % of the cells moved). new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-ppriime-light-golden',
  covers: ['visual/theme/ppriime-light'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "ppriime-light"),
});
