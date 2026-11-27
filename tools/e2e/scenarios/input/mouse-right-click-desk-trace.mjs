// input/mouse-right-click-desk-trace: a right press and release on the desk reach the engine as the right button at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/mouse-right-click-desk-trace',
  expectFail: 'new',
  covers: ['input/mouse/right-click'],
  run: (ctx) => I.clickTrace(ctx, 'right', 'desk'),
});
