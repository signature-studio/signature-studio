// input/key-arrow-up-menu: Up moves the File drop-down's highlight back: Down, Down, Up leaves it where the first Down put it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-arrow-up-menu',
  covers: ['input/key/arrow-up'],
  run: (ctx) => I.arrowMenu(ctx, 'up'),
});
