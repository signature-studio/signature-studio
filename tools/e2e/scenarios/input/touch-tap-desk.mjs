// input/touch-tap-desk: a tap on the desk reaches the engine as a left button down and up at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/touch-tap-desk',
  expectFail: 'new',
  covers: ['input/touch/tap'],
  run: (ctx) => I.touchTap(ctx, 'desk'),
});
