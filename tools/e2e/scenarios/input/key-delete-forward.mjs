// input/key-delete-forward: Delete removes the letter after the caret (Home, then Delete).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-delete-forward',
  covers: ['input/key/delete'],
  run: (ctx) => I.deleteCase(ctx),
});
