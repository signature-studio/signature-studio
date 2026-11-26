// input/key-shift-tab-leaves-desk: Shift+Tab moves the focus off the desk, and the key does not reach the engine.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-shift-tab-leaves-desk',
  covers: ['input/key/tab'],
  run: (ctx) => I.tabCase(ctx, true),
});
