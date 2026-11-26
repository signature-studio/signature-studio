// visual/cursor-text-field: the cursor over a text field is the picture cursor of the ratio (its declared resolution is the ratio).
// Known (Issue #1496 run, 2026-11-26): the text field's cursor is the CSS keyword text, not a picture, so its declared size cannot be read.
import { defineScenario } from '../../lib/scenario.mjs';
import { cursorTextField } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/cursor-text-field',
  covers: ['visual/cursor/text-field'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => cursorTextField(ctx),
});
