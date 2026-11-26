// input/clip-permission-change: the permission changes from 'prompt' to 'granted' between two pastes: the first asks to read, the second (granted) gets the text.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-permission-change',
  covers: ['input/clipboard/permission-change'],
  run: (ctx) => I.clipPermChange(ctx),
});
