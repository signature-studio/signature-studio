// input/mouse-drag-end: the drag ends at its last place: the move there and the button up at that place reach the engine.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-drag-end',
  covers: ['input/mouse/drag-end'],
  run: (ctx) => I.dragCase(ctx, 'end'),
});
