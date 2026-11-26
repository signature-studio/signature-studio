// input/touch-drag-trace: a touch dragged across the desk reaches the engine: the down at the start, a move at the end, the up there, and the hit line of the down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-drag-trace',
  covers: ['input/touch/drag'],
  run: (ctx) => I.touchDrag(ctx, true),
});
