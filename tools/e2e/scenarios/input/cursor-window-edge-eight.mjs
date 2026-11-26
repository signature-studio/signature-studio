// input/cursor-window-edge-eight: the cursor over all eight edges and corners of a sizable window is its resize cursor. Not met: the four diagonal edges show the picture of the north edge, and the two horizontal edges another picture: there are two pictures for the eight edges.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-window-edge-eight',
  covers: ['input/cursor/window-edge'],
  expectFail: 'new',
  run: (ctx) => I.cursorEdges(ctx, ['N', 'S', 'E', 'W', 'NE', 'SW', 'NW', 'SE']),
});
