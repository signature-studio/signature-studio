// input/clip-cut-granted: Ctrl+X cuts the selected box text under 'granted': the page gets the text.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-cut-granted',
  covers: ['input/clipboard/cut-granted'],
  run: (ctx) => I.clipCut(ctx),
});
