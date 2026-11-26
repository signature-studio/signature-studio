// windows/playlist-t1-b10: a click on the Playlist grid (track 1, bar 10) with the default tool: the grid's picture changes.
import { defineScenario } from '../../lib/scenario.mjs';
import { playlistCell } from '../../lib/windows.mjs';

export default defineScenario({
  id: 'windows/playlist-t1-b10',
  covers: ['windows/playlist/clip-place'],
  async run({ guest }) {
    return playlistCell(guest, 0, 9);
  },
});
