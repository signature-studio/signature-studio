// input/cursor-text-field-ibeam: the pointer over the channel name box (a text field) shows the I-beam (cursor text).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-text-field-ibeam',
  covers: ['input/cursor/text-field'],
  run: (ctx) => I.cursorText(ctx),
});
