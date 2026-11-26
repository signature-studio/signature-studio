// input/clip-paste-prompt: Ctrl+V under the 'prompt' permission: the page asks to read (readText is called) and the record keeps what the browser said.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-paste-prompt',
  covers: ['input/clipboard/paste-prompt'],
  run: (ctx) => I.clipPaste(ctx, 'prompt'),
});
