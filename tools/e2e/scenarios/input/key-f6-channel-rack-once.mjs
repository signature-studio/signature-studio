// input/key-f6-channel-rack-once: F6 changes the desk's picture (the Channel rack hides; the window is TStepSeqForm); the press is undone after the check.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f6-channel-rack-once',
  covers: ['input/key/f6'],
  run: (ctx) => I.fkeyToggle(ctx, 6, 'once'),
});
