// input/touch-long-press-file-label: a press held for 900 ms on the File label reaches the engine (down and up), and no browser context menu shows.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/touch-long-press-file-label',
  covers: ['input/touch/long-press'],
  run: (ctx) => I.touchLong(ctx, 'File'),
});
