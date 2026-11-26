// input/key-arrow-down-menu: Down moves the File drop-down's highlight (its picture changes).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-arrow-down-menu',
  covers: ['input/key/arrow-down'],
  run: (ctx) => I.arrowMenu(ctx, 'down'),
});
