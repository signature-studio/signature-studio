// input/mouse-right-click-desk-trace: a right press and release on the desk reach the engine as the right button at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-right-click-desk-trace',
  covers: ['input/mouse/right-click'],
  run: (ctx) => I.clickTrace(ctx, 'right', 'desk'),
});
