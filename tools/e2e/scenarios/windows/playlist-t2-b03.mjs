// windows/playlist-t2-b03: a click on the Playlist grid (track 2, bar 3) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t2-b03',
  covers: ['windows/playlist/clip-place'],
  async run({ guest }) {
    return playlistCell(guest, 1, 2);
  },
});
