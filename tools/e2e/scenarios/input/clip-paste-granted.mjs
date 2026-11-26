// input/clip-paste-granted: Ctrl+V under the 'granted' permission: the clipboard text goes into the box (in the guest's memory).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/clip-paste-granted',
  covers: ['input/clipboard/paste-granted'],
  run: (ctx) => I.clipPaste(ctx, 'granted'),
});
