// input/key-f10-settings: F10 shows the settings window (a top-level TMIDIForm).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f10-settings',
  covers: ['input/key/f10'],
  run: (ctx) => I.fkeyWindow(ctx, 10, 'TMIDIForm'),
});
