// input/mouse-middle-click-desk: a middle press and release on the desk reach the engine as the middle button, and a window takes the hit.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/mouse-middle-click-desk',
  expectFail: 'new',
  covers: ['input/mouse/middle-click'],
  run: (ctx) => I.clickTrace(ctx, 'middle', 'desk'),
});
