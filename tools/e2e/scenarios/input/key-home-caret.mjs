// input/key-home-caret: Home moves the caret to the start of the box: a letter typed there goes first.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-home-caret',
  covers: ['input/key/home'],
  run: (ctx) => I.caretKeys(ctx, 'home'),
});
