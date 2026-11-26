// input/key-enter-empty-rename: Enter closes the channel name box with no text typed.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-enter-empty-rename',
  covers: ['input/key/enter'],
  run: (ctx) => I.enterRename(ctx, true),
});
