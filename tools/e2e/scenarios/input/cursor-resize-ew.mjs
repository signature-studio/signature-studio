// input/cursor-resize-ew: over the left and right edges of a sizable window the cursor is the horizontal resize (ew-resize).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-resize-ew',
  covers: ['input/cursor/resize-ew'],
  run: (ctx) => I.cursorEdges(ctx, ['E', 'W']),
});
