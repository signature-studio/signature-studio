// input/clip-copy-prompt: Ctrl+C copies the selected box text under the 'prompt' permission: the page gets the text; the write is asked or refused and recorded.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-copy-prompt',
  covers: ['input/clipboard/copy-prompt'],
  run: (ctx) => I.clipCopy(ctx, 'prompt'),
});
