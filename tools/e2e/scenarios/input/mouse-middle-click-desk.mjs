// input/mouse-middle-click-desk: a middle press and release on the desk reach the engine as the middle button, and a window takes the hit.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-middle-click-desk',
  covers: ['input/mouse/middle-click'],
  run: (ctx) => I.clickTrace(ctx, 'middle', 'desk'),
});
