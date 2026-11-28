// input/key-f3-delivered: F3 reaches the engine as a key down and up, and the engine stays idle.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f3-delivered',
  covers: ['input/key/f3'],
  run: (ctx) => I.fkeyDelivered(ctx, 3),
});
