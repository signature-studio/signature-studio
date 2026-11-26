// input/touch-tap-file-label: a tap on the File label (touch down and up) reaches the engine as a left button and opens the drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-tap-file-label',
  covers: ['input/touch/tap'],
  run: (ctx) => I.touchTap(ctx, 'File'),
});
