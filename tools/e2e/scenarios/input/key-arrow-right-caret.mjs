// input/key-arrow-right-caret: Right moves the caret in the box: a letter typed after Home and two Right presses goes after two letters.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-arrow-right-caret',
  covers: ['input/key/arrow-right'],
  run: (ctx) => I.arrowCaret(ctx, 'right'),
});
