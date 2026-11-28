// input/key-f9-mixer-once: F9 changes the desk's picture with one press.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f9-mixer-once',
  covers: ['input/key/f9'],
  run: (ctx) => I.fkeyToggle(ctx, 9, 'once'),
});
