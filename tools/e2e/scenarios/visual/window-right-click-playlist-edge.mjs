// visual/window-right-click-playlist-edge: the drop-down of the Playlist (right-click on the playlist): its four edges are sharp and 2 device px wide at most.
// Known (Issue #1496 run, 2026-11-26): the window's top side reads a 3 device px frame (the criterion is 1 or 2); whether the product draws it or the reading counts a gradient is not checked yet.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowEdge } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-playlist-edge',
  covers: ['visual/edge/right-click-playlist'],
  displays: ['4k-dpr1'],
  expectFail: 'new',
  run: (ctx) => windowEdge(ctx, "right-click-playlist"),
});
