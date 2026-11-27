// input/mouse-left-click-desk-trace: a left press and release on the desk reach the engine at the place, and the desk takes the hit of a window.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/mouse-left-click-desk-trace',
  expectFail: 'new',
  covers: ['input/mouse/left-click'],
  run: (ctx) => I.clickTrace(ctx, 'left', 'desk'),
});
