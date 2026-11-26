// input/key-enter-chooses-menu-item: Down and Enter choose the File drop-down's first item: the drop-down closes, the engine stays idle, the main window stays.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-enter-chooses-menu-item',
  covers: ['input/key/enter'],
  run: (ctx) => I.enterMenuItem(ctx),
});
