// input/mouse-drag-middle: the moves while the left button is held reach the engine, at the places between the start and the end.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-drag-middle',
  covers: ['input/mouse/drag-middle'],
  run: (ctx) => I.dragCase(ctx, 'middle'),
});
