// input/touch-long-press-desk: a press held for 900 ms on the desk reaches the engine (down and up); any contextmenu is cancelled.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-long-press-desk',
  covers: ['input/touch/long-press'],
  run: (ctx) => I.touchLong(ctx, 'desk'),
});
