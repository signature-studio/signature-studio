// input/cursor-resize-diag-nw-se: over the north-west and south-east corners of a sizable window the cursor is nwse-resize. Not met: in this build the north-west and south-east corners show the picture of the north edge (see the reference N and E in the notes).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-resize-diag-nw-se',
  covers: ['input/cursor/resize-diag-nw-se'],
  expectFail: 'new',
  run: (ctx) => I.cursorEdges(ctx, ['NW', 'SE']),
});
