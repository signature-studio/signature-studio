// input/hint-bar-shows: the pointer resting on the top bar shows the hint: the bar's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/hint-bar-shows',
  covers: ['input/tooltip/hint-bar-shows'],
  run: (ctx) => I.hintCase(ctx, 'shows'),
});
