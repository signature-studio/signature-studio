// input/key-repeat-held-arrow: A key held down (three key downs) moves the caret three letters, as the auto-repeat does.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-repeat-held-arrow',
  covers: ['input/key/repeat'],
  run: (ctx) => I.repeatArrow(ctx),
});
