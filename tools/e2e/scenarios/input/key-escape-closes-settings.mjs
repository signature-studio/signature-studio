// input/key-escape-closes-settings: Escape closes the settings window that F10 shows.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/key-escape-closes-settings',
  covers: ['input/key/escape'],
  run: (ctx) => I.escapeWindow(ctx, 10, 'TMIDIForm'),
});
