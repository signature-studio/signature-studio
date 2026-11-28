// input/mouse-wheel-down-piano-roll: three notches down over the Piano roll reach the engine (wheel -120 each) and its picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-wheel-down-piano-roll',
  covers: ['input/mouse/wheel-vertical'],
  run: (ctx) => I.wheelCase(ctx, 100),
});
