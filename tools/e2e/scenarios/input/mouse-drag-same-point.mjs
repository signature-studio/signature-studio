// input/mouse-drag-same-point: a press and a release at one place (no move) reach the engine as a down and an up at that place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-drag-same-point',
  covers: ['input/mouse/drag-end'],
  run: (ctx) => I.dragCase(ctx, 'same'),
});
