// visual/theme-dedzii-light-golden: theme DEDZII Light: the main window's picture is the golden of the theme (theme.tsv).
// Known (Issue #1533 run, 2026-11-27): the DEDZII Light pick applies (the look is its own), but 5.86 % of the cells differ, all in the Browser panel's column (x 0-240: the web view,
// as window-tedgebrowser-golden). new: a measured value that does not fit the judge; the cause is not checked.
import { defineScenario } from '../../lib/scenario.mjs';
import { themeGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/theme-dedzii-light-golden',
  covers: ['visual/theme/dedzii-light'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => themeGolden(ctx, "dedzii-light"),
});
