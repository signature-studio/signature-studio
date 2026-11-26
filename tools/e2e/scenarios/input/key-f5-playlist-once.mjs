// input/key-f5-playlist-once: F5 changes the desk's picture with one press.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f5-playlist-once',
  covers: ['input/key/f5'],
  run: (ctx) => I.fkeyToggle(ctx, 5, 'once'),
});
