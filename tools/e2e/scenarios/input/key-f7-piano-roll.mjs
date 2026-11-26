// input/key-f7-piano-roll: F7 shows the Piano roll (a window whose caption starts with Piano roll).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f7-piano-roll',
  covers: ['input/key/f7'],
  run: (ctx) => I.fkeyPianoRoll(ctx),
});
