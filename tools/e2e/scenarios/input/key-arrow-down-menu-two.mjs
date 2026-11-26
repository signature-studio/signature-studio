// input/key-arrow-down-menu-two: two Down presses change the File drop-down's highlight twice.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-arrow-down-menu-two',
  covers: ['input/key/arrow-down'],
  run: (ctx) => I.arrowMenu(ctx, 'down2'),
});
