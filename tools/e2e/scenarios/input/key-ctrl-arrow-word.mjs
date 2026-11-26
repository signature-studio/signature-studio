// input/key-ctrl-arrow-word: Ctrl+Left moves the caret to the start of the previous word; a typed letter goes there.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ctrl-arrow-word',
  covers: ['input/key/ctrl-arrow'],
  run: (ctx) => I.ctrlArrow(ctx),
});
