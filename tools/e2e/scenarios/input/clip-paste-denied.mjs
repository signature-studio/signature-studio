// input/clip-paste-denied: Ctrl+V under the 'denied' permission: the page does not read (readText is not called) and the box gets no text.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-paste-denied',
  covers: ['input/clipboard/paste-denied'],
  run: (ctx) => I.clipPaste(ctx, 'denied'),
});
