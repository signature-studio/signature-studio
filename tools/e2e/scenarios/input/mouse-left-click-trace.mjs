// input/mouse-left-click-trace: a left press and release on the File label reach the engine's input at the place; the hit line of the press goes to a window.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-left-click-trace',
  covers: ['input/mouse/left-click'],
  run: (ctx) => I.clickTrace(ctx, 'left', 'File'),
});
