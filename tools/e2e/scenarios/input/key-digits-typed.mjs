// input/key-digits-typed: the digit keys (top row) type digits in the channel name box, and the digits are in the guest's memory.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-digits-typed',
  covers: ['input/key/alnum-digit'],
  run: (ctx) => I.typedRename(ctx, String(Math.floor(Math.random() * 1e8)).padStart(8, '0'), 'enter'),
});
