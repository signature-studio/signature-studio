// input/key-escape-closes-menu: Escape closes the File drop-down that a click opened.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-escape-closes-menu',
  covers: ['input/key/escape'],
  run: (ctx) => I.escapeMenu(ctx),
});
