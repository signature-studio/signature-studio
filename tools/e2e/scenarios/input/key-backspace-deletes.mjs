// input/key-backspace-deletes: Backspace deletes the last letter typed: the committed name is the rest.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-backspace-deletes',
  covers: ['input/key/backspace'],
  run: (ctx) => I.backspaceCase(ctx),
});
