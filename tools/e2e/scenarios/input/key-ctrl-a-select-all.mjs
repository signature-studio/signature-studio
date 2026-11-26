// input/key-ctrl-a-select-all: Ctrl+A selects all the text in the box, and a typed text replaces it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-ctrl-a-select-all',
  covers: ['input/key/ctrl'],
  run: (ctx) => I.ctrlA(ctx),
});
