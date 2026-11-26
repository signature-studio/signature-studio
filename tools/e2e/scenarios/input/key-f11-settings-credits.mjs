// input/key-f11-settings-credits: F11 shows the settings window (a TMIDIForm whose caption is Settings).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-f11-settings-credits',
  covers: ['input/key/f11'],
  run: (ctx) => I.fkeyWindow(ctx, 11, 'TMIDIForm', /Settings/),
});
