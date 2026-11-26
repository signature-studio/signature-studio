// input/mouse-double-click-desk: two clicks in a row on the desk reach the engine as two button downs and two ups at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-double-click-desk',
  covers: ['input/mouse/double-click'],
  run: (ctx) => I.doubleClick(ctx, false),
});
