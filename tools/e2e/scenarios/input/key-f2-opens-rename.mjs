// input/key-f2-opens-rename: F2 opens the channel name box (a new window).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f2-opens-rename',
  covers: ['input/key/f2'],
  run: (ctx) => I.fkeyF2(ctx),
});
