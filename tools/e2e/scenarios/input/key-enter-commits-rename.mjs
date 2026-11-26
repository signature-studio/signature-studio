// input/key-enter-commits-rename: Enter commits the typed name (it is in the guest's memory) and closes the box.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-enter-commits-rename',
  covers: ['input/key/enter'],
  run: (ctx) => I.enterRename(ctx, false),
});
