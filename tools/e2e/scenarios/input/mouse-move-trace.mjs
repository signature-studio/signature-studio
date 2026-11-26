// input/mouse-move-trace: ten moves over the desk reach the engine; its last move line is the last place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-move-trace',
  covers: ['input/mouse/move'],
  run: (ctx) => I.moveTrace(ctx),
});
