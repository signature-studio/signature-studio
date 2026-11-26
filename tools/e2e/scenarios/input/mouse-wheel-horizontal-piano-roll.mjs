// input/mouse-wheel-horizontal-piano-roll: a horizontal wheel (deltaX) over the Piano roll should reach the engine and move its view. The page sends no horizontal wheel (keymap.ts and pointer.ts read deltaY only): a known gap, recorded.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-wheel-horizontal-piano-roll',
  covers: ['input/mouse/wheel-horizontal'],
  expectFail: 'new',
  run: (ctx) => I.wheelHorizontal(ctx),
});
