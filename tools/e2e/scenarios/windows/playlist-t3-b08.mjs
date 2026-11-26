// windows/playlist-t3-b08: a click on the Playlist grid (track 3, bar 8) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t3-b08',
  covers: ['windows/playlist/clip-place'],
  async run({ guest }) {
    return playlistCell(guest, 2, 7);
  },
});
