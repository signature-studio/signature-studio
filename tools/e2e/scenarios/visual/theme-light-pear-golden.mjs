// visual/theme-light-pear-golden: theme Light pear: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the Light pear tile (tile 10) is below the Theme page's visible part, and the scroll puts it under a desktop-coloured box
// (#05070a, guest 1686-2154 x 803-1402); a click there does not apply the theme, so the picture is DEDZII Light's (99.2 % of the cells moved). new: as theme-ash-golden.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-light-pear-golden',
  covers: ['visual/theme/light-pear'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "light-pear"),
});
