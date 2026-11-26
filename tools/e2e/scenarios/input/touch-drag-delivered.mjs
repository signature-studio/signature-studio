// input/touch-drag-delivered: a touch dragged across the desk reaches the engine as a drag (the down at the start, the move and the up at the end).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-drag-delivered',
  covers: ['input/touch/drag'],
  run: (ctx) => I.touchDrag(ctx, false),
});
