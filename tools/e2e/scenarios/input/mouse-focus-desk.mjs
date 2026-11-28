// input/mouse-focus-desk: a left click on the desk gives the desk (#desktop) the keyboard focus.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/mouse-focus-desk',
  expectFail: 'new',
  covers: ['input/mouse/focus-desktop'],
  run: (ctx) => I.focusClick(ctx),
});
