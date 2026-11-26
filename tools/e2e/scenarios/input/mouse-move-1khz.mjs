// input/mouse-move-1khz: 200 moves sent as fast as the page can (about 1 kHz, one pixel apart) are each processed or dropped, and the last reaches the engine. The sound (no break) is #1491.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-move-1khz',
  covers: ['input/mouse/move-1khz'],
  run: (ctx) => I.move1khz(ctx),
});
