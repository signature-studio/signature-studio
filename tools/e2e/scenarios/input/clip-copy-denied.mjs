// input/clip-copy-denied: Ctrl+C under the 'denied' permission: the page is not asked to write (writeText is not called), and the record says why.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-copy-denied',
  covers: ['input/clipboard/copy-denied'],
  run: (ctx) => I.clipCopy(ctx, 'denied'),
});
