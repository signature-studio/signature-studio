// input/cursor-resize-diag-ne-sw: over the north-east and south-west corners of a sizable window the cursor is nesw-resize. Not met: in this build the north-east and south-west corners show the picture of the north edge (not a nesw-resize picture of their own; see the reference N and E in the notes).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-resize-diag-ne-sw',
  covers: ['input/cursor/resize-diag-ne-sw'],
  expectFail: 'new',
  run: (ctx) => I.cursorEdges(ctx, ['NE', 'SW']),
});
