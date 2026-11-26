// input/hint-bar-hides: when the pointer leaves the top bar, its picture is as it was before the hint (the hint is gone).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/hint-bar-hides',
  covers: ['input/tooltip/hint-bar-hides'],
  run: (ctx) => I.hintCase(ctx, 'hides'),
});
