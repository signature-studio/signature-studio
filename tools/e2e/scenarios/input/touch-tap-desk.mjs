// input/touch-tap-desk: a tap on the desk reaches the engine as a left button down and up at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-tap-desk',
  covers: ['input/touch/tap'],
  run: (ctx) => I.touchTap(ctx, 'desk'),
});
