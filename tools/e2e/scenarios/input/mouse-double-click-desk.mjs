// input/mouse-double-click-desk: two clicks in a row on the desk reach the engine as two button downs and two ups at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/mouse-double-click-desk',
  expectFail: 'new',
  covers: ['input/mouse/double-click'],
  run: (ctx) => I.doubleClick(ctx, false),
});
