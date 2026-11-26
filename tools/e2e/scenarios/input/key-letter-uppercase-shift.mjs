// input/key-letter-uppercase-shift: Shift with a letter types the capital letter in the channel name box (the engine reads the Shift state from its own key messages).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-letter-uppercase-shift',
  covers: ['input/key/shift'],
  run: (ctx) => I.typedRename(ctx, I.word(4).toUpperCase()),
});
