// visual/window-right-click-playlist-golden: the drop-down of the Playlist (right-click on the playlist): its picture is the golden of the window (window.tsv).
import { defineScenario } from '../../lib/scenario.mjs';
import { windowGolden } from '../../lib/visual.mjs';

export default defineScenario({
  id: 'visual/window-right-click-playlist-golden',
  covers: ['visual/window/right-click-playlist'],
  displays: ['4k-dpr1'],
  expectFail: null,
  run: (ctx) => windowGolden(ctx, "right-click-playlist"),
});
