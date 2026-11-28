// input/mouse-right-click-playlist: a right press on the Playlist opens its drop-down.
import { defineScenario } from '../../lib/scenario.mjs';
import * as I from '../../lib/input.mjs';

export default defineScenario({
  id: 'input/mouse-right-click-playlist',
  covers: ['input/mouse/right-click'],
  async run(ctx) {
    await I.ensurePlaylist(ctx.guest);
    return I.rightClickWindow(ctx, (w) => w.cls === 'TEventEditForm' && w.vis !== false && !/Piano roll/.test(w.label ?? ''), 'Playlist');
  },
});
