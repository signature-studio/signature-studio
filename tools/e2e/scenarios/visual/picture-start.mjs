// visual/picture-start: at the start, the whole picture is the golden of the start (start.tsv).
// Known (Issue #1533 run, 2026-11-27): 5.86 % of the cells differ, all in the Browser panel's column (x 0-240: the web view, as window-tedgebrowser-golden).
// new: a measured value that does not fit the judge (the README's rule for new); the cause is not checked.
import { defineScenario } from '../../lib/scenario.mjs';
import { pictureStart } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/picture-start',
  covers: ['visual/picture/start'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => pictureStart(ctx),
});
