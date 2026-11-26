// input/mouse-move-hover-menu: the pointer moved over the File drop-down reaches the engine and the drop-down's highlight (its picture) follows it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-move-hover-menu',
  covers: ['input/mouse/move'],
  run: (ctx) => I.hoverMenu(ctx),
});
