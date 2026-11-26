// input/cursor-resize-ns: over the top and bottom edges of a sizable window the cursor is the vertical resize (ns-resize).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-resize-ns',
  covers: ['input/cursor/resize-ns'],
  run: (ctx) => I.cursorEdges(ctx, ['N', 'S']),
});
