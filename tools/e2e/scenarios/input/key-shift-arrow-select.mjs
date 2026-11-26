// input/key-shift-arrow-select: Shift+Left selects the last character of the box, and a typed letter replaces it.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-shift-arrow-select',
  covers: ['input/key/shift-arrow'],
  run: (ctx) => I.shiftArrow(ctx),
});
