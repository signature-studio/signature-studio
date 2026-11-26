// input/mouse-focus-desk: a left click on the desk gives the desk (#desktop) the keyboard focus.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-focus-desk',
  covers: ['input/mouse/focus-desktop'],
  run: (ctx) => I.focusClick(ctx),
});
