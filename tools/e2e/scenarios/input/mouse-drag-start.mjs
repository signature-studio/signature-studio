// input/mouse-drag-start: a press on the desk and the start of a drag reach the engine as the left button down at the start.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-drag-start',
  covers: ['input/mouse/drag-start'],
  run: (ctx) => I.dragCase(ctx, 'start'),
});
