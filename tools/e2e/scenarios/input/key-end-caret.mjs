// input/key-end-caret: End moves the caret to the end of the box: a letter typed there goes last.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-end-caret',
  covers: ['input/key/end'],
  run: (ctx) => I.caretKeys(ctx, 'end'),
});
