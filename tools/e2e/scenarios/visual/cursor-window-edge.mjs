// visual/cursor-window-edge: the cursor over a window edge is the picture cursor of the ratio (its declared resolution is the ratio).
import { defineScenario } from '../../lib/scenario.mjs';
import { cursorWindowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/cursor-window-edge',
  covers: ['visual/cursor/window-edge'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => cursorWindowEdge(ctx),
});
