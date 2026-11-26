// input/key-space-inserts: Space types a space in the box: the two words with a space between are in the guest's memory.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-space-inserts',
  covers: ['input/key/space'],
  run: (ctx) => I.typedRename(ctx, `${I.word(3)} ${I.word(3)}`, 'enter'),
});
