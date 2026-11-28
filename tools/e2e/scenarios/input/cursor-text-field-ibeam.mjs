// input/cursor-text-field-ibeam: the pointer over the channel name box (a text field) shows the I-beam (cursor text).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

// Issue #1532: a box the window table lists on top, over the Playlist, gets the Playlist's picture cursor (not the I-beam); over the top bar it is the I-beam. The cause is not confirmed (the cursor choice or the window order).
export default defineScenario({
  id: 'input/cursor-text-field-ibeam',
  expectFail: 'new',
  covers: ['input/cursor/text-field'],
  run: (ctx) => I.cursorText(ctx),
});
