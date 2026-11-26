// input/key-punctuation-minus: a hyphen types a '-' in the channel name box (the US layout's Minus key).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-punctuation-minus',
  covers: ['input/key/punctuation'],
  run: (ctx) => I.typedRename(ctx, `${I.word(3)}-${I.word(3)}`, 'enter'),
});
