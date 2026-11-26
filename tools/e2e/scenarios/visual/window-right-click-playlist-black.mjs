// visual/window-right-click-playlist-black: the drop-down of the Playlist (right-click on the playlist): no all-black 64 x 64 tile is drawn with it open.
import { defineScenario } from '../../lib/scenario.mjs';
import { windowBlack } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-playlist-black',
  covers: ['visual/black/after-window'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowBlack(ctx, "right-click-playlist"),
});
