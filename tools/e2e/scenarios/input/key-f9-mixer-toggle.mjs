// input/key-f9-mixer-toggle: F9 changes the desk's picture (the Mixer shows or hides), and a second press brings the picture back.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f9-mixer-toggle',
  covers: ['input/key/f9'],
  run: (ctx) => I.fkeyToggle(ctx, 9, 'toggle'),
});
