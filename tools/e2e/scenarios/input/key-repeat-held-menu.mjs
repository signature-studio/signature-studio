// input/key-repeat-held-menu: A Down key held for three key downs moves the drop-down's highlight as three separate presses do.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-repeat-held-menu',
  covers: ['input/key/repeat'],
  run: (ctx) => I.repeatMenu(ctx),
});
