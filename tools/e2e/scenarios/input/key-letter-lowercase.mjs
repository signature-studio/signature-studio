// input/key-letter-lowercase: typed lower case letters in the channel name box (F2) reach the guest's memory and Enter commits them.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-letter-lowercase',
  covers: ['input/key/alnum-letter'],
  run: (ctx) => I.typedRename(ctx, I.word(6), 'enter'),
});
