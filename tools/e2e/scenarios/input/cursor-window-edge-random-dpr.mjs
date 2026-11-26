// input/cursor-window-edge-random-dpr: the eight edges and corners at a random device pixel ratio (1.01 to 3.00): the same resize cursors. Not met: the four diagonal edges show the picture of the north edge (the same as cursor-window-edge-eight).
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/cursor-window-edge-random-dpr',
  covers: ['input/cursor/window-edge'],
  expectFail: 'new',
  displays: ['4k-random'],
  run: (ctx) => I.cursorEdges(ctx, ['N', 'S', 'E', 'W', 'NE', 'SW', 'NW', 'SE']),
});
