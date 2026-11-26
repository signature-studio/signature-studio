// input/hint-bar-shows-right: the pointer resting on the right part of the top bar changes its picture (the hint).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/hint-bar-shows-right',
  covers: ['input/tooltip/hint-bar-shows'],
  run: (ctx) => I.hintCase(ctx, 'shows', 'right'),
});
