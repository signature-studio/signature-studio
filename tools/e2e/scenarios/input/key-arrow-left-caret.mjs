// input/key-arrow-left-caret: Left moves the caret in the box: a letter typed after two Left presses goes two letters before the end.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-arrow-left-caret',
  covers: ['input/key/arrow-left'],
  run: (ctx) => I.arrowCaret(ctx, 'left'),
});
