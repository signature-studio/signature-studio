// input/key-escape-cancels-rename: Escape cancels the channel name box: the box is shown before and gone after.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-escape-cancels-rename',
  covers: ['input/key/escape'],
  run: (ctx) => I.escapeRename(ctx),
});
