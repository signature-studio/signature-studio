// input/mouse-double-click-trace: two clicks in a row reach the engine's hit test twice, both at the place.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-double-click-trace',
  covers: ['input/mouse/double-click'],
  run: (ctx) => I.doubleClick(ctx, true),
});
