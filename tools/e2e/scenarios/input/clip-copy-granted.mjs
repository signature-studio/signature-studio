// input/clip-copy-granted: Ctrl+C copies the selected box text under the 'granted' permission: the text is on the clipboard (read back by the page).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-copy-granted',
  covers: ['input/clipboard/copy-granted'],
  run: (ctx) => I.clipCopy(ctx, 'granted'),
});
