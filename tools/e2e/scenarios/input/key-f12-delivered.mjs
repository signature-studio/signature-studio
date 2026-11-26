// input/key-f12-delivered: F12 reaches the engine as a key down and up, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f12-delivered',
  covers: ['input/key/f12'],
  run: (ctx) => I.fkeyDelivered(ctx, 12),
});
