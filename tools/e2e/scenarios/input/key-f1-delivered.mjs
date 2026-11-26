// input/key-f1-delivered: F1 reaches the engine as a key down and up, and the engine stays idle (what the key shows is not read).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f1-delivered',
  covers: ['input/key/f1'],
  run: (ctx) => I.fkeyDelivered(ctx, 1),
});
