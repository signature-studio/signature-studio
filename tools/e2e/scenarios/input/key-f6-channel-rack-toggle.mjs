// input/key-f6-channel-rack-toggle: F6 changes the desk's picture (the Channel rack hides or shows), and a second press brings the picture back.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f6-channel-rack-toggle',
  covers: ['input/key/f6'],
  run: (ctx) => I.fkeyToggle(ctx, 6, 'toggle'),
});
