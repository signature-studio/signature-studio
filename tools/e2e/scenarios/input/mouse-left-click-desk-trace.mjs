// input/mouse-left-click-desk-trace: a left press and release on the desk reach the engine at the place, and the desk takes the hit of a window.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-left-click-desk-trace',
  covers: ['input/mouse/left-click'],
  run: (ctx) => I.clickTrace(ctx, 'left', 'desk'),
});
