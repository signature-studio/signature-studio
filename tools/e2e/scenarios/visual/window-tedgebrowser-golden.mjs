// visual/window-tedgebrowser-golden: the Browser panel (TEdgeBrowser, the web view of the Browser): its picture is the golden of the window (window.tsv).
// Known (Issue #1533 run, 2026-11-27): the Browser panel's web view is black in this run (the Default look's black), and the golden's cells differ in every cell (100 %).
// The web view's picture is not a value the golden can hold (cause not checked). new: a measured value that does not fit the judge (the README's rule for new).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-tedgebrowser-golden',
  covers: ['visual/window/tedgebrowser'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => windowGolden(ctx, "tedgebrowser"),
});
