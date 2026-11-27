// input/touch-long-press-desk: a press held for 900 ms on the desk reaches the engine (down and up); any contextmenu is cancelled.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// The desk's centre is covered by the WebView2 face of the closed trial window (an iframe left over the desk after File > New), so the press never reaches the desk (Issue #1532: a product defect, see the notes' element).
export default defineScenario({
  id: 'input/touch-long-press-desk',
  expectFail: 'new',
  covers: ['input/touch/long-press'],
  run: (ctx) => I.touchLong(ctx, 'desk'),
});
