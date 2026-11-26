// input/key-numpad-delivered: the numeric keypad keys reach the engine as key downs and ups (the digits depend on NumLock and are not read).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-numpad-delivered',
  covers: ['input/key/numpad'],
  run: (ctx) => I.numpadCase(ctx),
});
